use crate::ui::Ui;
use crate::util::command::run_chroot;
use anyhow::Result;
use std::path::Path;

pub(crate) fn run(ui: &Ui) -> Result<bool> {
    let efi_vars_exist = Path::new("/sys/firmware/efi/efivars").exists();
    if !efi_vars_exist {
        ui.warning("/sys/firmware/efi/efivars not found.");
        ui.status("It appears you booted the installer in Legacy (BIOS) mode instead of UEFI.");
        ui.status("NVRAM registration will be disabled because EFI variables are inaccessible.");
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

    run_chroot(&grub_args)?;
    run_chroot(&["xbps-reconfigure", "-fa"])?;
    run_chroot(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"])?;

    Ok(update_nvram)
}
