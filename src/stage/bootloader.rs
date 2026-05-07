use crate::ui::Ui;
use crate::util::command::run_chroot;
use anyhow::Result;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

pub(crate) fn run(ui: &Ui) -> Result<bool> {
    let efi_vars_exist = Path::new("/sys/firmware/efi/efivars").exists();
    if !efi_vars_exist {
        ui.warning("/sys/firmware/efi/efivars not found.");
        ui.info("It appears you booted the installer in Legacy (BIOS) mode instead of UEFI.");
        ui.info("NVRAM registration will be disabled because EFI variables are inaccessible.");
    }

    let update_nvram = if efi_vars_exist {
        ui.confirm(
            "Register Void in motherboard UEFI Boot Menu? (Choose 'Yes' if using F12 to select OS)",
            true,
        )?
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

    ui.status("Configuring chroot mtab for GRUB...");
    let mtab_path = format!("{}/etc/mtab", crate::context::TARGET);
    let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
    let mut clean_mtab = String::new();
    let target_exact = format!(" {} ", crate::context::TARGET);
    let target_prefix = format!(" {}/", crate::context::TARGET);

    for line in mounts.lines() {
        if line.contains(&target_exact) || line.contains(&target_prefix) {
            let mut new_line = line.replace(&target_exact, " / ");
            new_line = new_line.replace(&target_prefix, " /");
            clean_mtab.push_str(&new_line);
            clean_mtab.push('\n');
        }
    }

    if clean_mtab.is_empty() {
        ui.warning("Could not derive chroot mount entries from /proc/mounts.");
        ui.warning("GRUB may fail if the live environment's mounts confuse grub-probe.");
    }

    let _ = fs::remove_file(&mtab_path);
    fs::write(&mtab_path, &clean_mtab)?;

    ui.status("Installing GRUB to EFI system partition...");

    let res = (|| -> Result<()> {
        run_chroot(&grub_args)?;
        ui.status("Reconfiguring installed packages...");
        run_chroot(&["xbps-reconfigure", "-fa"])?;
        ui.status("Generating GRUB configuration...");
        run_chroot(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"])?;
        Ok(())
    })();

    // Restore the standard symlink so the installed system behaves normally.
    let _ = fs::remove_file(&mtab_path);
    let _ = symlink("/proc/self/mounts", &mtab_path);

    res?;

    ui.success("Bootloader installed.");

    Ok(update_nvram)
}
