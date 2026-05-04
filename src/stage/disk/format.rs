use crate::types::FsType;
use crate::ui::Ui;
use crate::util::command;
use anyhow::Result;

/// Format the root device with the chosen filesystem.
pub(crate) fn format_root(ui: &Ui, device: &str, fs_type: FsType) -> Result<()> {
    let proceed = ui.confirm_destructive(
        "All data on this partition/volume will be permanently erased.",
        &format!("Format {device} as {fs_type}?"),
    )?;

    if !proceed {
        ui.info("Skipping root format (assuming it is already formatted).");
        return Ok(());
    }

    ui.status(&format!("Formatting {device} as {fs_type}..."));
    match fs_type {
        FsType::Ext4 => command::run("mkfs.ext4", &["-F", device])?,
        FsType::Xfs => command::run("mkfs.xfs", &["-f", device])?,
        FsType::Btrfs => command::run("mkfs.btrfs", &["-f", device])?,
    }

    ui.success("Root device formatted.");
    Ok(())
}

/// Format a partition as FAT32 for the EFI System Partition.
pub(crate) fn format_efi(ui: &Ui, device: &str) -> Result<()> {
    ui.status(&format!("Formatting {device} as FAT32..."));
    command::run("mkfs.fat", &["-F32", device])?;
    ui.success("EFI partition formatted.");
    Ok(())
}
