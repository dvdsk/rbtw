#![feature(unix_chown)]

use std::env;
use std::os::unix::fs::chown;
use std::process::Command;

mod efi;
mod setuid;

const ROOT: u32 = 0;
const HELP: &str = "Usage: call without any arguments to restart to windows. \n\n\
        - Requires sudo on its first run, will rerun with sudo when not provided. \n\
        - The next boot after this will be to the default OS again.\n\
        - Will stop if no windows bootloader is found.\n\n\
        Options:\n    --help, -h    Print this help message\n";

fn main() {
    if let Some(arg) = env::args().nth(1) {
        if arg == "--help" || arg == "-h" {
            println!("{HELP}");
            return;
        }
    }

    sudo::escalate_if_needed()
        .expect("sudo failed, you may also call rbtw with sudo in front of it");

    let was_set = setuid::is_set();

    let path = std::env::current_exe().unwrap();
    chown(path, Some(ROOT), Some(ROOT)).unwrap();
    setuid::set();

    if !was_set {
        println!("next time you can run without sudo!");
        println!(
            "(setuid bit and permissions set, {} is now owned by root)",
            std::env::args().next().unwrap()
        );
    }

    efi::set_next_boot();
    Command::new("reboot").arg("now").status().unwrap();
}
