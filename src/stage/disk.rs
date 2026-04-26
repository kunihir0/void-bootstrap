use crate::context::InstallContext;
use crate::types::FsType;
use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::Result;
use std::fs;
use std::process::Command;

pub(crate) fn run(ui: &Ui) -> Result<InstallContext> {
    ui.status("Current disk layout:");
    command::run("lsblk", &[])?;

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

    let efi_part = ui.prompt_validated(
        "Enter the EFI partition (e.g., /dev/nvme0n1p1):",
        None,
        |p| validate_block_device(p).map_err(|e| format!("Invalid: {e}. Please try again.")),
    )?;

    let fs_type: FsType =
        ui.select_parsed("Root filesystem type:", FsType::SELECT_OPTIONS.to_vec())?;

    let format_root = ui.confirm_destructive(
        "All data on this partition will be permanently erased.",
        "Format the ROOT partition now?",
    )?;

    if format_root {
        ui.status(&format!("Formatting {root_part} as {fs_type}..."));
        match fs_type {
            FsType::Ext4 => command::run("mkfs.ext4", &["-F", &root_part])?,
            FsType::Xfs => command::run("mkfs.xfs", &["-f", &root_part])?,
            FsType::Btrfs => {
                command::run("mkfs.btrfs", &["-f", &root_part])?;
                ui.status("Creating btrfs subvolume '@'...");
                fs::create_dir_all("/tmp/btrfs-setup")?;
                command::run("mount", &[&root_part, "/tmp/btrfs-setup"])?;

                scopeguard::defer! {
                    let _ = Command::new("umount").arg("/tmp/btrfs-setup").status();
                }

                command::run("btrfs", &["subvolume", "create", "/tmp/btrfs-setup/@"])?;
            }
        }
        ui.success("Root partition formatted.");
    }

    let format_efi_initial = ui.confirm_destructive(
        "Formatting the EFI partition will destroy existing bootloaders (Windows/OpenCore).",
        "Format the EFI partition?",
    )?;

    let mut format_efi = false;
    if format_efi_initial {
        let confirm_text = ui.prompt_text(
            "Are you SURE? Type 'YES' in all caps to format the EFI partition:",
            None,
        )?;
        format_efi = confirm_text == "YES";

        if !format_efi {
            ui.info("EFI formatting cancelled.");
        }
    }

    if format_efi {
        ui.status(&format!("Formatting {efi_part} as FAT32..."));
        command::run("mkfs.fat", &["-F32", &efi_part])?;
        ui.success("EFI partition formatted.");
    } else {
        ui.info("Skipping EFI format. Existing bootloaders will be preserved.");
    }

    Ok(InstallContext {
        root_part,
        efi_part,
        fs_type,
    })
}
