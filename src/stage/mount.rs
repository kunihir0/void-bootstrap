use crate::context::{InstallContext, TARGET};
use crate::types::FsType;
use crate::ui::Ui;
use crate::util::command;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    if command::run_output("findmnt", &["-M", TARGET]).is_ok() {
        let auto_unmount = ui.confirm(
            &format!("{TARGET} is already mounted (likely from a previous run). Auto-unmount it now?"),
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

    ui.status(&format!("Mounting {} at {TARGET}...", ctx.root_part));
    if ctx.fs_type == FsType::Btrfs {
        command::run("mount", &["-o", "subvol=@", &ctx.root_part, TARGET])?;
    } else {
        command::run("mount", &[&ctx.root_part, TARGET])?;
    }

    let efi_mount = ctx.target_path("boot/efi");
    let efi_mount_str = efi_mount.to_string_lossy();
    ui.status(&format!("Mounting {} at {efi_mount_str}...", ctx.efi_part));
    fs::create_dir_all(&efi_mount).context("Failed to create EFI directory")?;
    command::run("mount", &[&ctx.efi_part, &efi_mount_str])?;

    ui.success("Partitions mounted.");

    Ok(())
}
