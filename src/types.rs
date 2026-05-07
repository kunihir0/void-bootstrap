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

    /// Mount options for an arbitrary BTRFS subvolume.
    pub(crate) fn subvol_mount_opts(self, subvol: &str) -> String {
        format!("defaults,compress=zstd,space_cache=v2,subvol={subvol}")
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

// ── Partitioning types ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PartitionStrategy {
    Auto,
    Manual,
}

impl PartitionStrategy {
    pub(crate) const SELECT_OPTIONS: &[&str] = &["Automated", "Manual"];
}

impl fmt::Display for PartitionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("Automated"),
            Self::Manual => f.write_str("Manual"),
        }
    }
}

impl std::str::FromStr for PartitionStrategy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Automated" => Ok(Self::Auto),
            "Manual" => Ok(Self::Manual),
            _ => Err(format!("Unknown partition strategy: '{s}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VolumeManager {
    Standard,
    Lvm,
}

impl VolumeManager {
    pub(crate) const SELECT_OPTIONS: &[&str] = &["Standard", "LVM"];

    /// Default Volume Group name used by the installer.
    pub(crate) const VG_NAME: &str = "vg_void";
    /// Default root Logical Volume name.
    pub(crate) const LV_ROOT: &str = "lv_root";
}

impl fmt::Display for VolumeManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard => f.write_str("Standard"),
            Self::Lvm => f.write_str("LVM"),
        }
    }
}

impl std::str::FromStr for VolumeManager {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Standard" => Ok(Self::Standard),
            "LVM" => Ok(Self::Lvm),
            _ => Err(format!("Unknown volume manager: '{s}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BtrfsLayout {
    Simple,
    Full,
}

impl BtrfsLayout {
    pub(crate) const SELECT_OPTIONS: &[&str] = &["Simple", "Full"];

    /// Return the subvolume definitions for this layout.
    pub(crate) fn subvolumes(self) -> &'static [BtrfsSubvol] {
        match self {
            Self::Simple => &BTRFS_SIMPLE_SUBVOLS,
            Self::Full => &BTRFS_FULL_SUBVOLS,
        }
    }
}

impl fmt::Display for BtrfsLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple => f.write_str("Simple"),
            Self::Full => f.write_str("Full"),
        }
    }
}

impl std::str::FromStr for BtrfsLayout {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Simple" => Ok(Self::Simple),
            "Full" => Ok(Self::Full),
            _ => Err(format!("Unknown BTRFS layout: '{s}'")),
        }
    }
}

/// A single BTRFS subvolume definition.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BtrfsSubvol {
    /// Subvolume name (e.g. `@`, `@home`).
    pub name: &'static str,
    /// Mount point relative to TARGET (empty string = root).
    pub mountpoint: &'static str,
}

const BTRFS_SIMPLE_SUBVOLS: [BtrfsSubvol; 1] = [BtrfsSubvol {
    name: "@",
    mountpoint: "",
}];

const BTRFS_FULL_SUBVOLS: [BtrfsSubvol; 5] = [
    BtrfsSubvol {
        name: "@",
        mountpoint: "",
    },
    BtrfsSubvol {
        name: "@home",
        mountpoint: "home",
    },
    BtrfsSubvol {
        name: "@log",
        mountpoint: "var/log",
    },
    BtrfsSubvol {
        name: "@cache",
        mountpoint: "var/cache",
    },
    BtrfsSubvol {
        name: "@snapshots",
        mountpoint: ".snapshots",
    },
];

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

    // ── partition strategy ──────────────────────────────────────

    #[test]
    fn partition_strategy_round_trips() {
        for label in PartitionStrategy::SELECT_OPTIONS {
            let parsed: PartitionStrategy = label.parse().unwrap();
            assert_eq!(parsed.to_string(), *label);
        }
    }

    #[test]
    fn partition_strategy_rejects_unknown() {
        assert!("Magic".parse::<PartitionStrategy>().is_err());
    }

    // ── volume manager ─────────────────────────────────────────

    #[test]
    fn volume_manager_round_trips() {
        for label in VolumeManager::SELECT_OPTIONS {
            let parsed: VolumeManager = label.parse().unwrap();
            assert_eq!(parsed.to_string(), *label);
        }
    }

    #[test]
    fn volume_manager_rejects_unknown() {
        assert!("ZFS".parse::<VolumeManager>().is_err());
    }

    // ── btrfs layout ───────────────────────────────────────────

    #[test]
    fn btrfs_layout_round_trips() {
        for label in BtrfsLayout::SELECT_OPTIONS {
            let parsed: BtrfsLayout = label.parse().unwrap();
            assert_eq!(parsed.to_string(), *label);
        }
    }

    #[test]
    fn btrfs_layout_rejects_unknown() {
        assert!("Exotic".parse::<BtrfsLayout>().is_err());
    }

    #[test]
    fn btrfs_simple_has_root_only() {
        let subvols = BtrfsLayout::Simple.subvolumes();
        assert_eq!(subvols.len(), 1);
        assert_eq!(subvols[0].name, "@");
    }

    #[test]
    fn btrfs_full_has_multiple_subvols() {
        let subvols = BtrfsLayout::Full.subvolumes();
        assert!(subvols.len() > 1);
        assert_eq!(subvols[0].name, "@");
        assert!(subvols.iter().any(|sv| sv.name == "@home"));
        assert!(subvols.iter().any(|sv| sv.name == "@snapshots"));
    }
}
