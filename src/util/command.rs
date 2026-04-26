use crate::context::TARGET;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub(crate) fn run(command: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute '{command}'"))?;

    if !status.success() {
        anyhow::bail!("'{command} {}' failed with {status}", args.join(" "));
    }
    Ok(())
}

pub(crate) fn run_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute '{command}'"))?;

    if !output.status.success() {
        anyhow::bail!(
            "'{command} {}' failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn run_chroot(args: &[&str]) -> Result<()> {
    run("chroot", &[&[TARGET], args].concat())
}

pub(crate) fn block_device_uuid(partition: &str) -> Result<String> {
    let uuid = run_output("blkid", &["-s", "UUID", "-o", "value", partition])?;
    let uuid = uuid.trim().to_string();
    if uuid.is_empty() {
        anyhow::bail!("Could not read UUID for {partition}. Formatting may have failed.");
    }
    Ok(uuid)
}
