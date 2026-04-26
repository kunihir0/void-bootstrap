use anyhow::Result;
use clap::Parser;
use std::os::unix::fs::MetadataExt;

pub(crate) mod context;
pub(crate) mod stage;
pub(crate) mod types;
pub(crate) mod ui;
pub(crate) mod util;
pub(crate) mod validation;

use ui::Ui;

/// Void Linux Automated Bootstrap (TTY)
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

fn main() {
    let cli = Cli::parse();

    // Propagate --no-color as the standard env var so Ui::new() picks it up.
    if cli.no_color {
        // SAFETY: called before any threads are spawned; single-threaded at this point.
        unsafe { std::env::set_var("NO_COLOR", "1") };
    }

    let ui = Ui::new();

    if let Err(e) = preflight().and_then(|()| stage::run_pipeline(&ui)) {
        ui.error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

/// Pre-flight checks that must pass before touching the filesystem.
fn preflight() -> Result<()> {
    let uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(u32::MAX);

    if uid != 0 {
        anyhow::bail!(
            "This installer must be run as root (current uid: {uid}).\n\
             Re-run with: sudo {}",
            std::env::args().next().unwrap_or_default()
        );
    }

    Ok(())
}
