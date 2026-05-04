use crate::context::TARGET;
use crate::ui::Ui;
use crate::util::command::run_chroot;
use crate::validation::validate_username;
use anyhow::Result;
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub(crate) fn run(ui: &Ui) -> Result<()> {
    ui.status("Set ROOT password:");
    set_password(ui, &["passwd"])?;

    let username = ui.prompt_validated("Enter primary username:", Some("baobao"), |u| {
        validate_username(u)
    })?;

    run_chroot(&[
        "useradd",
        "-m",
        "-s",
        "/bin/bash",
        "-G",
        "wheel,audio,video,cdrom,input",
        &username,
    ])?;

    ui.status(&format!("Set password for {username}:"));
    set_password(ui, &["passwd", &username])?;

    let sudoers_path = format!("{TARGET}/etc/sudoers.d/wheel");
    fs::write(&sudoers_path, "%wheel ALL=(ALL:ALL) ALL\n")?;
    fs::set_permissions(&sudoers_path, fs::Permissions::from_mode(0o440))?;

    ui.status("Enabling dbus and NetworkManager services...");
    run_chroot(&["ln", "-s", "/etc/sv/dbus", "/var/service/"]).ok();
    run_chroot(&["ln", "-s", "/etc/sv/NetworkManager", "/var/service/"]).ok();

    ui.success("Users and services configured.");

    Ok(())
}

/// Retry `passwd` until the user enters a valid, matching password.
///
/// `passwd` exits non-zero (e.g. code 10 = "password unchanged") when the
/// input is rejected — this is not a fatal installer error, just a prompt
/// to try again.
fn set_password(ui: &Ui, args: &[&str]) -> Result<()> {
    loop {
        match run_chroot(args) {
            Ok(()) => return Ok(()),
            Err(_) => {
                ui.warning("Password was not set. Please try again.");
            }
        }
    }
}
