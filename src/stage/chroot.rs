use crate::context::TARGET;
use crate::ui::Ui;
use crate::util::command::run;
use anyhow::Result;
use std::fs;
use std::process::Command;

/// RAII guard for chroot bind mounts.
///
/// Bind-mounts `/dev`, `/proc`, `/sys`, `/run` into the target on
/// creation and unmounts them on drop — keeping setup and teardown
/// in the same place.
pub(crate) struct ChrootGuard;

impl ChrootGuard {
    /// Bind-mount host filesystems into the target and copy DNS config.
    pub(crate) fn enter(ui: &Ui) -> Result<Self> {
        // Create the guard first so that if a mount below fails, Drop
        // still runs and cleans up any partial mounts.
        let guard = Self;

        ui.status("Binding host filesystems into chroot...");
        for dir in &["dev", "proc", "sys", "run"] {
            run(
                "mount",
                &["--rbind", &format!("/{dir}"), &format!("{TARGET}/{dir}")],
            )?;
            run("mount", &["--make-rslave", &format!("{TARGET}/{dir}")])?;
        }

        let host_resolv = "/etc/resolv.conf";
        let target_resolv = format!("{TARGET}/etc/resolv.conf");
        if std::path::Path::new(host_resolv).exists() {
            ui.status("Copying DNS configuration...");
            let _ = fs::remove_file(&target_resolv);
            fs::copy(host_resolv, &target_resolv)?;
        }

        ui.success("Chroot environment ready.");
        Ok(guard)
    }
}

impl Drop for ChrootGuard {
    fn drop(&mut self) {
        // Best-effort cleanup — umount failures are silently ignored
        // because we may be tearing down after a failed stage.
        for dir in &["run", "sys", "proc", "dev"] {
            let _ = Command::new("umount")
                .args(["-R", &format!("{TARGET}/{dir}")])
                .status();
        }
    }
}
