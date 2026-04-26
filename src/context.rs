use crate::types::FsType;
use std::path::PathBuf;

/// Root mountpoint for the target system.
pub(crate) const TARGET: &str = "/mnt";

#[derive(Debug)]
pub(crate) struct InstallContext {
    pub root_part: String,
    pub efi_part: String,
    pub fs_type: FsType,
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

    #[test]
    fn target_path_joins_correctly() {
        let ctx = InstallContext {
            root_part: String::new(),
            efi_part: String::new(),
            fs_type: FsType::Ext4,
        };
        assert_eq!(ctx.target_path("etc/hostname"), PathBuf::from("/mnt/etc/hostname"));
        assert_eq!(ctx.target_path("boot/efi"), PathBuf::from("/mnt/boot/efi"));
    }
}
