use std::os::unix::fs::PermissionsExt;

const SETGID_BIT: u32 = 0o2000;

pub fn is_set() -> bool {
    let path = std::env::current_exe().unwrap();
    let permissions = path.metadata().unwrap().permissions();
    permissions.mode() & SETGID_BIT != 0
}

pub fn set() {
    let path = std::env::current_exe().unwrap();
    let mut permissions = path.metadata().unwrap().permissions();
    permissions.set_mode(permissions.mode() | SETGID_BIT);
    std::fs::set_permissions(&path, permissions).unwrap();
}
