//! A rather 'creative' way to store the target OS parameter. Instead of a config file
//! we append the setting to the executable. Otherwise having multiple
//! commands would require aliasses and root owned read only config files.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::str::FromStr;

use color_eyre::eyre::Context;
use color_eyre::{Result, Section};
use itertools::Itertools;

/// random string
const STORE_START: &[u8] = "rsientbmefu39285cmser".as_bytes();
const STORE_END: &[u8] = "3894nestmvcsrent3".as_bytes();

pub struct Store {
    binary_bytes: Vec<u8>,
    pub data_bytes: Vec<u8>,
}

impl Store {
    pub fn open() -> Result<Self> {
        let path = std::env::current_exe().wrap_err("Could not get location of executable")?;
        let mut file = File::open(&path)
            .wrap_err("Could not open execable for reading")
            .with_note(|| format!("executable path: {}", path.display()))?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)
            .wrap_err("Failed to read executable to memory")?;

        Ok(if let Some(range) = data_range(&file_content) {
            Store {
                binary_bytes: file_content[0..range.header_start].to_vec(),
                data_bytes: file_content[range.data_start..range.data_end].to_vec(),
            }
        } else {
            Store {
                binary_bytes: file_content,
                data_bytes: Vec::new(),
            }
        })
    }

    pub fn set_data(self, target: &str) -> Result<()> {
        let original = std::env::current_exe().wrap_err("Could not get location of executable")?;
        let original_permissions = std::fs::metadata(&original)
            .wrap_err("Could not get permissions for current executable")?
            .permissions();

        let file_name = original.file_name().expect("executable has a a file name");
        let mut tmp_name = OsString::from_str(".").expect("dot fits in OsString");
        tmp_name.push(file_name);

        let copy = original.with_file_name(tmp_name);
        {
            let mut copy_file =
                File::create(&copy).wrap_err("Could not create new file for adding data too")?;
            copy_file
                .write_all(&self.binary_bytes)
                .wrap_err("Could not copy binary")?;
            copy_file
                .write_all(STORE_START)
                .wrap_err("Could not append data")?;
            copy_file
                .write_all(target.as_bytes())
                .wrap_err("Could not append data")?;
            copy_file
                .write_all(STORE_END)
                .wrap_err("Could not append data")?;
            copy_file
                .set_permissions(original_permissions)
                .wrap_err("Could not set the current permissions to the new executable")?;
        }

        fs::rename(&copy, &original)
            .wrap_err("Could not replace the executable with the one with the data stored")
            .with_note(|| format!("copy: {}", copy.display()))
            .with_note(|| format!("executable: {}", original.display()))?;

        Ok(())
    }
}

struct DataRange {
    header_start: usize,
    data_start: usize,
    data_end: usize,
}

fn data_range(file_content: &[u8]) -> Option<DataRange> {
    if !file_content.ends_with(STORE_END) {
        return None;
    }

    let mut split = file_content
        .windows(STORE_START.len())
        .positions(|w| w == STORE_START);

    let start = split.next_back().unwrap();
    let data_end = file_content.len() - STORE_END.len();
    Some(DataRange {
        header_start: start,
        data_start: start + STORE_START.len(),
        data_end,
    })
}
