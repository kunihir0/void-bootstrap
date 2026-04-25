use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub(crate) fn run(command: &str, args: &[&str]) -> Result<()> {
    // In ui.rs, we handle printing. But here we just print for now or wait until ui.rs?
    // Actually, util/command.rs should probably not print directly, but the original code did.
    // The plan said ">> Running: ..." could stay or move. The plan mentions ui.rs has status(&str).
    // For now, let's just keep the println to match original behavior, or remove it and let stages call ui.status().
    // Actually, in the plan: "All user-facing output goes through ui module".
    // I will remove the println here and let the caller or StageRunner handle it, or we just pass a string to ui.
    // For simplicity, let's keep it pure, but it might be handy to log it. Let's comment out println! for now.
    // Wait, let's just keep `println!(">> Running: {} {}", command, args.join(" "));` to maintain exact behavior until ui.rs takes over everything.
    println!(">> Running: {command} {}", args.join(" "));
    let status = Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute {command}"))?;

    if !status.success() {
        anyhow::bail!("Command failed with status: {status}");
    }
    Ok(())
}

pub(crate) fn run_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute {command}"))?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn run_chroot(args: &[&str]) -> Result<()> {
    run("chroot", &[&["/mnt"], args].concat())
}

pub(crate) fn block_device_uuid(partition: &str) -> Result<String> {
    let uuid = run_output("blkid", &["-s", "UUID", "-o", "value", partition])?;
    let uuid = uuid.trim().to_string();
    if uuid.is_empty() {
        anyhow::bail!("Could not read UUID for {partition}. Formatting may have failed.");
    }
    Ok(uuid)
}
