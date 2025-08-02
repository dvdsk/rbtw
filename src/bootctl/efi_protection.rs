use color_eyre::eyre::Context;
use color_eyre::Result;
use color_eyre::Section;
use core::ffi::c_long;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;

mod ioctl {
    use core::ffi::c_long;
    // See table at:
    // https://www.kernel.org/doc/html/latest/userspace-api/ioctl/ioctl-number.html
    type Code = u8;
    const FS: Code = b'f';

    // The table from kernel.org/doc points to the fs.h header. You should
    // find your kernel headers at /usr/src/linux-headers-<some_version>.
    // Go there and open: include/uapi/linux/fs.h
    //
    // There we find:
    // ```c
    // #define	FS_IOC_GETFLAGS			_IOR('f', 1, long)
    // #define	FS_IOC_SETFLAGS			_IOW('f', 2, long)
    // ```
    //
    // From which we derive that the sequence
    type SeqenceNumber = u8;
    const GETFLAGS: SeqenceNumber = 1;
    const SETFLAGS: SeqenceNumber = 2;

    nix::ioctl_read!(get_flags, FS, GETFLAGS, c_long);
    nix::ioctl_write_ptr!(set_flags, FS, SETFLAGS, c_long);
}

pub fn get_flags(path: &Path) -> Result<isize> {
    let file = File::open(path).wrap_err("Could not open file")?;
    let mut flags: c_long = 0;
    let flags_ptr = &mut flags as *mut c_long;

    unsafe { ioctl::get_flags(file.as_raw_fd(), flags_ptr) }.wrap_err("ioctl setflags failed")?;

    Ok(flags.try_into().expect("c_long should equal isize"))
}

pub fn set_flags(path: &Path, flags: isize) -> Result<()> {
    let file = File::open(path).wrap_err("Could not open file")?;
    let mut flags = flags as c_long;
    let flags_ptr = &mut flags as *mut c_long;

    unsafe { ioctl::set_flags(file.as_raw_fd(), flags_ptr) }.wrap_err("ioctl setflags failed")?;
    Ok(())
}

pub fn remove(path: &Path) -> Result<()> {
    // from man ioctl_iflags(2)
    //
    // The file is immutable: no changes are permitted to the file
    // contents or metadata (permissions,  timestamps,  ownership,
    // link count and so on).  (This restriction applies even to the
    // superuser.)  Only a privileged process (CAP_LINUX_IMMUTABLE)
    // can set or clear this attribute.

    // Again from include/uapi/linux/fs.h we get:
    // ```c
    // #define FS_IMMUTABLE_FL         0x00000010 /* Immutable file */
    // ```
    pub const FS_IMMUTABLE_FL: isize = 0x00000010; // Immutable file

    let attr = get_flags(path)
        .wrap_err("Could not read attributes")
        .with_note(|| format!("path: {}", path.display()))?;
    let attr = attr & !FS_IMMUTABLE_FL;
    set_flags(path, attr)
        .wrap_err("Could not write attributes")
        .with_note(|| format!("path: {}", path.display()))
}
