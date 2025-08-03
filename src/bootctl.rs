use std::fmt::Display;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

use color_eyre::eyre::{eyre, Context, Result};
use efivar::efi::{Variable, VariableFlags, VariableVendor};
use itertools::Itertools;
use uuid::Uuid;

mod efi_protection;

use crate::showln;

const SYSTEMD_BOOT_UUID: &str = "4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";
const SYSTEMD_BOOT: VariableVendor = const {
    match Uuid::try_parse("4a67b082-0a4c-41cf-b6c7-440b29bb8c4f") {
        Err(_) => panic!("could not parse uuid"),
        Ok(uuid) => VariableVendor::Custom(uuid),
    }
};

#[derive(Debug, Clone)]
pub struct BootEntry {
    title: String,
    id: String,
}

impl Display for BootEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.title)
    }
}

impl BootEntry {
    fn from_file(path: &Path) -> Option<Result<Self>> {
        let Some(id) = path.file_name()?.to_str() else {
            return Some(Err(eyre!(
                "Bootloader entry can not be represented in utf8"
            )));
        };

        let mut title = None;
        let s = read_to_string(path).unwrap();
        for line in s.lines() {
            let line = line.trim();
            match (title, line.strip_prefix("title ")) {
                (None, entry_title) => title = entry_title,
                (Some(_), Some(_)) => panic!("two titles in entry"),
                _ => (),
            }
        }

        match title {
            None => panic!("no title in entry with id: {id}"),
            Some(title) => Some(Ok(Self {
                title: title.trim().to_string(),
                id: id.to_string(),
            })),
        }
    }

    fn encode(&self) -> Vec<u8> {
        self.id
            .encode_utf16()
            .flat_map(|char| char.to_le_bytes().into_iter())
            .collect()
    }
}

pub fn list() -> Result<Vec<BootEntry>> {
    read_dir("/boot/efi/loader/entries")
        .wrap_err("Could not read dir: /boot/efi/loader/entries")?
        .filter_map_ok(|e| BootEntry::from_file(&e.path()))
        .flatten()
        .collect::<Result<_, _>>()
        .wrap_err("Could not read entry in /boot/efi/loader/entries")
}

pub fn matching_entry(boot_target: &str) -> Result<Option<BootEntry>> {
    let entries = list()?;
    let mut matches = entries
        .iter()
        .filter(|e| e.title.to_lowercase().contains(&boot_target.to_lowercase()));
    let Some(choice) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        showln!("multiple matching options");
    }

    Ok(Some(choice.clone()))
}

// check if this worked with:
// sudo cat /sys/firmware/efi/efivars/LoaderEntryOneShot-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f
pub fn set_loader_entry_oneshot(choice: BootEntry) -> Result<()> {
    let var = Variable::new_with_vendor("LoaderEntryOneShot", SYSTEMD_BOOT);
    let mut flags = VariableFlags::empty();
    flags.insert(VariableFlags::NON_VOLATILE);
    flags.insert(VariableFlags::BOOTSERVICE_ACCESS);
    flags.insert(VariableFlags::RUNTIME_ACCESS);

    let path = PathBuf::from(format!(
        "/sys/firmware/efi/efivars/LoaderEntryOneShot-{SYSTEMD_BOOT_UUID}"
    ));
    if path.is_file() {
        efi_protection::remove(&path)
            .wrap_err("Could not remove immutable flag protecting efi variable")?;
    }
    efivar::system()
        .write(&var, flags, &choice.encode())
        .wrap_err("Failed to configure systemd-boot through efi variable")?;
    Ok(())
}
