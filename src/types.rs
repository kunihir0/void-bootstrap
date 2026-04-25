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
}

pub(crate) const XBPS_REPO: &str = "https://repo-default.voidlinux.org/current";
pub(crate) const VALID_ENCODINGS: &[&str] =
    &["UTF-8", "ISO8859-1", "ISO8859-15", "EUC-JP", "EUC-KR"];
