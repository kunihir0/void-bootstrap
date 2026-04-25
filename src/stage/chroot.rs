use crate::ui::Ui;
use crate::util::command::run;
use anyhow::Result;

pub(crate) fn setup(_ui: &Ui) -> Result<()> {
    for dir in &["dev", "proc", "sys", "run"] {
        run(
            "mount",
            &["--rbind", &format!("/{dir}"), &format!("/mnt/{dir}")],
        )?;
        run("mount", &["--make-rslave", &format!("/mnt/{dir}")])?;
    }
    Ok(())
}
