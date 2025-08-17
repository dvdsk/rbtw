use color_eyre::eyre::{eyre, Context, OptionExt};
use color_eyre::{Result, Section};
use efibootnext::Adapter;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::bootctl::{self, BootEntry};
use crate::efi;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootTarget {
    Efi { pattern: String },
    SystemdBoot { title: String, root: String },
}

impl BootTarget {
    pub fn to_bytes(&self) -> Vec<u8> {
        ron::to_string(self)
            .expect("Ron can serialize enums and strings")
            .into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let s = str::from_utf8(bytes).wrap_err("Data was not utf8")?;
        ron::from_str(s)
            .wrap_err("Could not deserialize BootTarget")
            .with_note(|| format!("data was: {s}"))
            .suggestion("Set boot target again with --boot-target")
    }

    pub fn resolve(pattern: String) -> Result<Self> {
        let mut adapter = efibootnext::Adapter::default();
        if efi::boot_num(&mut adapter, &pattern)?.is_some() {
            Ok(BootTarget::Efi { pattern })
        } else if let Some(BootEntry { title, root, .. }) = bootctl::matching_pattern(&pattern)? {
            Ok(BootTarget::SystemdBoot { title, root })
        } else {
            Err(no_matching_entry_error(&mut adapter, &pattern).unwrap_err())
        }
    }

    pub fn configure_next_boot(&self) -> Result<()> {
        match self {
            BootTarget::Efi { pattern } => {
                let mut adapter = efibootnext::Adapter::default();
                let num = efi::boot_num(&mut adapter, pattern)?
                    .ok_or_eyre("Could not find boot number")
                    .suggestion("Try resetting boot target with: --set-target")?;
                adapter
                    .set_boot_next(num)
                    .wrap_err("Failed to configure UEFI bootnext")
            }
            BootTarget::SystemdBoot { title, root } => {
                if let Some(entry) = bootctl::matching_pattern(title)? {
                    if entry.root != *root {
                        Err(eyre!("Root for configured OS changed"))
                    .suggestion("If the OS was reinstalled or moved to another disk try resetting boot target with: --set-target")
                    } else {
                        bootctl::set_loader_entry_oneshot(entry)
                            .wrap_err("Could not configure systemd-boot oneshot")
                    }
                } else if let Some(entry) = bootctl::matching_root(root)? {
                    if inquire::Confirm::new(
                        "The title of the boot entry changed, \
                        do you want us to change it back?",
                    )
                    .prompt()?
                    {
                        bootctl::rename_entry_title(&entry, title)
                            .wrap_err("Failed to rename boot entry")?;
                    }

                    bootctl::set_loader_entry_oneshot(entry)
                        .wrap_err("Could not configure systemd-boot oneshot")
                } else {
                    todo!()
                }
            }
        }
    }
}

fn no_matching_entry_error(adapter: &mut Adapter, boot_target: &str) -> Result<()> {
    let list = efi::list(adapter)?
        .iter()
        .map(ToString::to_string)
        .chain(bootctl::list()?.iter().map(ToString::to_string))
        .join("\n  - ");
    Err(eyre!("No boot entry that matches"))
        .with_note(|| format!("target pattern: {boot_target}"))
        .with_note(|| format!("available boot targets:\n  - {list}"))
}
