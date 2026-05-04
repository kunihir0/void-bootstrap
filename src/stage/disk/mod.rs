use crate::context::InstallContext;
use crate::types::{BtrfsLayout, FsType, PartitionStrategy, VolumeManager};
use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::Result;

mod auto;
mod btrfs;
mod format;
mod lvm;
mod manual;

pub(crate) fn run(ui: &Ui) -> Result<InstallContext> {
    ui.status("Current disk layout:");
    command::run("lsblk", &[])?;

    // ── Step 1: Existing EFI partition? ─────────────────────────
    let use_existing_efi = ui.confirm(
        "Do you have an existing EFI partition to reuse? (e.g., dual-booting with another OS)",
        false,
    )?;

    let existing_efi: Option<String> = if use_existing_efi {
        let efi = ui.prompt_validated(
            "Enter the existing EFI partition (e.g., /dev/nvme0n1p1):",
            None,
            |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
        )?;
        Some(efi)
    } else {
        None
    };

    // ── Step 2: Partitioning strategy ───────────────────────────
    let strategy: PartitionStrategy = ui.select_parsed(
        "Choose your partitioning strategy:",
        PartitionStrategy::SELECT_OPTIONS.to_vec(),
    )?;

    let (root_device, efi_device) = match strategy {
        PartitionStrategy::Auto => {
            let result = auto::run(ui, existing_efi.as_deref())?;
            let efi = existing_efi.unwrap_or_else(|| {
                result
                    .efi_part
                    .expect("auto mode must create EFI when none was provided")
            });
            (result.linux_part, efi)
        }
        PartitionStrategy::Manual => {
            let result = manual::run(ui, existing_efi.as_deref())?;
            (result.root_part, result.efi_part)
        }
    };

    // ── Step 3: Volume management ───────────────────────────────
    let volume_mgr: VolumeManager = ui.select_parsed(
        "Select volume management:",
        VolumeManager::SELECT_OPTIONS.to_vec(),
    )?;

    let root_device = if volume_mgr == VolumeManager::Lvm {
        lvm::setup(ui, &root_device)?
    } else {
        root_device
    };

    // ── Step 4: Filesystem & layout ─────────────────────────────
    let fs_type: FsType =
        ui.select_parsed("Root filesystem type:", FsType::SELECT_OPTIONS.to_vec())?;

    let btrfs_layout = if fs_type == FsType::Btrfs {
        if volume_mgr == VolumeManager::Lvm {
            ui.warning("BTRFS on LVM — NEVER use LVM-level snapshots!");
            ui.warning(
                "BTRFS is highly sensitive to UUID duplication. An LVM snapshot creates \
                 a second block device with the same BTRFS UUID; the kernel may treat \
                 both as part of the same filesystem, leading to catastrophic data corruption.",
            );
            ui.info("Use BTRFS's native subvolume snapshots (e.g. snapper, btrbk) instead.");
        }

        ui.info("Simple  — single '@' subvolume (classic layout).");
        ui.info("Full    — '@', '@home', '@log', '@cache', '@snapshots' (snapshot-friendly).");
        let layout: BtrfsLayout = ui.select_parsed(
            "Select BTRFS subvolume layout:",
            BtrfsLayout::SELECT_OPTIONS.to_vec(),
        )?;
        Some(layout)
    } else {
        None
    };

    // ── Step 5: Format & create structures ──────────────────────
    format::format_root(ui, &root_device, fs_type)?;

    if fs_type == FsType::Btrfs {
        let layout = btrfs_layout.unwrap_or(BtrfsLayout::Simple);
        btrfs::create_subvolumes(ui, &root_device, layout)?;
    }

    // Only format the EFI partition if the user didn't bring an existing one,
    // or if they explicitly ask.
    if use_existing_efi {
        let format_efi = ui.confirm_destructive(
            "Formatting will destroy existing bootloaders on this EFI partition.",
            "Format the existing EFI partition?",
        )?;
        if format_efi {
            format::format_efi(ui, &efi_device)?;
        } else {
            ui.info("Skipping EFI format. Existing bootloaders will be preserved.");
        }
    } else {
        // Auto-created EFI partition — always format it.
        format::format_efi(ui, &efi_device)?;
    }

    Ok(InstallContext {
        root_device,
        efi_device,
        fs_type,
        btrfs_layout,
        volume_mgr,
    })
}
