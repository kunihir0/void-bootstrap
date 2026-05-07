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

/// Run a command feeding `stdin_data` to its standard input.
pub(crate) fn run_with_stdin(command: &str, args: &[&str], stdin_data: &str) -> Result<()> {
    use std::io::Write;

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to execute '{command}'"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .with_context(|| format!("Failed to write to '{command}' stdin"))?;
    }

    let status = child
        .wait()
        .with_context(|| format!("Failed to wait for '{command}'"))?;

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

/// Run a shell script inside an isolated mount namespace where the target
/// root (`/mnt`) has been `pivot_root`'d to become `/`.
///
/// This is needed for `grub-install` because `grub-probe` reads
/// `/proc/self/mountinfo` which, inside a plain `chroot`, still shows
/// host-relative paths (e.g. `/dev/sdb3 /` for the live USB).  After
/// `pivot_root`, the kernel reports mount paths relative to the new root,
/// so mountinfo correctly shows `/dev/sda2 /` and `/dev/sda1 /boot/efi`.
pub(crate) fn run_pivoted(script: &str) -> Result<()> {
    let wrapper = format!(
        "mount --make-rprivate / && \
         pivot_root {TARGET} {TARGET}/mnt && \
         cd / && \
         umount -l /mnt 2>/dev/null; \
         mount -t proc proc /proc && \
         {script}"
    );

    run("unshare", &["--mount", "--fork", "--", "sh", "-c", &wrapper])
}

pub(crate) fn block_device_uuid(partition: &str) -> Result<String> {
    let uuid = run_output("blkid", &["-s", "UUID", "-o", "value", partition])?;
    let uuid = uuid.trim().to_string();
    if uuid.is_empty() {
        anyhow::bail!("Could not read UUID for {partition}. Formatting may have failed.");
    }
    Ok(uuid)
}
