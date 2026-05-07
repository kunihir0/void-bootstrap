use crate::ui::Ui;
use crate::util::command::run_chroot;
use anyhow::Result;
use std::fs;
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

    ui.status("Generating GRUB device map for chroot environment...");
    let device_map_path = format!("{}/boot/grub/device.map", crate::context::TARGET);
    let device_map = generate_device_map()?;
    fs::create_dir_all(format!("{}/boot/grub", crate::context::TARGET))?;
    fs::write(&device_map_path, &device_map)?;

    ui.status("Installing GRUB to EFI system partition...");
    run_chroot(&grub_args)?;
    ui.status("Reconfiguring installed packages...");
    run_chroot(&["xbps-reconfigure", "-fa"])?;
    ui.status("Generating GRUB configuration...");
    run_chroot(&["grub-mkconfig", "-o", "/boot/grub/grub.cfg"])?;

    // Clean up the synthetic device.map so the installed system
    // auto-detects on future kernel/grub updates.
    let _ = fs::remove_file(&device_map_path);

    ui.success("Bootloader installed.");

    Ok(update_nvram)
}

/// Build a GRUB `device.map` from `/sys/block`, covering every disk the
/// kernel can see (sd*, nvme*, vd*, mmcblk*).
fn generate_device_map() -> Result<String> {
    let mut entries = Vec::new();

    for entry in fs::read_dir("/sys/block")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        // Only include real disk devices, skip loop/ram/dm/sr/etc.
        let dominated = name.starts_with("sd")
            || name.starts_with("nvme")
            || name.starts_with("vd")
            || name.starts_with("mmcblk");

        if dominated {
            entries.push(name.into_owned());
        }
    }

    // Sort for deterministic ordering (hd0, hd1, …).
    entries.sort();

    let mut map = String::new();
    for (i, dev) in entries.iter().enumerate() {
        map.push_str(&format!("(hd{i}) /dev/{dev}\n"));
    }

    Ok(map)
}
