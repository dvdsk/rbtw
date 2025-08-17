use std::io::Write;
use std::os::unix::fs::chown;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Context;
use color_eyre::{Result, Section};

use crate::boot_target::BootTarget;

mod boot_target;
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

    if let Some(boot_target) = args.set_target {
        sudo::escalate_if_needed()
            .expect("sudo failed, you may also call rbtw with sudo in front of it");

        let boot_target = BootTarget::resolve(boot_target.clone())?;
        // SECURITY: do not allow booting to another OS then what was configured
        // before authenticating as super user.
        setuid::unset();
        store.set_data(&boot_target.to_bytes())?;
        println!("Boot target configured! Run again to reboot to it");
        return Ok(());
    }

    if store.data_bytes.is_empty() {
        println!("No boot target configured, please set one with: --set-target");
        return Ok(());
    }

    let target = BootTarget::from_bytes(&store.data_bytes)?;

    if args.current_target {
        println!("Boot target: {target:?}");
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

    target
        .configure_next_boot()
        .wrap_err("Failed to configure next boot")
        .with_note(|| format!("tried to find OS matching: {target:?}"))?;

    if !args.no_reboot {
        Command::new("reboot")
            .arg("now")
            .status()
            .wrap_err("Failed to call reboot")?;
    }
    Ok(())
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
