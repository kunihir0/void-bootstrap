use crate::ui::Ui;
use crate::util::command::run_chroot;
use anyhow::Result;
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

    let nvram_flag = if update_nvram { "" } else { "--no-nvram" };

    // With `xchroot`, the necessary virtual filesystems (/dev, /sys, /proc) are
    // available inside the chroot, allowing `grub-install` to natively resolve
    // block devices and automatically execute `efibootmgr`.
    let script = format!(
        r#"set -e
echo "Reconfiguring installed packages (dracut)..."
xbps-reconfigure -fa

echo "Installing GRUB to EFI system partition..."
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id="Void" {nvram_flag}

echo "Copying to fallback EFI path..."
mkdir -p /boot/efi/EFI/BOOT
cp /boot/efi/EFI/Void/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI

echo "Generating GRUB configuration..."
grub-mkconfig -o /boot/grub/grub.cfg"#
    );

    ui.status("Installing bootloader...");
    run_chroot(&["sh", "-c", &script])?;

    ui.success("Bootloader installed.");

    Ok(update_nvram)
}
