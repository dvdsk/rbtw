use std::io::Write;
use std::os::unix::fs::chown;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Context;
use color_eyre::Section;

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

fn configure_next_boot(boot_target: &str) -> color_eyre::Result<()> {
    let mut adapter = efibootnext::Adapter::default();
    if let Some(num) = efi::boot_num(&mut adapter, boot_target) {
        adapter
            .set_boot_next(num)
            .wrap_err("Failed to configure UEFI bootnext")?;
        Ok(())
    } else if let Ok(entry) = bootctl::matching_entry(boot_target) {
        bootctl::set_loader_entry_oneshot(entry)
            .wrap_err("Could not configure systemd-boot oneshot")?;
        Ok(())
    } else {
        Err(color_eyre::eyre::eyre!(
            "Could not find matching UEFI or systemd-boot entry"
        ))
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
pub(crate) use showln;
