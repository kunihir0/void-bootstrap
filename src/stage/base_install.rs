use crate::context::{InstallContext, TARGET};
use crate::types::{FsType, GpuVendor, XBPS_REPO};
use crate::ui::Ui;
use crate::util::command;
use crate::util::fs::copy_dir_all;
use anyhow::Result;
use std::fs;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    let gpu: GpuVendor =
        ui.select_parsed("Select GPU vendor for drivers:", GpuVendor::SELECT_OPTIONS.to_vec())?;

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

    let target_keys = ctx.target_path("var/db/xbps/keys");
    fs::create_dir_all(&target_keys)?;
    copy_dir_all("/var/db/xbps/keys", &target_keys)?;

    if gpu == GpuVendor::Nvidia {
        ui.status("Setting up Void non-free repository for NVIDIA...");
        ui.info("For Wayland support, you may also need 'nvidia-dkms'.");
        ui.info("Add 'nvidia-drm.modeset=1' to your GRUB_CMDLINE_LINUX.");
        command::run(
            "xbps-install",
            &[
                "-y",
                "-S",
                "-R",
                XBPS_REPO,
                "-r",
                TARGET,
                "--",
                "void-repo-nonfree",
            ],
        )?;
    }

    base_packages.extend(gpu.packages().iter().copied());

    ui.status("Installing base system packages (this may take a while)...");
    let mut xbps_args = vec!["-y", "-S", "-R", XBPS_REPO, "-r", TARGET, "--"];
    xbps_args.extend(base_packages.iter().copied());
    command::run("xbps-install", &xbps_args)?;

    ui.success("Base system installed.");

    Ok(())
}
