use std::fmt::Display;

use color_eyre::eyre::Context;
use color_eyre::Result;
use efibootnext::{Adapter, LoadOption};
use itertools::Itertools;

use crate::showln;

#[derive(Debug, Clone)]
pub struct BootEntry {
    title: String,
    number: u16,
}

impl Display for BootEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.title)
    }
}

impl From<LoadOption> for BootEntry {
    fn from(value: LoadOption) -> Self {
        Self {
            title: value.description,
            number: value.number,
        }
    }
}

pub fn list(adapter: &mut Adapter) -> Result<Vec<BootEntry>> {
    adapter
        .load_options()
        .wrap_err("Failed to iterate efi options")?
        .map(|res| res.wrap_err("Could not load efi entry"))
        .map_ok(BootEntry::from)
        .collect()
}

pub fn boot_num(adapter: &mut Adapter, boot_target: &str) -> Result<Option<u16>> {
    let mut efi_entries = list(adapter)?
        .into_iter()
        .filter(|BootEntry { title, .. }| {
            title.to_lowercase().contains(&boot_target.to_lowercase())
        })
        .map(|o| o.number);

    let Some(num) = efi_entries.next() else {
        return Ok(None);
    };

    if efi_entries.next().is_some() {
        showln!("multiple {boot_target} efi boot loaders, starting first, exiting");
    }
    Ok(Some(num))
}
