use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::Result;

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

/// Automatically partition a whole disk using `sfdisk` (part of util-linux).
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

    // Wipe existing filesystem signatures so sfdisk starts clean.
    ui.status(&format!("Wiping signatures on {disk}..."));
    command::run("wipefs", &["-a", "-f", &disk])?;

    let (efi_part, linux_part) = if existing_efi.is_some() {
        // Entire disk → single Linux partition.
        ui.status("Creating Linux partition (entire disk)...");
        let script = "label: gpt\n,,L,Linux filesystem\n";
        command::run_with_stdin("sfdisk", &["--wipe", "always", &disk], script)?;
        (None, partition_path(&disk, 1))
    } else {
        // EFI (512M) + Linux (remainder).
        ui.status("Creating EFI (512 MiB) + Linux partitions...");
        let script = "label: gpt\n,512M,U,EFI System\n,,L,Linux filesystem\n";
        command::run_with_stdin("sfdisk", &["--wipe", "always", &disk], script)?;
        (Some(partition_path(&disk, 1)), partition_path(&disk, 2))
    };

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
