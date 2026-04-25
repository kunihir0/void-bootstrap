use anyhow::{Context, Result};
use inquire::{Confirm, Select, Text};
use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::process::{Command, Stdio};

const XBPS_REPO: &str = "https://repo-default.voidlinux.org/current";
// Common encodings only — extend as needed for more esoteric locales
const VALID_ENCODINGS: &[&str] = &["UTF-8", "ISO8859-1", "ISO8859-15", "EUC-JP", "EUC-KR"];

#[derive(Debug, Clone, Copy, PartialEq)]
enum FsType {
    Ext4,
    Btrfs,
    Xfs,
}

impl FsType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ext4 => "ext4",
            Self::Btrfs => "btrfs",
            Self::Xfs => "xfs",
        }
    }

    fn mount_opts(&self) -> &'static str {
        match self {
            Self::Btrfs => "defaults,compress=zstd,space_cache=v2,subvol=@",
            _ => "defaults",
        }
    }

    fn fstab_dump_pass(&self) -> &'static str {
        match self {
            Self::Btrfs => "0 0",
            _ => "0 1",
        }
    }
}

#[derive(Debug)]
struct InstallState {
    root_part: String,
    efi_part: String,
    fs_type: FsType,
}

// --- UTILITIES ---

fn run_cmd(command: &str, args: &[&str]) -> Result<()> {
    println!(">> Running: {command} {}", args.join(" "));
    let status = Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute {command}"))?;

    if !status.success() {
        anyhow::bail!("Command failed with status: {status}");
    }
    Ok(())
}

fn run_cmd_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute {command}"))?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_chroot(args: &[&str]) -> Result<()> {
    run_cmd("chroot", &[&["/mnt"], args].concat())
}

fn validate_block_device(path: &str) -> Result<()> {
    let meta = fs::metadata(path).with_context(|| format!("Path does not exist: {path}"))?;

    if !meta.file_type().is_block_device() {
        anyhow::bail!("{path} is not a valid block device");
    }
    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    fs::create_dir_all(dst)?;
    fs::set_permissions(dst, fs::metadata(src)?.permissions())?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(entry.path(), dest_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(entry.path())?;
            fs::remove_file(&dest_path).ok();
            std::os::unix::fs::symlink(target, dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

fn is_valid_locale(s: &str) -> bool {
    matches!(s, "C" | "POSIX")
        || s.split_once('_')
            .and_then(|(lang, rest)| {
                rest.split_once('.').map(|(terr, enc)| {
                    lang.len() == 2
                        && lang.chars().all(|c| c.is_ascii_lowercase())
                        && terr.len() == 2
                        && terr.chars().all(|c| c.is_ascii_uppercase())
                        && VALID_ENCODINGS.contains(&enc)
                })
            })
            .unwrap_or(false)
}

// --- INSTALLATION STAGES ---

fn stage_disk() -> Result<InstallState> {
    println!("\n[1] Disk Setup");
    run_cmd("lsblk", &[])?;

    let run_partitioner =
        Confirm::new("Do you need to partition a disk first? (e.g., for a new PC)")
            .with_default(false)
            .prompt()?;

    if run_partitioner {
        let disk = loop {
            let p = Text::new("Enter the disk to partition (e.g., /dev/nvme0n1 or /dev/sda):")
                .prompt()?;
            match validate_block_device(&p) {
                Ok(()) => break p,
                Err(e) => eprintln!("Invalid: {e}. Please try again."),
            }
        };
        println!(">> Launching cfdisk for {disk}...");
        run_cmd("cfdisk", &[&disk])?;
        println!("\n>> Updated partition layout:");
        run_cmd("lsblk", &[])?;
    }

    let root_part = loop {
        let p = Text::new("Enter the ROOT partition (e.g., /dev/nvme0n1p2):").prompt()?;
        match validate_block_device(&p) {
            Ok(()) => break p,
            Err(e) => eprintln!("Invalid: {e}. Please try again."),
        }
    };

    let efi_part = loop {
        let p = Text::new("Enter the EFI partition (e.g., /dev/nvme0n1p1):").prompt()?;
        match validate_block_device(&p) {
            Ok(()) => break p,
            Err(e) => eprintln!("Invalid: {e}. Please try again."),
        }
    };

    let fs_type_str =
        Select::new("Root filesystem type:", vec!["ext4", "btrfs", "xfs"]).prompt()?;
    let fs_type = match fs_type_str {
        "ext4" => FsType::Ext4,
        "btrfs" => FsType::Btrfs,
        "xfs" => FsType::Xfs,
        _ => unreachable!("Select widget handles exhaustiveness"),
    };

    let format_root = Confirm::new(
        "DANGER: Format the ROOT partition now? All data on this partition will be lost.",
    )
    .with_default(false)
    .prompt()?;

    if format_root {
        match fs_type {
            FsType::Ext4 => run_cmd("mkfs.ext4", &["-F", &root_part])?,
            FsType::Xfs => run_cmd("mkfs.xfs", &["-f", &root_part])?,
            FsType::Btrfs => {
                run_cmd("mkfs.btrfs", &["-f", &root_part])?;
                fs::create_dir_all("/tmp/btrfs-setup")?;
                run_cmd("mount", &[&root_part, "/tmp/btrfs-setup"])?;

                scopeguard::defer! {
                    let _ = Command::new("umount").arg("/tmp/btrfs-setup").status();
                }

                run_cmd("btrfs", &["subvolume", "create", "/tmp/btrfs-setup/@"])?;
            }
        }
    }

    let format_efi = Confirm::new(
        "DANGER: Format the EFI partition? (Choose 'No' if sharing with Windows/OpenCore!)",
    )
    .with_default(false)
    .prompt()?;

    if format_efi {
        run_cmd("mkfs.fat", &["-F32", &efi_part])?;
    } else {
        println!(">> Skipping EFI format. Existing bootloaders will be preserved.");
    }

    Ok(InstallState {
        root_part,
        efi_part,
        fs_type,
    })
}

fn stage_mount(state: &InstallState) -> Result<()> {
    println!("\n[2] Mounting Partitions");

    if run_cmd_output("findmnt", &["-M", "/mnt"]).is_ok() {
        anyhow::bail!("/mnt is already mounted. Unmount it before running the installer.");
    }

    if state.fs_type == FsType::Btrfs {
        run_cmd("mount", &["-o", "subvol=@", &state.root_part, "/mnt"])?;
    } else {
        run_cmd("mount", &[&state.root_part, "/mnt"])?;
    }

    fs::create_dir_all("/mnt/boot/efi").context("Failed to create EFI directory")?;
    run_cmd("mount", &[&state.efi_part, "/mnt/boot/efi"])?;

    Ok(())
}

fn stage_base_install(state: &InstallState) -> Result<()> {
    println!("\n[3] Installing Base System via XBPS");

    let gpu = Select::new(
        "Select GPU vendor for drivers:",
        vec!["AMD", "Intel", "NVIDIA", "None"],
    )
    .prompt()?;

    let mut base_packages = vec![
        "base-system",
        "grub-x86_64-efi",
        "linux-mainline",
        "NetworkManager",
        "glibc-locales",
        "efibootmgr",
    ];

    if state.fs_type == FsType::Btrfs {
        base_packages.push("btrfs-progs");
    }

    fs::create_dir_all("/mnt/var/db/xbps/keys")?;
    copy_dir_all("/var/db/xbps/keys", "/mnt/var/db/xbps/keys")?;

    let gpu_packages: &[&str] = match gpu {
        "AMD" => &[
            "linux-firmware-amd",
            "mesa-dri",
            "mesa-vaapi",
            "mesa-vulkan-radeon",
        ],
        "Intel" => &[
            "linux-firmware-intel",
            "mesa-dri",
            "mesa-vaapi",
            "intel-video-accel",
        ],
        "NVIDIA" => {
            println!(">> Setting up Void non-free repository for NVIDIA...");
            println!("NOTE: For Wayland support, you may also need 'nvidia-dkms'.");
            println!("      Add 'nvidia-drm.modeset=1' to your GRUB_CMDLINE_LINUX.");
            run_cmd(
                "xbps-install",
                &[
                    "-y",
                    "-S",
                    "-R",
                    XBPS_REPO,
                    "-r",
                    "/mnt",
                    "--",
                    "void-repo-nonfree",
                ],
            )?;
            &["nvidia", "nvidia-libs"]
        }
        "None" => &[],
        _ => unreachable!("Select widget handles exhaustiveness"),
    };

    base_packages.extend(gpu_packages.iter().copied());

    let mut xbps_args = vec!["-y", "-S", "-R", XBPS_REPO, "-r", "/mnt", "--"];
    xbps_args.extend(base_packages.iter().copied());
    run_cmd("xbps-install", &xbps_args)?;

    Ok(())
}

fn stage_chroot_setup() -> Result<()> {
    println!("\n[4] Configuring the Chroot Environment");
    for dir in &["dev", "proc", "sys", "run"] {
        run_cmd(
            "mount",
            &["--rbind", &format!("/{dir}"), &format!("/mnt/{dir}")],
        )?;
        run_cmd("mount", &["--make-rslave", &format!("/mnt/{dir}")])?;
    }
    Ok(())
}

fn stage_configure(state: &InstallState) -> Result<()> {
    println!("\n[5] Native System Configuration");

    let hostname = loop {
        let h = Text::new("Enter system hostname:")
            .with_default("voidlinux")
            .prompt()?;
        if !h.is_empty()
            && h.len() <= 253
            && !h.starts_with('-')
            && !h.ends_with('-')
            && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            break h;
        }
        eprintln!("Invalid hostname format. Please try again.");
    };
    fs::write("/mnt/etc/hostname", format!("{hostname}\n"))?;

    let timezone = loop {
        let tz = Text::new("Enter timezone:")
            .with_default("America/Phoenix")
            .prompt()?;
        let tz_path = format!("/mnt/usr/share/zoneinfo/{tz}");
        if Path::new(&tz_path).exists()
            && tz
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "/_-+".contains(c))
            && !tz.contains("..")
        {
            break tz;
        }
        eprintln!("Timezone not found or invalid format. Please try again.");
    };
    run_chroot(&[
        "ln",
        "-sf",
        &format!("/usr/share/zoneinfo/{timezone}"),
        "/etc/localtime",
    ])?;

    let locale = loop {
        let loc = Text::new("Enter system locale:")
            .with_default("en_US.UTF-8")
            .prompt()?;
        if is_valid_locale(&loc) {
            break loc;
        }
        eprintln!("Invalid locale format (e.g., en_US.UTF-8). Please try again.");
    };

    fs::write("/mnt/etc/locale.conf", format!("LANG={locale}\n"))?;
    let libc_locales_path = "/mnt/etc/default/libc-locales";
    if Path::new(libc_locales_path).exists() {
        let contents = fs::read_to_string(libc_locales_path)?;
        let locale_prefix = format!("{locale} ");
        let uncommented = contents
            .lines()
            .map(|line| {
                let unhashed = line.trim_start_matches('#').trim_start();
                if unhashed == locale.as_str() || unhashed.starts_with(&locale_prefix) {
                    unhashed.to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(libc_locales_path, uncommented)?;
    }
    run_chroot(&["xbps-reconfigure", "-f", "glibc-locales"])?;

    let root_uuid = run_cmd_output("blkid", &["-s", "UUID", "-o", "value", &state.root_part])?;
    let root_uuid = root_uuid.trim();
    if root_uuid.is_empty() {
        anyhow::bail!(
            "Could not read UUID for {}. Formatting may have failed.",
            state.root_part
        );
    }

    let efi_uuid = run_cmd_output("blkid", &["-s", "UUID", "-o", "value", &state.efi_part])?;
    let efi_uuid = efi_uuid.trim();
    if efi_uuid.is_empty() {
        anyhow::bail!(
            "Could not read UUID for {}. Formatting may have failed.",
            state.efi_part
        );
    }

    let fs_str = state.fs_type.as_str();
    let root_opts = state.fs_type.mount_opts();
    let root_dump_pass = state.fs_type.fstab_dump_pass();

    let fstab = format!(
        "UUID={root_uuid} / {fs_str} {root_opts} {root_dump_pass}\nUUID={efi_uuid} /boot/efi vfat defaults 0 0\n"
    );
    fs::write("/mnt/etc/fstab", fstab)?;

    Ok(())
}

fn stage_bootloader() -> Result<bool> {
    println!("\n[6] Installing GRUB Bootloader");

    let efi_vars_exist = Path::new("/sys/firmware/efi/efivars").exists();
    if !efi_vars_exist {
        println!(">> WARNING: /sys/firmware/efi/efivars not found.");
        println!(">> It appears you booted the installer in Legacy (BIOS) mode instead of UEFI.");
        println!(">> NVRAM registration will be disabled because EFI variables are inaccessible.");
    }

    let update_nvram = if efi_vars_exist {
        Confirm::new(
            "Register Void in motherboard UEFI Boot Menu? (Choose 'Yes' if using F12 to select OS)",
        )
        .with_default(true)
        .prompt()?
    } else {
        false
    };

    let mut grub_args = vec![
        "grub-install",
        "--target=x86_64-efi",
        "--efi-directory=/boot/efi",
        "--bootloader-id=Void",
    ];

    if !update_nvram {
        grub_args.push("--no-nvram");
    }

    run_chroot(&grub_args)?;
    run_chroot(&["xbps-reconfigure", "-fa"])?;
    run_chroot(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"])?;

    Ok(update_nvram)
}

fn stage_users() -> Result<()> {
    println!("\n[7] Finalizing Users and Services");
    println!("Set ROOT password:");
    run_chroot(&["passwd"])?;

    let username = loop {
        let u = Text::new("Enter primary username:")
            .with_default("baobao")
            .prompt()?;
        if u.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
            && u.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            && u.len() <= 32
        {
            break u;
        }
        eprintln!("Invalid username format. Try again.");
    };

    run_chroot(&[
        "useradd",
        "-m",
        "-s",
        "/bin/bash",
        "-G",
        "wheel,audio,video,cdrom,input",
        &username,
    ])?;
    println!("Set password for {username}:");
    run_chroot(&["passwd", &username])?;

    fs::write("/mnt/etc/sudoers.d/wheel", "%wheel ALL=(ALL:ALL) ALL\n")?;
    fs::set_permissions(
        "/mnt/etc/sudoers.d/wheel",
        fs::Permissions::from_mode(0o440),
    )?;

    run_chroot(&["ln", "-s", "/etc/sv/NetworkManager", "/var/service/"])?;
    Ok(())
}

// --- MAIN ENTRY POINT ---

fn main() -> Result<()> {
    println!("========================================");
    println!("  Void Linux Automated Bootstrap (TTY)");
    println!("========================================");

    let state = stage_disk()?;
    stage_mount(&state)?;
    stage_base_install(&state)?;

    scopeguard::defer! {
        println!(">> Cleaning up bind mounts...");
        for dir in &["run", "sys", "proc", "dev"] {
            let _ = Command::new("umount").args(["-R", &format!("/mnt/{dir}")]).status();
        }
    }

    stage_chroot_setup()?;
    stage_configure(&state)?;

    let nvram_updated = stage_bootloader()?;

    stage_users()?;

    println!("\n========================================");
    println!("Installation Complete!");

    if !nvram_updated {
        println!("\n[!] CRITICAL NEXT STEPS FOR DUAL/MULTI-BOOT");
        println!("Because '--no-nvram' was used, Void is NOT yet in your UEFI boot list.");
        println!(
            "Before rebooting, you must do ONE of the following to ensure you can boot Void:\n"
        );
        println!("  A) Add to OpenCore Config:");
        println!("     Point a Misc > Entries item at \\EFI\\Void\\grubx64.efi");
        println!("     (Ensure Misc > Boot > LauncherOption is set properly if needed).\n");
        println!("  B) Register via efibootmgr (run before unmounting):");
        println!("     efibootmgr --create --disk /dev/sdX --part N \\");
        println!("       --label 'Void Linux' --loader '\\EFI\\Void\\grubx64.efi'\n");
        println!("     (Verify exact path with: ls /mnt/boot/efi/EFI/Void/ before registering)\n");
    } else {
        println!("\n>> Void Linux has been registered in your UEFI boot menu.");
        println!(">> You can use your motherboard's boot menu (F12/F8) to select your OS.");
    }

    println!("Once done, safely unmount and reboot:");
    println!("  umount -R /mnt && reboot");
    println!("========================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_validation() {
        assert!(is_valid_locale("en_US.UTF-8"));
        assert!(is_valid_locale("C"));
        assert!(is_valid_locale("POSIX"));
        assert!(is_valid_locale("ja_JP.EUC-JP"));

        assert!(!is_valid_locale("en_US.GARBAGE")); // Invalid encoding
        assert!(!is_valid_locale("en_USXX.UTF-8")); // Territory too long
        assert!(!is_valid_locale("en.UTF-8")); // Missing territory
        assert!(!is_valid_locale("")); // Empty
    }

    #[test]
    fn test_fs_type_fstab_logic() {
        assert_eq!(FsType::Btrfs.fstab_dump_pass(), "0 0");
        assert_eq!(FsType::Ext4.fstab_dump_pass(), "0 1");
        assert_eq!(FsType::Xfs.fstab_dump_pass(), "0 1");

        assert!(FsType::Btrfs.mount_opts().contains("subvol=@"));
        assert_eq!(FsType::Ext4.mount_opts(), "defaults");
    }
}
