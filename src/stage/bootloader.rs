use crate::ui::Ui;
use crate::util::command::run_pivoted;
use anyhow::Result;
use std::path::Path;

/// GRUB modules to embed in the EFI binary.
///
/// These cover: partition tables, filesystems the installer supports,
/// UEFI video, kernel loading, config search, and chainloading (Windows).
const GRUB_MODULES: &[&str] = &[
    // Partition tables
    "part_gpt",
    "part_msdos",
    // Filesystems
    "fat",
    "btrfs",
    "ext2",
    "xfs",
    // Boot
    "normal",
    "boot",
    "linux",
    "configfile",
    "chain",
    // Search / probe
    "search",
    "search_fs_uuid",
    "search_label",
    "probe",
    // Video
    "all_video",
    "efi_gop",
    "efi_uga",
    // Misc
    "echo",
    "test",
    "ls",
    "cat",
    "gzio",
    "loopback",
    "lvm",
];

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

    let modules = GRUB_MODULES.join(" ");

    // Build the GRUB EFI binary manually with grub-mkimage instead of
    // grub-install.  grub-install's device-mapping logic fails when
    // running from a live USB because it cannot map the host's block
    // devices to GRUB drive names.  grub-mkimage doesn't need device
    // mapping — it just bundles modules into an EFI PE binary.
    let mut script = format!(
        r#"echo "Building GRUB EFI binary with grub-mkimage..."
mkdir -p /boot/efi/EFI/Void
mkdir -p /boot/efi/EFI/BOOT
grub-mkimage \
  -O x86_64-efi \
  -o /boot/efi/EFI/Void/grubx64.efi \
  -p /boot/grub \
  {modules}
echo "Copying to fallback EFI path..."
cp /boot/efi/EFI/Void/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
echo "Copying GRUB modules to /boot/grub/x86_64-efi/..."
mkdir -p /boot/grub/x86_64-efi
cp /usr/lib/grub/x86_64-efi/*.mod /boot/grub/x86_64-efi/ 2>/dev/null || true
cp /usr/lib/grub/x86_64-efi/*.lst /boot/grub/x86_64-efi/ 2>/dev/null || true"#
    );

    if update_nvram {
        // Parse EFI partition device into disk + partition number for efibootmgr.
        // e.g. /dev/sda1 → disk=/dev/sda part=1
        //      /dev/nvme0n1p1 → disk=/dev/nvme0n1 part=1
        script.push_str(
            r#"
echo "Registering Void in UEFI boot menu..."
EFI_DEV=$(findmnt -n -o SOURCE /boot/efi)
if echo "$EFI_DEV" | grep -q 'nvme\|mmcblk'; then
  EFI_DISK=$(echo "$EFI_DEV" | sed 's/p[0-9]*$//')
  EFI_PART=$(echo "$EFI_DEV" | grep -o '[0-9]*$')
else
  EFI_DISK=$(echo "$EFI_DEV" | sed 's/[0-9]*$//')
  EFI_PART=$(echo "$EFI_DEV" | grep -o '[0-9]*$')
fi
echo "EFI disk=$EFI_DISK partition=$EFI_PART"
efibootmgr -c -d "$EFI_DISK" -p "$EFI_PART" -L "Void" -l '\EFI\Void\grubx64.efi' || echo "Warning: efibootmgr failed (non-fatal)"
"#,
        );
    }

    script.push_str(
        r#"
echo "Reconfiguring installed packages..."
xbps-reconfigure -fa
echo "Generating GRUB configuration..."
grub-mkconfig -o /boot/grub/grub.cfg"#,
    );

    ui.status("Installing GRUB to EFI system partition...");
    run_pivoted(&script)?;

    ui.success("Bootloader installed.");

    Ok(update_nvram)
}
