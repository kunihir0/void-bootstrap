use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::Result;

/// Result of manual partition selection.
pub(crate) struct ManualResult {
    pub root_part: String,
    pub efi_part: String,
}

/// Manual partition flow: optionally launch `cfdisk`, then prompt the user
/// to identify the root and (optionally) EFI partitions.
pub(crate) fn run(ui: &Ui, existing_efi: Option<&str>) -> Result<ManualResult> {
    let run_partitioner = ui.confirm(
        "Do you need to partition a disk first? (e.g., for a new PC)",
        false,
    )?;

    if run_partitioner {
        let disk = ui.prompt_validated(
            "Enter the disk to partition (e.g., /dev/nvme0n1 or /dev/sda):",
            None,
            |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
        )?;
        ui.status(&format!("Launching cfdisk for {disk}..."));
        command::run("cfdisk", &[&disk])?;
        ui.status("Updated partition layout:");
        command::run("lsblk", &[])?;
    }

    let root_part = ui.prompt_validated(
        "Enter the ROOT partition (e.g., /dev/nvme0n1p2):",
        None,
        |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
    )?;

    let efi_part = if let Some(efi) = existing_efi {
        efi.to_string()
    } else {
        ui.prompt_validated(
            "Enter the EFI partition (e.g., /dev/nvme0n1p1):",
            None,
            |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
        )?
    };

    Ok(ManualResult { root_part, efi_part })
}
