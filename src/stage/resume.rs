use crate::context::{InstallContext, TARGET};
use crate::types::{BtrfsLayout, FsType, VolumeManager};
use crate::util::command::run_output;
use anyhow::{Context, Result};

/// Reconstruct an `InstallContext` by probing the already-mounted target at
/// `/mnt`.  This lets the installer resume from a later stage without
/// re-running disk setup / base install.
pub(crate) fn reconstruct_context() -> Result<InstallContext> {
    // ── Root device + filesystem type ──────────────────────────
    let root_source = findmnt_field(TARGET, "SOURCE")
        .context("Cannot determine root device — is /mnt mounted?")?;
    let root_fstype =
        findmnt_field(TARGET, "FSTYPE").context("Cannot determine root filesystem type")?;

    let fs_type = match root_fstype.as_str() {
        "ext4" => FsType::Ext4,
        "btrfs" => FsType::Btrfs,
        "xfs" => FsType::Xfs,
        other => anyhow::bail!("Unsupported root filesystem: {other}"),
    };

    // ── EFI device ─────────────────────────────────────────────
    let efi_mount = format!("{TARGET}/boot/efi");
    let efi_device = findmnt_field(&efi_mount, "SOURCE")
        .context("Cannot determine EFI device — is /mnt/boot/efi mounted?")?;

    // ── BTRFS layout ───────────────────────────────────────────
    let btrfs_layout = if fs_type == FsType::Btrfs {
        detect_btrfs_layout(&root_source)?
    } else {
        None
    };

    // ── Volume manager ─────────────────────────────────────────
    let volume_mgr = if root_source.starts_with("/dev/vg_")
        || root_source.starts_with(&format!("/dev/{}/", VolumeManager::VG_NAME))
    {
        VolumeManager::Lvm
    } else {
        VolumeManager::Standard
    };

    Ok(InstallContext {
        root_device: root_source,
        efi_device,
        fs_type,
        btrfs_layout,
        volume_mgr,
    })
}

/// Query a single field from `findmnt` for a given mountpoint.
fn findmnt_field(mountpoint: &str, field: &str) -> Result<String> {
    let out = run_output("findmnt", &["-n", "-o", field, mountpoint])?;
    let val = out.trim().to_string();
    if val.is_empty() {
        anyhow::bail!("findmnt returned empty {field} for {mountpoint}");
    }
    Ok(val)
}

/// Detect BTRFS subvolume layout by listing subvolumes on the device.
fn detect_btrfs_layout(_device: &str) -> Result<Option<BtrfsLayout>> {
    let output = run_output("btrfs", &["subvolume", "list", TARGET]);
    let output = match output {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    let has_home = output.lines().any(|l| l.contains("@home"));
    let has_snapshots = output.lines().any(|l| l.contains("@snapshots"));

    if has_home || has_snapshots {
        Ok(Some(BtrfsLayout::Full))
    } else {
        Ok(Some(BtrfsLayout::Simple))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lvm_detection_by_vg_name() {
        // Devices under /dev/vg_void/ should be detected as LVM.
        assert!("/dev/vg_void/lv_root".starts_with(&format!("/dev/{}/", VolumeManager::VG_NAME)));
    }
}
