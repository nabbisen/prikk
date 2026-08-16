//! Prikk's sole FFI surface (DC-96 Windows Anchor Identity). Every future FFI need lands here
//! rather than motivating a second entry in `UNSAFE_EXEMPT_CRATES`, which DC-90 forbids anyway --
//! see that gate's own module doc for why at most one workspace crate may hold this exemption.
//! Compiles to nothing on non-Windows targets: there is no FFI need there today, and this crate
//! exists to hold the one that does, not to anticipate ones that don't yet.

/// Identity of an open filesystem object on Windows: the `(volume serial number, file index)`
/// pair that distinguishes one directory object from another on a volume
/// (`GetFileInformationByHandle`, Microsoft Learn). Meaningful only within one boot on one
/// volume -- never derive an object id, a container path, or any on-disk artifact from it, and
/// never persist it past the process that read it.
#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileIdentity {
    volume_serial_number: u32,
    file_index: u64,
}

/// Read `file`'s identity from its already-open handle. The caller owns opening the handle,
/// including share flags and reparse-point policy (`FILE_SHARE_DELETE`,
/// `FILE_FLAG_OPEN_REPARSE_POINT`) -- this function does one thing: read one out-parameter and
/// combine two of its fields.
#[cfg(windows)]
pub fn identity_of(file: &std::fs::File) -> std::io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let handle: HANDLE = file.as_raw_handle();
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `handle` is a valid, open HANDLE for the duration of this call -- it is borrowed
    // from `file`, which outlives the call and is not closed here. `info` is `#[repr(C)]` with
    // every field a plain integer (`#[derive(Default)]`, windows-sys 0.61.2), so a well-formed
    // `&mut` pointer to it is always a valid write target for every field
    // `GetFileInformationByHandle` writes on success. On failure the function returns 0 (`BOOL`
    // false) and writes nothing we read; the OS error is read via
    // `std::io::Error::last_os_error` immediately after, before any other call can overwrite it.
    let succeeded = unsafe { GetFileInformationByHandle(handle, &raw mut info) };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(FileIdentity {
        volume_serial_number: info.dwVolumeSerialNumber,
        file_index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::{FileIdentity, identity_of};

    /// DC-96 design-v1.md §6.4: without this, a `FileIdentity` that never equals itself -- or
    /// always equals everything -- would pass every other test in this increment silently.
    #[test]
    fn identity_distinguishes_different_files_and_matches_the_same_one() -> std::io::Result<()> {
        let directory = std::env::temp_dir();
        let suffix = std::process::id();
        let path_a = directory.join(format!("prikk-ffi-identity-test-a-{suffix}"));
        let path_b = directory.join(format!("prikk-ffi-identity-test-b-{suffix}"));
        std::fs::write(&path_a, b"a")?;
        std::fs::write(&path_b, b"b")?;

        let identity_a = identity_of(&std::fs::File::open(&path_a)?)?;
        let identity_b = identity_of(&std::fs::File::open(&path_b)?)?;
        let identity_a_reopened = identity_of(&std::fs::File::open(&path_a)?)?;

        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);

        assert_ne!(
            identity_a, identity_b,
            "two different files must not compare equal"
        );
        assert_eq!(
            identity_a, identity_a_reopened,
            "the same file, reopened, must compare equal"
        );
        Ok(())
    }

    /// Guards the struct's own shape: if a future edit adds a field this derive doesn't cover, or
    /// removes `PartialEq`, this fails to compile rather than silently comparing fewer fields than
    /// intended.
    #[test]
    fn file_identity_is_copy_and_comparable() {
        fn assert_bounds<T: Copy + PartialEq + Eq + std::fmt::Debug>() {}
        assert_bounds::<FileIdentity>();
    }
}
