use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::{Context, Result};

/// Result of automatic disk partitioning.
pub(crate) struct AutoResult {
    /// The EFI partition created (None when the user supplied an existing one).
    pub efi_part: Option<String>,
    /// The Linux partition (or PV partition when using LVM).
    pub linux_part: String,
}

/// Derive partition device paths from a whole-disk path.
///
/// NVMe and mmcblk devices use the `p` separator (e.g. `/dev/nvme0n1p1`),
/// while traditional SCSI/SATA disks append the number directly (`/dev/sda1`).
fn partition_path(disk: &str, num: u32) -> String {
    if disk
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_digit())
    {
        format!("{disk}p{num}")
    } else {
        format!("{disk}{num}")
    }
}

/// Automatically partition a whole disk using `sgdisk`.
///
/// If `existing_efi` is `Some`, the entire disk is allocated to Linux.
/// Otherwise a 512 MiB EFI System Partition is created first.
pub(crate) fn run(ui: &Ui, existing_efi: Option<&str>) -> Result<AutoResult> {
    let disk = ui.prompt_validated(
        "Enter the target disk for automated partitioning (e.g., /dev/sda):",
        None,
        |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
    )?;

    let proceed = ui.confirm_destructive(
        &format!("ALL DATA on {disk} will be permanently destroyed."),
        &format!("Wipe {disk} and create a new partition table?"),
    )?;

    if !proceed {
        anyhow::bail!("Automated partitioning cancelled by user.");
    }

    ui.status(&format!("Wiping partition table on {disk}..."));
    command::run("sgdisk", &["--zap-all", &disk])?;

    let (efi_part, linux_part) = if existing_efi.is_some() {
        // Entire disk → single Linux partition.
        ui.status("Creating Linux partition (entire disk)...");
        command::run(
            "sgdisk",
            &["-n", "1:0:0", "-t", "1:8300", "-c", "1:Linux filesystem", &disk],
        )?;
        (None, partition_path(&disk, 1))
    } else {
        // EFI (512M) + Linux (remainder).
        ui.status("Creating EFI (512 MiB) + Linux partitions...");
        command::run(
            "sgdisk",
            &[
                "-n", "1:0:+512M", "-t", "1:ef00", "-c", "1:EFI System",
                "-n", "2:0:0",     "-t", "2:8300", "-c", "2:Linux filesystem",
                &disk,
            ],
        )?;
        (Some(partition_path(&disk, 1)), partition_path(&disk, 2))
    };

    // Inform the kernel of the new table.
    ui.status("Refreshing partition table...");
    command::run("partprobe", &[&disk])
        .context("partprobe failed — the kernel may not see new partitions")?;

    ui.status("Updated partition layout:");
    command::run("lsblk", &[])?;

    ui.success("Automated partitioning complete.");

    Ok(AutoResult {
        efi_part,
        linux_part,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_path_sata() {
        assert_eq!(partition_path("/dev/sda", 1), "/dev/sda1");
        assert_eq!(partition_path("/dev/sdb", 3), "/dev/sdb3");
    }

    #[test]
    fn partition_path_nvme() {
        assert_eq!(partition_path("/dev/nvme0n1", 1), "/dev/nvme0n1p1");
        assert_eq!(partition_path("/dev/nvme0n1", 2), "/dev/nvme0n1p2");
    }

    #[test]
    fn partition_path_mmcblk() {
        assert_eq!(partition_path("/dev/mmcblk0", 1), "/dev/mmcblk0p1");
    }
}
