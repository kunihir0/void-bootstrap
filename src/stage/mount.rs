use crate::context::InstallContext;
use crate::types::FsType;
use crate::ui::Ui;
use crate::util::command;
use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    if command::run_output("findmnt", &["-M", "/mnt"]).is_ok() {
        let auto_unmount = ui.confirm(
            "/mnt is already mounted (likely from a previous run). Auto-unmount it now?",
            true,
        )?;

        if auto_unmount {
            ui.status("Unmounting lingering partitions...");
            let _ = Command::new("umount").args(["-R", "/mnt"]).status();

            if command::run_output("findmnt", &["-M", "/mnt"]).is_ok() {
                anyhow::bail!(
                    "Failed to unmount /mnt completely. Please unmount manually: umount -R /mnt"
                );
            }
        } else {
            anyhow::bail!("/mnt is already mounted. Unmount it before running the installer.");
        }
    }

    if ctx.fs_type == FsType::Btrfs {
        command::run("mount", &["-o", "subvol=@", &ctx.root_part, "/mnt"])?;
    } else {
        command::run("mount", &[&ctx.root_part, "/mnt"])?;
    }

    fs::create_dir_all("/mnt/boot/efi").context("Failed to create EFI directory")?;
    command::run("mount", &[&ctx.efi_part, "/mnt/boot/efi"])?;

    Ok(())
}
