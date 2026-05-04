use crate::context::InstallContext;
use crate::types::VolumeManager;
use crate::ui::Ui;
use crate::util::command::{block_device_uuid, run_chroot};
use crate::validation::{validate_hostname, validate_locale, validate_timezone};
use anyhow::Result;
use std::fs;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    let hostname = ui.prompt_validated("Enter system hostname:", Some("voidlinux"), |h| {
        validate_hostname(h)
    })?;
    fs::write(ctx.target_path("etc/hostname"), format!("{hostname}\n"))?;

    let tz_zoneinfo = ctx.target_path("usr/share/zoneinfo");
    let timezone = ui.prompt_validated("Enter timezone:", Some("America/Phoenix"), |tz| {
        validate_timezone(tz, &tz_zoneinfo)
    })?;
    run_chroot(&[
        "ln",
        "-sf",
        &format!("/usr/share/zoneinfo/{timezone}"),
        "/etc/localtime",
    ])?;

    let locale = ui.prompt_validated("Enter system locale:", Some("en_US.UTF-8"), |l| {
        validate_locale(l)
    })?;

    fs::write(
        ctx.target_path("etc/locale.conf"),
        format!("LANG={locale}\n"),
    )?;
    let libc_locales_path = ctx.target_path("etc/default/libc-locales");
    if libc_locales_path.exists() {
        let contents = fs::read_to_string(&libc_locales_path)?;
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
        fs::write(&libc_locales_path, uncommented)?;
    }
    ui.status("Reconfiguring glibc locales...");
    run_chroot(&["xbps-reconfigure", "-f", "glibc-locales"])?;

    // ── fstab generation ────────────────────────────────────────
    let fstab = generate_fstab(ctx)?;
    fs::write(ctx.target_path("etc/fstab"), fstab)?;

    // ── LVM boot plumbing (dracut + GRUB) ───────────────────────
    if ctx.volume_mgr == VolumeManager::Lvm {
        configure_lvm_boot(ui, ctx)?;
    }

    ui.success("System configured.");

    Ok(())
}

/// Ensure the initramfs (dracut) and bootloader (GRUB) are configured so
/// that an LVM root volume can be activated and mounted at boot.
///
/// This must run *before* the bootloader stage, which calls
/// `xbps-reconfigure -fa` (regenerating the initramfs) and `grub-mkconfig`.
fn configure_lvm_boot(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    let vg = VolumeManager::VG_NAME;
    let lv = VolumeManager::LV_ROOT;
    let lvm_param = format!("rd.lvm.lv={vg}/{lv}");

    // ── 1. Dracut: force-include the `lvm` module ───────────────
    // Inside a chroot dracut cannot auto-detect that the host root
    // lives on LVM, so we drop an explicit config snippet.
    ui.status("Configuring dracut to include LVM support in initramfs...");
    let dracut_dir = ctx.target_path("etc/dracut.conf.d");
    fs::create_dir_all(&dracut_dir)?;
    fs::write(
        dracut_dir.join("lvm.conf"),
        "# Added by void_bootstrap — LVM root support.\nadd_dracutmodules+=\" lvm \"\n",
    )?;

    // ── 2. GRUB: add rd.lvm.lv kernel parameter ────────────────
    // Tells dracut which LV to activate before mounting root.
    ui.status("Adding LVM kernel parameters to GRUB defaults...");
    let grub_defaults = ctx.target_path("etc/default/grub");

    if grub_defaults.exists() {
        let contents = fs::read_to_string(&grub_defaults)?;
        if !contents.contains(&lvm_param) {
            let updated = contents
                .lines()
                .map(|line| {
                    if line.starts_with("GRUB_CMDLINE_LINUX_DEFAULT=") {
                        // Append inside the existing quoted value.
                        let trimmed = line.trim_end_matches('"');
                        format!("{trimmed} {lvm_param}\"")
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            fs::write(&grub_defaults, updated)?;
        }
    } else {
        // No defaults file yet — write a minimal one.
        fs::write(
            &grub_defaults,
            format!("GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=4 {lvm_param}\"\n"),
        )?;
    }

    ui.success("LVM boot configuration applied (dracut + GRUB).");

    Ok(())
}

/// Build `/etc/fstab` contents from the install context.
fn generate_fstab(ctx: &InstallContext) -> Result<String> {
    let efi_uuid = block_device_uuid(&ctx.efi_device)?;
    let mut lines: Vec<String> = Vec::new();

    // ── Root entry ──────────────────────────────────────────────
    let root_id = root_device_id(ctx)?;
    let root_opts = ctx.fs_type.mount_opts();
    let fs_str = ctx.fs_type.as_str();
    let root_dump_pass = ctx.fs_type.fstab_dump_pass();
    lines.push(format!("{root_id} / {fs_str} {root_opts} {root_dump_pass}"));

    // ── Additional BTRFS subvolume entries ──────────────────────
    if let Some(layout) = ctx.btrfs_layout {
        for sv in layout.subvolumes() {
            if sv.mountpoint.is_empty() {
                continue; // root is already covered above
            }
            let opts = ctx.fs_type.subvol_mount_opts(sv.name);
            let mountpoint = format!("/{}", sv.mountpoint);
            lines.push(format!("{root_id} {mountpoint} {fs_str} {opts} 0 0"));
        }
    }

    // ── EFI entry ───────────────────────────────────────────────
    lines.push(format!("UUID={efi_uuid} /boot/efi vfat defaults 0 0"));

    // Trailing newline for POSIX compliance.
    Ok(lines.join("\n") + "\n")
}

/// Return the device identifier string to use in fstab for the root device.
///
/// LVM devices use their `/dev/vg/lv` path; standard partitions use UUID.
fn root_device_id(ctx: &InstallContext) -> Result<String> {
    if ctx.volume_mgr == VolumeManager::Lvm {
        Ok(ctx.root_device.clone())
    } else {
        let uuid = block_device_uuid(&ctx.root_device)?;
        Ok(format!("UUID={uuid}"))
    }
}
