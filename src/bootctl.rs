use std::fmt::Display;
use std::fs::{self, read_dir, read_to_string};
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
    pub title: String,
    pub root: String,
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

        let mut title_line = None;
        let mut options_line = None;
        let s = read_to_string(path).unwrap();
        for line in s.lines() {
            let line = line.trim();
            match (title_line, line.strip_prefix("title ")) {
                (None, line) => title_line = line,
                (Some(_), Some(_)) => panic!("two titles in entry"),
                _ => (),
            }
            match (options_line, line.strip_prefix("options ")) {
                (None, line) => options_line = line,
                (Some(_), Some(_)) => panic!("two options lines in entry"),
                _ => (),
            }
        }

        let Some((title, options)) = title_line.zip(options_line) else {
            panic!("no title and options line in boot entry with id: {id}");
        };

        let Some((_, root)) = options.split(' ').find_map(|option| option.split_once('=')) else {
            panic!("no title in entry with id: {id}");
        };

        Some(Ok(Self {
            title: title.trim().to_string(),
            root: root.to_string(),
            id: id.to_string(),
        }))
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

pub fn matching_pattern(pattern: &str) -> Result<Option<BootEntry>> {
    let entries = list()?;
    let mut matches = entries
        .iter()
        .filter(|e| e.title.to_lowercase().contains(&pattern.to_lowercase()));
    let Some(choice) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        showln!("multiple matching options");
    }

    Ok(Some(choice.clone()))
}

pub fn matching_root(root: &str) -> Result<Option<BootEntry>> {
    let entries = list()?;
    let mut matches = entries.iter().filter(|e| e.root == root);
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
    assert!(path.is_file());
    efi_protection::remove(&path)
        .wrap_err("Could not remove immutable flag protecting efi variable")?;
    efivar::system()
        .write(&var, flags, &choice.encode())
        .wrap_err("Failed to configure systemd-boot through efi variable")?;
    efi_protection::add(&path)
        .wrap_err("Could not re-add immutable flag protecting efi variable")?;
    Ok(())
}

pub(crate) fn rename_entry_title(BootEntry { id, .. }: &BootEntry, new_title: &str) -> Result<()> {
    let entry_path = format!("/boot/efi/loader/entries/{id}");
    let existing = fs::read_to_string(&entry_path).wrap_err("Could not read existing entry")?;
    let renamed: String = existing
        .lines()
        .map(|line| {
            if line.starts_with("title") {
                line.chars()
                    .skip("title".chars().count())
                    .peekable()
                    .peeking_take_while(|c| c.is_whitespace())
                    .chain(new_title.chars())
                    .collect::<String>()
            } else {
                line.to_string()
            }
        })
        .collect();
    let tmp_entry_path = format!("/boot/efi/loader/entries/{id}_renamed");
    fs::write(&tmp_entry_path, renamed).wrap_err("Could not write tmp entry with renamed title")?;
    fs::rename(tmp_entry_path, entry_path).wrap_err("Could not swap existing entry with new entry")
}
