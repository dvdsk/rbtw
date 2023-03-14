#![feature(unix_chown)]


use std::os::unix::fs::chown;

use std::process::Command;

mod efi;
mod gid;

const ROOT: u32 = 0;

fn main() {
    sudo::escalate_if_needed().expect("sudo failed");

    let was_set = gid::is_set();

    let path = std::env::current_exe().unwrap();
    chown(path, Some(ROOT), Some(ROOT)).unwrap();
    gid::set();

    if !was_set {
        println!("next time you can run without sudo!");
        println!(
            "(git bit set and {} is now owned by root)",
            std::env::args().next().unwrap()
        );
    }

    efi::set_next_boot();
    Command::new("reboot").arg("now").status().unwrap();
}
