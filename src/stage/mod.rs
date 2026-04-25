use crate::ui::Ui;
use anyhow::{Context, Result};

pub(crate) mod base_install;
pub(crate) mod bootloader;
pub(crate) mod chroot;
pub(crate) mod configure;
pub(crate) mod disk;
pub(crate) mod mount;
pub(crate) mod users;

pub(crate) struct StageRunner<'a> {
    ui: &'a Ui,
    step: usize,
}

impl<'a> StageRunner<'a> {
    pub(crate) fn new(ui: &'a Ui) -> Self {
        Self { ui, step: 0 }
    }

    pub(crate) fn run<F, T>(&mut self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce(&Ui) -> Result<T>,
    {
        self.step += 1;
        self.ui.section(self.step, name);
        f(self.ui).with_context(|| format!("Stage '{name}' failed"))
    }
}

pub(crate) fn run_pipeline(ui: &Ui) -> Result<()> {
    ui.banner();

    let mut runner = StageRunner::new(ui);

    let ctx = runner.run("Disk Setup", disk::run)?;

    runner.run("Mounting Partitions", |ui| mount::run(ui, &ctx))?;
    runner.run("Installing Base System via XBPS", |ui| {
        base_install::run(ui, &ctx)
    })?;

    scopeguard::defer! {
        ui.status("Cleaning up bind mounts...");
        for dir in &["run", "sys", "proc", "dev"] {
            let _ = std::process::Command::new("umount").args(["-R", &format!("/mnt/{dir}")]).status();
        }
    }

    runner.run("Configuring the Chroot Environment", chroot::setup)?;
    runner.run("Native System Configuration", |ui| configure::run(ui, &ctx))?;

    let nvram_updated = runner.run("Installing GRUB Bootloader", bootloader::run)?;

    runner.run("Finalizing Users and Services", users::run)?;

    ui.completion(nvram_updated);

    Ok(())
}
