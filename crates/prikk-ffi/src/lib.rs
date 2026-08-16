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

/// The current path of an already-open handle's object (`GetFinalPathNameByHandle`, Microsoft
/// Learn). A directory handle follows its object across a rename -- re-deriving the path this way
/// before a walk, rather than re-walking a path string captured earlier, is what lets Windows
/// continue operating correctly against the object that was validated even after its directory
/// entry has been renamed elsewhere (DC-96 implementation-ruling-v1 §4). Returned with the
/// `VOLUME_NAME_DOS` / `FILE_NAME_NORMALIZED` flags (value `0`) -- the ordinary drive-letter path
/// form every other function in this crate and its caller already expects, at the cost of the
/// well-known `\\?\` extended-length prefix Windows adds to this form; `std::fs`/`CreateFileW`
/// both accept it transparently.
#[cfg(windows)]
pub fn current_path_of(file: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;

    let handle: HANDLE = file.as_raw_handle();
    // Zero-initialized and grown by `Vec::resize`, never left partially uninitialized -- so
    // `buffer.as_mut_ptr()` below is always a pointer to `buffer.len()` valid, initialized `u16`
    // slots, regardless of how much of that capacity the call actually writes.
    let mut buffer: Vec<u16> = vec![0; 512];
    loop {
        let capacity = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
        // SAFETY: `handle` is a valid, open HANDLE for the duration of this call, borrowed from
        // `file`. `buffer.as_mut_ptr()` points to `capacity` initialized, writable `u16` slots
        // (see above); the function writes at most `capacity` of them and returns the count
        // actually written (success) or the count that would have been required (buffer too
        // small) -- it never writes past what `capacity` promises is available. On failure
        // (return value 0) it writes nothing we read; the OS error is read via
        // `std::io::Error::last_os_error` immediately after, before any other call can clobber
        // it.
        let written =
            unsafe { GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), capacity, 0) };
        if written == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if written < capacity {
            // Success: `written` is the length actually used, excluding the null terminator.
            buffer.truncate(written as usize);
            break;
        }
        // Too small: `written` is the required size, including the null terminator this time.
        buffer.resize(written as usize, 0);
    }
    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}

#[cfg(all(test, windows))]
mod tests {
    use super::{FileIdentity, current_path_of, identity_of};

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

    fn open_directory(path: &std::path::Path) -> std::io::Result<std::fs::File> {
        use std::os::windows::fs::OpenOptionsExt;
        // `FILE_FLAG_BACKUP_SEMANTICS` (`0x02000000`) -- required to obtain a directory handle via
        // `CreateFile` at all (Microsoft Learn, `CreateFileA`, `dwFlagsAndAttributes`). The same
        // constant `windows.rs` uses for the same reason; not re-exported from there to keep this
        // crate's only dependency on its caller one-directional (this crate takes no dependency on
        // `prikk-store`).
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x0200_0000)
            .open(path)
    }

    /// DC-96 implementation-ruling-v1 §4: the mechanism the whole correction rests on. A retained
    /// directory handle must keep resolving to its *current* path after the directory is renamed
    /// out from under it -- this is what turns identity comparison from the sole mechanism
    /// (detection only, and wrong per that ruling) into the secondary confirmation after a walk
    /// that already starts from the right place (prevention).
    #[test]
    fn current_path_of_follows_the_handle_across_a_rename() -> std::io::Result<()> {
        let temporary_root = std::env::temp_dir();
        let suffix = std::process::id();
        let original = temporary_root.join(format!("prikk-ffi-rename-test-original-{suffix}"));
        let renamed = temporary_root.join(format!("prikk-ffi-rename-test-renamed-{suffix}"));
        let _ = std::fs::remove_dir_all(&original);
        let _ = std::fs::remove_dir_all(&renamed);
        std::fs::create_dir(&original)?;

        let handle = open_directory(&original)?;
        let path_before = current_path_of(&handle)?;
        let expected_before = std::fs::canonicalize(&original)?;

        std::fs::rename(&original, &renamed)?;
        let path_after = current_path_of(&handle)?;
        let expected_after = std::fs::canonicalize(&renamed)?;

        let _ = std::fs::remove_dir_all(&renamed);

        assert_eq!(
            path_before, expected_before,
            "before any rename, the handle's path must match the directory it was opened from"
        );
        assert_eq!(
            path_after, expected_after,
            "after renaming the directory out from under the still-open handle, current_path_of \
             must report the NEW path -- this is the whole mechanism DC-96 depends on"
        );
        assert_ne!(
            path_before, path_after,
            "the rename must actually have been observed, not silently ignored"
        );
        Ok(())
    }
}
