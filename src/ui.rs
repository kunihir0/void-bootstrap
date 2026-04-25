#![allow(clippy::unused_self)]

use anyhow::Result;
use inquire::{Confirm, Select, Text};

pub(crate) struct Ui;

impl Ui {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn banner(&self) {
        println!("========================================");
        println!("  Void Linux Automated Bootstrap (TTY)");
        println!("========================================");
    }

    pub(crate) fn section(&self, step: usize, title: &str) {
        println!("\n[{step}] {title}");
    }

    pub(crate) fn status(&self, msg: &str) {
        println!(">> {msg}");
    }

    pub(crate) fn warning(&self, msg: &str) {
        println!(">> WARNING: {msg}");
    }

    pub(crate) fn note(&self, msg: &str) {
        println!("NOTE: {msg}");
    }

    pub(crate) fn completion(&self, nvram_updated: bool) {
        println!("\n========================================");
        println!("Installation Complete!");

        if nvram_updated {
            println!("\n>> Void Linux has been registered in your UEFI boot menu.");
            println!(">> You can use your motherboard's boot menu (F12/F8) to select your OS.");
        } else {
            println!("\n[!] CRITICAL NEXT STEPS FOR DUAL/MULTI-BOOT");
            println!("Because '--no-nvram' was used, Void is NOT yet in your UEFI boot list.");
            println!(
                "Before rebooting, you must do ONE of the following to ensure you can boot Void:\n"
            );
            println!("  A) Add to OpenCore Config:");
            println!("     Point a Misc > Entries item at \\EFI\\Void\\grubx64.efi");
            println!("     (Ensure Misc > Boot > LauncherOption is set properly if needed).\n");
            println!("  B) Register via efibootmgr (run before unmounting):");
            println!("     efibootmgr --create --disk /dev/sdX --part N \\");
            println!("       --label 'Void Linux' --loader '\\EFI\\Void\\grubx64.efi'\n");
            println!(
                "     (Verify exact path with: ls /mnt/boot/efi/EFI/Void/ before registering)\n"
            );
        }

        println!("Once done, safely unmount and reboot:");
        println!("  umount -R /mnt && reboot");
        println!("========================================");
    }

    pub(crate) fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        let result = Confirm::new(prompt).with_default(default).prompt()?;
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
                Err(e) => eprintln!("{e}"),
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
