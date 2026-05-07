use crate::ui::Ui;
use crate::util::command::run_pivoted;
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

    let mut grub_args = vec![
        "grub-install",
        "--target=x86_64-efi",
        "--efi-directory=/boot/efi",
        "--bootloader-id=Void",
    ];

    if !update_nvram {
        grub_args.push("--no-nvram");
    }

    let grub_cmd = grub_args.join(" ");

    // Run all GRUB commands inside an isolated mount namespace via
    // pivot_root.  This makes /proc/self/mountinfo show the target's
    // mounts with correct paths so grub-probe can resolve devices.
    //
    // We also generate a device.map inside the pivoted environment so
    // grub-probe can map device nodes (like /dev/sdb3) to GRUB drive
    // names (like (hd1,gpt3)).
    let script = format!(
        r#"echo "Generating device.map..."
i=0; for d in /sys/block/sd* /sys/block/nvme* /sys/block/vd* /sys/block/mmcblk*; do
  [ -e "$d" ] || continue
  name=$(basename "$d")
  echo "(hd$i) /dev/$name"
  i=$((i+1))
done > /boot/grub/device.map
cat /boot/grub/device.map
{grub_cmd} && \
xbps-reconfigure -fa && \
grub-mkconfig -o /boot/grub/grub.cfg
rm -f /boot/grub/device.map"#
    );

    ui.status("Installing GRUB to EFI system partition...");
    run_pivoted(&script)?;

    ui.success("Bootloader installed.");

    Ok(update_nvram)
}
