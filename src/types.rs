use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FsType {
    Ext4,
    Btrfs,
    Xfs,
}

impl FsType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ext4 => "ext4",
            Self::Btrfs => "btrfs",
            Self::Xfs => "xfs",
        }
    }

    pub(crate) fn mount_opts(self) -> &'static str {
        match self {
            Self::Btrfs => "defaults,compress=zstd,space_cache=v2,subvol=@",
            _ => "defaults",
        }
    }

    pub(crate) fn fstab_dump_pass(self) -> &'static str {
        match self {
            Self::Btrfs => "0 0",
            _ => "0 1",
        }
    }

    /// Labels used in the interactive select prompt.
    pub(crate) const SELECT_OPTIONS: &[&str] = &["ext4", "btrfs", "xfs"];
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FsType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ext4" => Ok(Self::Ext4),
            "btrfs" => Ok(Self::Btrfs),
            "xfs" => Ok(Self::Xfs),
            _ => Err(format!("Unknown filesystem type: '{s}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    None,
}

impl GpuVendor {
    pub(crate) fn packages(self) -> &'static [&'static str] {
        match self {
            Self::Amd => &[
                "linux-firmware-amd",
                "mesa-dri",
                "mesa-vaapi",
                "mesa-vulkan-radeon",
            ],
            Self::Intel => &[
                "linux-firmware-intel",
                "mesa-dri",
                "mesa-vaapi",
                "intel-video-accel",
            ],
            Self::Nvidia => &["nvidia", "nvidia-libs"],
            Self::None => &[],
        }
    }

    /// Labels used in the interactive select prompt.
    pub(crate) const SELECT_OPTIONS: &[&str] = &["AMD", "Intel", "NVIDIA", "None"];
}

impl fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Amd => f.write_str("AMD"),
            Self::Intel => f.write_str("Intel"),
            Self::Nvidia => f.write_str("NVIDIA"),
            Self::None => f.write_str("None"),
        }
    }
}

impl std::str::FromStr for GpuVendor {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "AMD" => Ok(Self::Amd),
            "Intel" => Ok(Self::Intel),
            "NVIDIA" => Ok(Self::Nvidia),
            "None" => Ok(Self::None),
            _ => Err(format!("Unknown GPU vendor: '{s}'")),
        }
    }
}

pub(crate) const XBPS_REPO: &str = "https://repo-default.voidlinux.org/current";
pub(crate) const VALID_ENCODINGS: &[&str] =
    &["UTF-8", "ISO8859-1", "ISO8859-15", "EUC-JP", "EUC-KR"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_type_display_matches_as_str() {
        for fs in [FsType::Ext4, FsType::Btrfs, FsType::Xfs] {
            assert_eq!(fs.to_string(), fs.as_str());
        }
    }

    #[test]
    fn fs_type_round_trips_through_fromstr() {
        for label in FsType::SELECT_OPTIONS {
            let parsed: FsType = label.parse().unwrap();
            assert_eq!(parsed.to_string(), *label);
        }
    }

    #[test]
    fn fs_type_fromstr_rejects_unknown() {
        assert!("zfs".parse::<FsType>().is_err());
    }

    #[test]
    fn btrfs_has_distinct_mount_opts() {
        assert_ne!(FsType::Btrfs.mount_opts(), FsType::Ext4.mount_opts());
        assert!(FsType::Btrfs.mount_opts().contains("subvol=@"));
    }

    #[test]
    fn btrfs_has_no_fsck_pass() {
        assert_eq!(FsType::Btrfs.fstab_dump_pass(), "0 0");
        assert_eq!(FsType::Ext4.fstab_dump_pass(), "0 1");
    }

    #[test]
    fn gpu_vendor_round_trips_through_fromstr() {
        for label in GpuVendor::SELECT_OPTIONS {
            let parsed: GpuVendor = label.parse().unwrap();
            assert_eq!(parsed.to_string(), *label);
        }
    }

    #[test]
    fn gpu_vendor_fromstr_rejects_unknown() {
        assert!("Qualcomm".parse::<GpuVendor>().is_err());
    }

    #[test]
    fn nvidia_has_packages() {
        assert!(!GpuVendor::Nvidia.packages().is_empty());
    }

    #[test]
    fn gpu_none_has_no_packages() {
        assert!(GpuVendor::None.packages().is_empty());
    }
}
