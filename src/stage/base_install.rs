use crate::context::InstallContext;
use crate::types::{FsType, GpuVendor, XBPS_REPO};
use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::copy_dir_all;
use anyhow::Result;
use std::fs;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    let gpu_str = ui.select(
        "Select GPU vendor for drivers:",
        vec!["AMD", "Intel", "NVIDIA", "None"],
    )?;

    let gpu = match gpu_str {
        "AMD" => GpuVendor::Amd,
        "Intel" => GpuVendor::Intel,
        "NVIDIA" => GpuVendor::Nvidia,
        "None" => GpuVendor::None,
        _ => unreachable!("Select widget handles exhaustiveness"),
    };

    let mut base_packages = vec![
        "base-system",
        "grub-x86_64-efi",
        "linux-mainline",
        "NetworkManager",
        "glibc-locales",
        "efibootmgr",
    ];

    if ctx.fs_type == FsType::Btrfs {
        base_packages.push("btrfs-progs");
    }

    fs::create_dir_all("/mnt/var/db/xbps/keys")?;
    copy_dir_all("/var/db/xbps/keys", "/mnt/var/db/xbps/keys")?;

    if gpu == GpuVendor::Nvidia {
        ui.status("Setting up Void non-free repository for NVIDIA...");
        ui.note("For Wayland support, you may also need 'nvidia-dkms'.");
        ui.note("Add 'nvidia-drm.modeset=1' to your GRUB_CMDLINE_LINUX.");
        command::run(
            "xbps-install",
            &[
                "-y",
                "-S",
                "-R",
                XBPS_REPO,
                "-r",
                "/mnt",
                "--",
                "void-repo-nonfree",
            ],
        )?;
    }

    base_packages.extend(gpu.packages().iter().copied());

    let mut xbps_args = vec!["-y", "-S", "-R", XBPS_REPO, "-r", "/mnt", "--"];
    xbps_args.extend(base_packages.iter().copied());
    command::run("xbps-install", &xbps_args)?;

    Ok(())
}
