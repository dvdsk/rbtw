use std::io::Write;
use std::os::unix::fs::chown;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::{eyre, Context};
use color_eyre::{Result, Section};

mod bootctl;
mod efi;
mod setuid;
mod store;

const ROOT: u32 = 0;

/// Reboot directly into another OS witout password prompt regardless of
/// bootloader & UEFI defaults. The next boot after will again go to the
/// booloader/UEFI default. Requires sudo on first run.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Configure which target we should try to boot
    /// then exit without rebooting
    ///
    /// Example usage: --set-target Windows
    #[arg(short, long)]
    set_target: Option<String>,
    /// Show the target that we will boot then exit
    #[arg(short, long)]
    current_target: bool,
    /// Only configure the next reboot dont start a reboot
    #[arg(short, long)]
    no_reboot: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    let args = Args::parse();
    let store = store::Store::open()?;

    let boot_target = if let Some(boot_target) = args.set_target {
        check_if_target_exists(&boot_target)?;
        setuid::unset();
        store.set_data(&boot_target)?;
        println!("Boot target configured! Run again to reboot to it");
        return Ok(());
    } else if store.data_bytes.is_empty() {
        println!("No boot target configured, please set one with: --set-target");
        return Ok(());
    } else {
        String::from_utf8(store.data_bytes).expect("we only store utf8 strings")
    };

    if args.current_target {
        println!("Boot target: {boot_target}");
        return Ok(());
    }

    sudo::escalate_if_needed()
        .expect("sudo failed, you may also call rbtw with sudo in front of it");

    let was_set = setuid::is_set();

    let path = std::env::current_exe().unwrap();
    chown(path, Some(ROOT), Some(ROOT)).unwrap();
    setuid::set();

    if !was_set {
        let path = std::env::args().next().unwrap();
        println!(
            "- setuid bit and permissions set and \n    {path}\n\
             now owned by root\n\
            - next time you can run without sudo!\n\
            - next time reboot will happen instandly"
        );
        if !args.no_reboot {
            for i in (1..=10).rev() {
                print!("\rrebooting in {i}s ");
                std::io::stdout().flush().unwrap();
                sleep(Duration::from_secs(1));
            }
        }
    }

    configure_next_boot(&boot_target)
        .wrap_err("Failed to configure next boot")
        .with_note(|| format!("tried to find OS matching: {boot_target}"))?;

    if !args.no_reboot {
        Command::new("reboot")
            .arg("now")
            .status()
            .wrap_err("Failed to call reboot")?;
    }
    Ok(())
}

fn check_if_target_exists(boot_target: &str) -> Result<()> {
    let mut adapter = efibootnext::Adapter::default();
    if efi::boot_num(&mut adapter, boot_target)?.is_some()
        || bootctl::matching_entry(boot_target)?.is_some()
    {
        Ok(())
    } else {
        no_matching_entry_error(&mut adapter, boot_target)
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

fn configure_next_boot(boot_target: &str) -> Result<()> {
    let mut adapter = efibootnext::Adapter::default();
    if let Some(num) = efi::boot_num(&mut adapter, boot_target)? {
        adapter
            .set_boot_next(num)
            .wrap_err("Failed to configure UEFI bootnext")?;
        Ok(())
    } else if let Some(entry) = bootctl::matching_entry(boot_target)? {
        bootctl::set_loader_entry_oneshot(entry)
            .wrap_err("Could not configure systemd-boot oneshot")?;
        Ok(())
    } else {
        no_matching_entry_error(&mut adapter, boot_target)
    }
}

/// A println that first sleeps for 5 seconds so the message can be seen
macro_rules! showln {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        eprintln!("Continuing in 5 seconds");
        ::std::thread::sleep(::std::time::Duration::from_secs(5));
    }};
}
use efibootnext::Adapter;
use itertools::Itertools;
pub(crate) use showln;
