use crate::ui::Ui;
use crate::util::command::run;
use anyhow::Result;
use std::fs;

pub(crate) fn setup(_ui: &Ui) -> Result<()> {
    for dir in &["dev", "proc", "sys", "run"] {
        run(
            "mount",
            &["--rbind", &format!("/{dir}"), &format!("/mnt/{dir}")],
        )?;
        run("mount", &["--make-rslave", &format!("/mnt/{dir}")])?;
    }

    if std::path::Path::new("/etc/resolv.conf").exists() {
        let _ = fs::remove_file("/mnt/etc/resolv.conf");
        fs::copy("/etc/resolv.conf", "/mnt/etc/resolv.conf")?;
    }

    Ok(())
}
