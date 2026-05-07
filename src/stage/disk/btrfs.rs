use crate::types::BtrfsLayout;
use crate::ui::Ui;
use crate::util::command;
use anyhow::Result;
use std::fs;
use std::process::Command;

const BTRFS_SETUP_DIR: &str = "/tmp/btrfs-setup";

/// Format the device as BTRFS, mount it temporarily, and create all
/// subvolumes dictated by `layout`.
pub(crate) fn create_subvolumes(ui: &Ui, device: &str, layout: BtrfsLayout) -> Result<()> {
    let subvols = layout.subvolumes();

    ui.status(&format!(
        "Creating {} BTRFS subvolume(s) on {device}...",
        subvols.len()
    ));

    fs::create_dir_all(BTRFS_SETUP_DIR)?;
    command::run("mount", &[device, BTRFS_SETUP_DIR])?;

    // Ensure we unmount even if subvolume creation fails.
    scopeguard::defer! {
        let _ = Command::new("umount").arg(BTRFS_SETUP_DIR).status();
    }

    for sv in subvols {
        let path = format!("{BTRFS_SETUP_DIR}/{}", sv.name);
        ui.info(&format!("  subvolume: {} → /{}", sv.name, sv.mountpoint));
        
        if let Some(parent) = std::path::Path::new(&path).parent() {
            fs::create_dir_all(parent)?;
        }

        command::run("btrfs", &["subvolume", "create", &path])?;
        if sv.nocow {
            ui.info(&format!("    Disabling COW on {}", sv.name));
            command::run("chattr", &["+C", &path])?;
        }
    }

    ui.success("BTRFS subvolumes created.");
    Ok(())
}
