use crate::context::InstallContext;
use crate::types::FsType;
use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::validate_block_device;
use anyhow::Result;
use std::fs;
use std::process::Command;

pub(crate) fn run(ui: &Ui) -> Result<InstallContext> {
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

    let fs_type_str = ui.select("Root filesystem type:", vec!["ext4", "btrfs", "xfs"])?;
    let fs_type = match fs_type_str {
        "ext4" => FsType::Ext4,
        "btrfs" => FsType::Btrfs,
        "xfs" => FsType::Xfs,
        _ => unreachable!("Select widget handles exhaustiveness"),
    };

    let format_root = ui.confirm(
        "DANGER: Format the ROOT partition now? All data on this partition will be lost.",
        false,
    )?;

    if format_root {
        match fs_type {
            FsType::Ext4 => command::run("mkfs.ext4", &["-F", &root_part])?,
            FsType::Xfs => command::run("mkfs.xfs", &["-f", &root_part])?,
            FsType::Btrfs => {
                command::run("mkfs.btrfs", &["-f", &root_part])?;
                fs::create_dir_all("/tmp/btrfs-setup")?;
                command::run("mount", &[&root_part, "/tmp/btrfs-setup"])?;

                scopeguard::defer! {
                    let _ = Command::new("umount").arg("/tmp/btrfs-setup").status();
                }

                command::run("btrfs", &["subvolume", "create", "/tmp/btrfs-setup/@"])?;
            }
        }
    }

    let format_efi_initial = ui.confirm(
        "DANGER: Format the EFI partition? (Choose 'No' if sharing with Windows/OpenCore!)",
        false,
    )?;

    let mut format_efi = false;
    if format_efi_initial {
        let confirm_text = ui.prompt_text(
            "Are you SURE? Type 'YES' in all caps to format the EFI partition:",
            None,
        )?;
        format_efi = confirm_text == "YES";

        if !format_efi {
            ui.status("EFI formatting cancelled.");
        }
    }

    if format_efi {
        command::run("mkfs.fat", &["-F32", &efi_part])?;
    } else {
        ui.status("Skipping EFI format. Existing bootloaders will be preserved.");
    }

    Ok(InstallContext {
        root_part,
        efi_part,
        fs_type,
    })
}
