use std::os::unix::fs::PermissionsExt;

// ocatal number sys
const SETUID_BIT: u32 = 0o4000;
const OTHERS_READ: u32 = 0o0004;
const OTHERS_EXECUTE: u32 = 0o0001;
const OWNER_FREE: u32 = 0o0700;
const GROUP_NO_PERM: u32 = 0o0000;

const PERM: u32 = SETUID_BIT 
    | OTHERS_READ 
    | OTHERS_EXECUTE 
    | OWNER_FREE 
    | GROUP_NO_PERM;

pub fn is_set() -> bool {
    let path = std::env::current_exe().unwrap();
    let permissions = path.metadata().unwrap().permissions();
    permissions.mode() & SETUID_BIT != 0
}

pub fn set() {
    let path = std::env::current_exe().unwrap();
    let mut permissions = path.metadata().unwrap().permissions();
    permissions.set_mode(PERM);
    std::fs::set_permissions(&path, permissions).unwrap();
}
