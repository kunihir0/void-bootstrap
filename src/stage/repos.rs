use crate::ui::Ui;
use crate::util::command::run_chroot;
use anyhow::Result;
use std::fs;

const XLIBRE_KEY_URL: &str = "https://github.com/xlibre-void/xlibre/raw/refs/heads/main/repo-keys/x86_64/00:ca:42:57:c9:c0:9a:ec:94:b4:7d:97:e5:a9:aa:1e.plist";
const XLIBRE_KEY_PATH: &str =
    "/var/db/xbps/keys/00:ca:42:57:c9:c0:9a:ec:94:b4:7d:97:e5:a9:aa:1e.plist";
const XLIBRE_REPO_URL: &str = "https://github.com/xlibre-void/xlibre/releases/latest/download/";

pub(crate) fn run(ui: &Ui) -> Result<()> {
    let add_xlibre = ui.confirm("Add the XLibre (X server fork) repository?", false)?;

    if add_xlibre {
        install_xlibre(ui)?;
    }

    Ok(())
}

fn install_xlibre(ui: &Ui) -> Result<()> {
    // wget/curl are not available in a fresh base install,
    // so we pull wget into the chroot first.
    ui.status("Installing wget for key retrieval...");
    run_chroot(&["xbps-install", "-y", "wget"])?;

    ui.status("Downloading XLibre repository key...");
    run_chroot(&["wget", "-O", XLIBRE_KEY_PATH, XLIBRE_KEY_URL])?;

    ui.status("Adding XLibre repository configuration...");
    fs::create_dir_all("/mnt/etc/xbps.d")?;
    fs::write(
        "/mnt/etc/xbps.d/99-repository-xlibre.conf",
        format!("repository={XLIBRE_REPO_URL}\n"),
    )?;

    ui.success("XLibre repository added successfully.");
    Ok(())
}
