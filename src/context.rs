use crate::types::{BtrfsLayout, FsType, VolumeManager};
use std::path::PathBuf;

/// Root mountpoint for the target system.
pub(crate) const TARGET: &str = "/mnt";

#[derive(Debug)]
pub(crate) struct InstallContext {
    /// Device path for the root filesystem.
    /// Standard partition (e.g. `/dev/nvme0n1p2`) or LVM LV (e.g. `/dev/vg_void/lv_root`).
    pub root_device: String,
    /// Device path for the EFI system partition.
    pub efi_device: String,
    pub fs_type: FsType,
    /// `None` when the filesystem is not BTRFS.
    pub btrfs_layout: Option<BtrfsLayout>,
    pub volume_mgr: VolumeManager,
}

impl InstallContext {
    /// Resolve a path relative to the target root.
    ///
    /// `ctx.target_path("etc/hostname")` → `"/mnt/etc/hostname"`
    pub(crate) fn target_path(&self, relative: &str) -> PathBuf {
        PathBuf::from(TARGET).join(relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_ctx() -> InstallContext {
        InstallContext {
            root_device: String::new(),
            efi_device: String::new(),
            fs_type: FsType::Ext4,
            btrfs_layout: None,
            volume_mgr: VolumeManager::Standard,
        }
    }

    #[test]
    fn target_path_joins_correctly() {
        let ctx = dummy_ctx();
        assert_eq!(
            ctx.target_path("etc/hostname"),
            PathBuf::from("/mnt/etc/hostname")
        );
        assert_eq!(ctx.target_path("boot/efi"), PathBuf::from("/mnt/boot/efi"));
    }
}
