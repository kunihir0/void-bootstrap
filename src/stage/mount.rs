use crate::context::{InstallContext, TARGET};
use crate::types::{BtrfsLayout, FsType};
use crate::ui::Ui;
use crate::util::command;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    if command::run_output("findmnt", &["-M", TARGET]).is_ok() {
        let auto_unmount = ui.confirm(
            &format!(
                "{TARGET} is already mounted (likely from a previous run). Auto-unmount it now?"
            ),
            true,
        )?;

        if auto_unmount {
            ui.status("Unmounting lingering partitions...");
            let _ = Command::new("umount").args(["-R", TARGET]).status();

            if command::run_output("findmnt", &["-M", TARGET]).is_ok() {
                anyhow::bail!(
                    "Failed to unmount {TARGET} completely. Please unmount manually: umount -R {TARGET}"
                );
            }
        } else {
            anyhow::bail!("{TARGET} is already mounted. Unmount it before running the installer.");
        }
    }

    // ── Mount root ──────────────────────────────────────────────
    mount_root(ui, ctx)?;

    // ── Mount additional BTRFS subvolumes ───────────────────────
    if let Some(layout) = ctx.btrfs_layout {
        mount_btrfs_subvolumes(ui, ctx, layout)?;
    }

    // ── Mount EFI ───────────────────────────────────────────────
    let efi_mount = ctx.target_path("boot/efi");
    let efi_mount_str = efi_mount.to_string_lossy();
    ui.status(&format!(
        "Mounting {} at {efi_mount_str}...",
        ctx.efi_device
    ));
    fs::create_dir_all(&efi_mount).context("Failed to create EFI directory")?;
    command::run("mount", &[&ctx.efi_device, &efi_mount_str])?;

    // ── Clean up existing EFI & NVRAM entries ───────────────────
    let void_efi_dir = ctx.target_path("boot/efi/EFI/Void");
    if void_efi_dir.exists() {
        if ui.confirm("Detected existing Void Linux bootloader files in the EFI partition. Remove them to ensure a clean install?", true)? {
            // Safety check: assert the path ends with EFI/Void
            if void_efi_dir.ends_with("EFI/Void") || void_efi_dir.ends_with("EFI\\Void") {
                ui.status("Cleaning up old EFI/Void directory...");
                fs::remove_dir_all(&void_efi_dir).context("Failed to remove old EFI directory")?;
            } else {
                ui.warning("Safety check failed: Path does not end with EFI/Void. Skipping cleanup.");
            }
        }
    }

    if std::path::Path::new("/sys/firmware/efi/efivars").exists() {
        if let Ok(output) = command::run_output("efibootmgr", &[]) {
            let mut stale_entries = Vec::new();
            for line in output.lines() {
                if line.starts_with("Boot") && line.contains("Void") {
                    stale_entries.push(line.to_string());
                }
            }

            if !stale_entries.is_empty() {
                ui.info("Found the following stale 'Void' entries in the UEFI boot menu:");
                for entry in &stale_entries {
                    ui.info(&format!("  {entry}"));
                }
                
                if ui.confirm("Remove these specific 'Void' entries from the motherboard UEFI boot menu?", true)? {
                    for entry in stale_entries {
                        if let Some(boot_str) = entry.split('*').next() {
                            if boot_str.len() >= 8 {
                                let boot_num = &boot_str[4..8];
                                ui.status(&format!("Removing NVRAM entry {boot_num}..."));
                                let _ = command::run("efibootmgr", &["-b", boot_num, "-B", "-q"]);
                            }
                        }
                    }
                }
            }
        }
    }

    ui.success("Partitions mounted.");

    Ok(())
}

/// Mount the root subvolume / partition.
fn mount_root(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    ui.status(&format!("Mounting {} at {TARGET}...", ctx.root_device));

    if ctx.fs_type == FsType::Btrfs {
        let opts = ctx.fs_type.mount_opts(); // includes subvol=@
        command::run("mount", &["-o", opts, &ctx.root_device, TARGET])?;
    } else {
        command::run("mount", &[&ctx.root_device, TARGET])?;
    }

    Ok(())
}

/// Mount non-root BTRFS subvolumes (e.g. @home, @log, @cache, @snapshots).
fn mount_btrfs_subvolumes(ui: &Ui, ctx: &InstallContext, layout: BtrfsLayout) -> Result<()> {
    for sv in layout.subvolumes() {
        // The root subvolume (@) is already mounted above.
        if sv.mountpoint.is_empty() {
            continue;
        }

        let mount_target = ctx.target_path(sv.mountpoint);
        let mount_target_str = mount_target.to_string_lossy().to_string();
        let opts = ctx.fs_type.subvol_mount_opts(sv.name);

        ui.status(&format!(
            "Mounting subvol {} at {mount_target_str}...",
            sv.name
        ));
        fs::create_dir_all(&mount_target)?;
        command::run("mount", &["-o", &opts, &ctx.root_device, &mount_target_str])?;
    }

    Ok(())
}
