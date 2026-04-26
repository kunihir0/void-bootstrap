use anyhow::Result;
use inquire::ui::{Color, RenderConfig, StyleSheet, Styled};
use inquire::{Confirm, Select, Text};
use std::io::IsTerminal;

pub(crate) struct Ui {
    color: bool,
}

impl Ui {
    pub(crate) fn new() -> Self {
        let color = std::io::stdout().is_terminal()
            && std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").map_or(true, |t| t != "dumb");

        if color {
            let mut config = RenderConfig::default_colored();
            config.prompt_prefix = Styled::new(">").with_fg(Color::DarkCyan);
            config.answered_prompt_prefix = Styled::new(">").with_fg(Color::DarkGreen);
            config.prompt = StyleSheet::new().with_fg(Color::White);
            config.answer = StyleSheet::new().with_fg(Color::DarkGreen);
            config.error_message = config
                .error_message
                .with_prefix(Styled::new("!!").with_fg(Color::DarkRed));
            inquire::set_global_render_config(config);
        } else {
            inquire::set_global_render_config(RenderConfig::empty());
        }

        Self { color }
    }

    /// Apply an ANSI SGR code to text, respecting NO_COLOR / non-TTY.
    fn ansi(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    // ── Severity output ─────────────────────────────────────────

    /// Top-of-run branding banner.
    pub(crate) fn banner(&self) {
        let rule = self.ansi("1;36", "========================================");
        let title = self.ansi("1;37", "  Void Linux Automated Bootstrap (TTY)");
        println!("{rule}");
        println!("{title}");
        println!("{rule}");
    }

    /// Stage header — the most prominent non-error output.
    pub(crate) fn step(&self, num: usize, title: &str) {
        println!();
        println!("{}", self.ansi("1;36", &format!("==> [{num}] {title}")));
    }

    /// Neutral progress — default color, low prominence.
    pub(crate) fn status(&self, msg: &str) {
        println!(" >> {msg}");
    }

    /// Supplementary tip or background info — blue.
    pub(crate) fn info(&self, msg: &str) {
        println!("{} {msg}", self.ansi("34", " ->"));
    }

    /// Positive completion of a sub-task — green.
    pub(crate) fn success(&self, msg: &str) {
        println!("{} {msg}", self.ansi("1;32", " >>"));
    }

    /// Non-blocking caution — yellow, bold.
    pub(crate) fn warning(&self, msg: &str) {
        println!("{} {msg}", self.ansi("1;33", " !!"));
    }

    /// Blocking failure or critical issue — red, bold, stderr.
    pub(crate) fn error(&self, msg: &str) {
        eprintln!("{} {msg}", self.ansi("1;31", " !!"));
    }

    // ── Completion ──────────────────────────────────────────────

    pub(crate) fn completion(&self, nvram_updated: bool) {
        println!();
        let rule = self.ansi("1;32", "========================================");
        println!("{rule}");
        println!("{}", self.ansi("1;32", "  Installation Complete!"));
        println!("{rule}");

        if nvram_updated {
            self.success("Void Linux has been registered in your UEFI boot menu.");
            self.info("Use your motherboard's boot menu (F12/F8) to select your OS.");
        } else {
            println!();
            println!(
                "{}",
                self.ansi("1;31", " !! CRITICAL NEXT STEPS FOR DUAL/MULTI-BOOT")
            );
            self.warning(
                "Because '--no-nvram' was used, Void is NOT yet in your UEFI boot list.",
            );
            println!(
                "    Before rebooting, you must do ONE of the following to ensure you can boot Void:\n"
            );
            println!("    A) Add to OpenCore Config:");
            println!("       Point a Misc > Entries item at \\EFI\\Void\\grubx64.efi");
            println!(
                "       (Ensure Misc > Boot > LauncherOption is set properly if needed).\n"
            );
            println!("    B) Register via efibootmgr (run before unmounting):");
            println!("       efibootmgr --create --disk /dev/sdX --part N \\");
            println!(
                "         --label 'Void Linux' --loader '\\EFI\\Void\\grubx64.efi'\n"
            );
            self.info(
                "Verify exact path with: ls /mnt/boot/efi/EFI/Void/ before registering",
            );
        }

        println!();
        self.info("Once done, safely unmount and reboot:");
        println!("    umount -R /mnt && reboot");
        println!("{rule}");
    }

    // ── Interactive prompts ─────────────────────────────────────

    pub(crate) fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        let result = Confirm::new(prompt).with_default(default).prompt()?;
        Ok(result)
    }

    /// Destructive confirmation — prints a red warning line before the
    /// prompt and defaults to `No`.
    pub(crate) fn confirm_destructive(
        &self,
        warning: &str,
        prompt: &str,
    ) -> Result<bool> {
        println!(
            "{} {}",
            self.ansi("1;31", " !!"),
            self.ansi("1;31", warning)
        );
        let result = Confirm::new(prompt).with_default(false).prompt()?;
        Ok(result)
    }

    pub(crate) fn select<'a>(&self, prompt: &str, options: Vec<&'a str>) -> Result<&'a str> {
        let result = Select::new(prompt, options).prompt()?;
        Ok(result)
    }

    pub(crate) fn prompt_validated<F>(
        &self,
        prompt: &str,
        default: Option<&str>,
        validate: F,
    ) -> Result<String>
    where
        F: Fn(&str) -> std::result::Result<(), String>,
    {
        loop {
            let mut text = Text::new(prompt);
            if let Some(def) = default {
                text = text.with_default(def);
            }
            let input = text.prompt()?;
            match validate(&input) {
                Ok(()) => return Ok(input),
                Err(e) => self.error(&e),
            }
        }
    }

    pub(crate) fn prompt_text(&self, prompt: &str, default: Option<&str>) -> Result<String> {
        let mut text = Text::new(prompt);
        if let Some(def) = default {
            text = text.with_default(def);
        }
        let input = text.prompt()?;
        Ok(input)
    }
}
