use crate::ui::Ui;
use anyhow::{Context, Result};

pub(crate) mod base_install;
pub(crate) mod bootloader;
pub(crate) mod chroot;
pub(crate) mod configure;
pub(crate) mod disk;
pub(crate) mod mount;
pub(crate) mod repos;
pub(crate) mod resume;
pub(crate) mod users;

/// Number of stages in the install pipeline.
pub(crate) const STAGE_COUNT: usize = 8;

/// Human-readable labels for each stage (1-indexed for display).
pub(crate) const STAGE_NAMES: [&str; STAGE_COUNT] = [
    "Disk Setup",
    "Mounting Partitions",
    "Installing Base System via XBPS",
    "Configuring the Chroot Environment",
    "Native System Configuration",
    "Installing GRUB Bootloader",
    "Finalizing Users and Services",
    "Additional Repositories",
];

pub(crate) struct StageRunner<'a> {
    ui: &'a Ui,
    step: usize,
    resume_from: usize,
}

impl<'a> StageRunner<'a> {
    pub(crate) fn new(ui: &'a Ui, resume_from: usize) -> Self {
        Self {
            ui,
            step: 0,
            resume_from,
        }
    }

    /// Run a stage.  If `resume_from` is set and this stage's number is
    /// below the resume point, the stage is skipped and the provided
    /// `skip_value` is returned instead.
    pub(crate) fn run_or_skip<F, T>(
        &mut self,
        name: &str,
        skip_value: T,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&Ui) -> Result<T>,
    {
        self.step += 1;
        if self.step < self.resume_from {
            self.ui.step(self.step, &format!("{name} [skipped — resuming]"));
            return Ok(skip_value);
        }
        self.ui.step(self.step, name);
        f(self.ui).with_context(|| format!("Stage '{name}' failed"))
    }
}

pub(crate) fn run_pipeline(ui: &Ui, resume_from: usize) -> Result<()> {
    ui.banner();

    let mut runner = StageRunner::new(ui, resume_from);

    // ── Stage 1: Disk Setup ─────────────────────────────────────
    // When resuming, reconstruct the install context from the existing
    // mounted state instead of re-running disk setup.
    let ctx = if resume_from > 1 {
        runner.step += 1;
        ui.step(1, "Disk Setup [skipped — resuming]");
        ui.status("Reconstructing install context from mounted filesystems...");
        let ctx = resume::reconstruct_context()?;
        ui.success(&format!(
            "Detected: root={} efi={} fs={} vol={}",
            ctx.root_device, ctx.efi_device, ctx.fs_type, ctx.volume_mgr,
        ));
        ctx
    } else {
        runner.step += 1;
        ui.step(1, "Disk Setup");
        disk::run(ui).context("Stage 'Disk Setup' failed")?
    };

    // ── Stage 2: Mounting Partitions ────────────────────────────
    runner.run_or_skip("Mounting Partitions", (), |ui| mount::run(ui, &ctx))?;

    // ── Stage 3: Installing Base System ─────────────────────────
    runner.run_or_skip("Installing Base System via XBPS", (), |ui| {
        base_install::run(ui, &ctx)
    })?;

    // ── Stage 4: Chroot Environment ─────────────────────────────
    // ChrootGuard binds host filesystems on enter and unmounts on drop.
    // Even when resuming past this stage, we always re-enter the chroot
    // because stages 5–8 need the bind mounts active.
    runner.step += 1;
    if resume_from > 4 {
        ui.step(runner.step, "Configuring the Chroot Environment [re-entering]");
    } else {
        ui.step(runner.step, "Configuring the Chroot Environment");
    }
    let _chroot = chroot::ChrootGuard::enter(ui)?;

    // ── Stage 5: Native System Configuration ────────────────────
    runner.run_or_skip("Native System Configuration", (), |ui| {
        configure::run(ui, &ctx)
    })?;

    // ── Stage 6: GRUB Bootloader ────────────────────────────────
    let nvram_updated =
        runner.run_or_skip("Installing GRUB Bootloader", false, bootloader::run)?;

    // ── Stage 7: Users and Services ─────────────────────────────
    runner.run_or_skip("Finalizing Users and Services", (), users::run)?;

    // ── Stage 8: Additional Repositories ────────────────────────
    runner.run_or_skip("Additional Repositories", (), repos::run)?;

    ui.completion(nvram_updated);

    Ok(())
}
