use crate::context::InstallContext;
use crate::ui::Ui;
use crate::util::command::{block_device_uuid, run_chroot};
use crate::validation::{validate_hostname, validate_locale, validate_timezone};
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) fn run(ui: &Ui, ctx: &InstallContext) -> Result<()> {
    let hostname = ui.prompt_validated("Enter system hostname:", Some("voidlinux"), |h| {
        validate_hostname(h)
    })?;
    fs::write("/mnt/etc/hostname", format!("{hostname}\n"))?;

    let timezone = ui.prompt_validated("Enter timezone:", Some("America/Phoenix"), |tz| {
        validate_timezone(tz, Path::new("/mnt/usr/share/zoneinfo"))
    })?;
    run_chroot(&[
        "ln",
        "-sf",
        &format!("/usr/share/zoneinfo/{timezone}"),
        "/etc/localtime",
    ])?;

    let locale = ui.prompt_validated("Enter system locale:", Some("en_US.UTF-8"), |l| {
        validate_locale(l)
    })?;

    fs::write("/mnt/etc/locale.conf", format!("LANG={locale}\n"))?;
    let libc_locales_path = "/mnt/etc/default/libc-locales";
    if Path::new(libc_locales_path).exists() {
        let contents = fs::read_to_string(libc_locales_path)?;
        let locale_prefix = format!("{locale} ");
        let uncommented = contents
            .lines()
            .map(|line| {
                let unhashed = line.trim_start_matches('#').trim_start();
                if unhashed == locale.as_str() || unhashed.starts_with(&locale_prefix) {
                    unhashed.to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(libc_locales_path, uncommented)?;
    }
    ui.status("Reconfiguring glibc locales...");
    run_chroot(&["xbps-reconfigure", "-f", "glibc-locales"])?;

    let root_uuid = block_device_uuid(&ctx.root_part)?;
    let efi_uuid = block_device_uuid(&ctx.efi_part)?;

    let fs_str = ctx.fs_type.as_str();
    let root_opts = ctx.fs_type.mount_opts();
    let root_dump_pass = ctx.fs_type.fstab_dump_pass();

    let fstab = format!(
        "UUID={root_uuid} / {fs_str} {root_opts} {root_dump_pass}\nUUID={efi_uuid} /boot/efi vfat defaults 0 0\n"
    );
    fs::write("/mnt/etc/fstab", fstab)?;

    ui.success("System configured.");

    Ok(())
}
