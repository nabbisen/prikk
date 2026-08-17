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

/// Best-effort, advisory liveness of a process id (DC-99, `prikk unlock`'s Windows primitive).
/// `prikk-ffi` cannot depend on `prikk-store`, so this returns its own enum -- map to
/// `unlock.rs::PidLiveness` at the call site, the same split `identity_of`/`current_path_of` already
/// draw between this crate's raw Win32 answer and its caller's own vocabulary.
#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProcessLiveness {
    /// A handle was opened and confirmed still running: `WaitForSingleObject` reports the handle
    /// nonsignaled, or the open itself failed with `ERROR_ACCESS_DENIED` -- the kernel found a
    /// process to check permissions against, the same reasoning Unix's `EPERM` branch already
    /// applies (`unlock.rs::check_pid_liveness`).
    Exists,
    /// Positively established absence: `OpenProcess` failed with `ERROR_INVALID_PARAMETER` (no such
    /// process), or a successfully opened handle's process has since terminated
    /// (`WaitForSingleObject` reports the handle signaled).
    DoesNotExist,
    /// Neither established -- an unexpected error, an unanticipated wait result, or a degenerate PID
    /// this function refuses to ask the OS about at all. Never authorization to clear anything; see
    /// `PidLiveness`'s own doc at the call site for why a negative or unknown result is advisory
    /// only.
    Indeterminate,
}

/// PID 0 names the System Idle Process on Windows -- a real PID, but never a value a lock file's own
/// `pid=` field can legitimately record for a user process. Rejected before the OS call rather than
/// let it reach `OpenProcess`, so a corrupt or hand-edited lock file recording `pid=0` produces the
/// same `Indeterminate` answer here that `rustix::process::Pid::from_raw(0)` already produces as
/// `None` on Linux/macOS (`unlock.rs::check_pid_liveness`) -- not merely to avoid a syscall, but to
/// keep the two platforms' answers equal for the same malformed input (DC-99
/// stage-1-investigation-ruling-v1 §2: without this guard, `pid=0` reaches `DoesNotExist` on Windows
/// and `Unknown` on Unix for the identical recorded value, and `DoesNotExist` is the one answer that
/// can authorize clearing a lock).
#[cfg(windows)]
pub fn process_liveness(pid: u32) -> ProcessLiveness {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };

    if pid == 0 {
        return ProcessLiveness::Indeterminate;
    }

    // SAFETY: `OpenProcess` takes no pointer arguments; every argument is a plain integer, and the
    // return value is either NULL (checked immediately below) or a `HANDLE` this function takes
    // ownership of via `OwnedProcessHandle`. Requesting only `PROCESS_QUERY_LIMITED_INFORMATION |
    // PROCESS_SYNCHRONIZE` (not a broader right) is deliberate -- it is queryable against a process
    // this caller does not own, which is exactly the access-denied case this function must
    // distinguish from genuine absence.
    let handle: HANDLE = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        // `std::io::Error::last_os_error` wraps `GetLastError`, read immediately after the call that
        // can set it, matching this crate's other two functions -- no separate FFI import needed for
        // it.
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(code) if code == ERROR_INVALID_PARAMETER as i32 => ProcessLiveness::DoesNotExist,
            Some(code) if code == ERROR_ACCESS_DENIED as i32 => ProcessLiveness::Exists,
            _ => ProcessLiveness::Indeterminate,
        };
    }
    let guard = OwnedProcessHandle(handle);

    // SAFETY: `guard.0` is the valid, open HANDLE just obtained above, not yet closed (the guard
    // closes it on drop, after this call returns and its result is captured). A zero timeout never
    // blocks -- it reports the object's current state and returns immediately either way, so this
    // cannot hang the command that calls it.
    let waited = unsafe { WaitForSingleObject(guard.0, 0) };
    match waited {
        WAIT_TIMEOUT => ProcessLiveness::Exists,
        WAIT_OBJECT_0 => ProcessLiveness::DoesNotExist,
        _ => ProcessLiveness::Indeterminate,
    }
    // `guard` drops here on every path above, including the unwind path if a future edit adds a
    // panicking call between the two `unsafe` blocks -- `CloseHandle` runs in `Drop`, not repeated at
    // each branch, so there is exactly one place a handle leak could hide, and it is not a per-branch
    // decision that could be forgotten in a new arm.
}

/// The `HANDLE` `OpenProcess` returns, owned. `windows-sys` provides no RAII wrapper for `HANDLE`
/// (`identity_of`/`current_path_of` never own one -- they borrow from a `std::fs::File` that already
/// manages its own closing); this is the one function in the crate that does, so it is the one place
/// that needs its own `Drop`.
#[cfg(windows)]
struct OwnedProcessHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid, open HANDLE this struct exclusively owns (constructed only
        // from a non-null `OpenProcess` success in `process_liveness`, never copied or shared) and
        // has not been closed yet -- `Drop::drop` runs at most once per value. `CloseHandle`'s own
        // return value is not checked: there is nothing a liveness check can safely do in response to
        // a close failure, and a leaked handle here would be a real defect (DC-99 design-v1.md §
        // "close the handle") but a checked-and-ignored return would not prevent one.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{FileIdentity, ProcessLiveness, current_path_of, identity_of, process_liveness};

    /// The one case this advisory check can prove reliably: the current process's own PID, which is
    /// definitely alive because it is the process running this assertion.
    #[test]
    fn process_liveness_of_the_current_process_is_exists() {
        assert_eq!(
            process_liveness(std::process::id()),
            ProcessLiveness::Exists
        );
    }

    /// DC-99 stage-1-investigation-ruling-v1 §2: PID 0 must not reach `DoesNotExist` -- it names the
    /// System Idle Process, a real PID, and the same malformed lock-file value produces `Unknown` on
    /// Linux/macOS (`Pid::from_raw(0)` returning `None`). `Indeterminate` here keeps the two
    /// platforms' answers equal for the same degenerate input.
    #[test]
    fn process_liveness_of_pid_zero_is_indeterminate() {
        assert_eq!(process_liveness(0), ProcessLiveness::Indeterminate);
    }

    /// A PID astronomically unlikely to name a real process on a test runner -- real Windows PIDs
    /// stay far below this range in practice, the same "unlikely" reasoning
    /// `unlock/tests.rs::a_lock_recording_a_nonexistent_pid_is_reported_as_not_appearing_to_run` uses
    /// for its own `999999` on Linux/macOS. `OpenProcess` is expected to fail with
    /// `ERROR_INVALID_PARAMETER` for it -- demonstrated by running this, not assumed from the API
    /// contract (RFC criterion 1).
    #[test]
    fn process_liveness_of_an_implausible_pid_is_does_not_exist() {
        assert_eq!(process_liveness(0x7FFF_FFFF), ProcessLiveness::DoesNotExist);
    }

    /// RFC criterion 3 / design-v1.md: an honest, Windows-reachable access-denied test, not one
    /// asserted only by reading the API contract. PID 4 is the Windows System process by
    /// long-standing OS convention, and `OpenProcess`'s own reference page states explicitly that
    /// opening it "fails and the last error code is ERROR_ACCESS_DENIED because their access
    /// restrictions prevent user-level code from opening them" -- regardless of which access right is
    /// requested, unlike an ordinary process where a narrower right (`PROCESS_QUERY_LIMITED_INFORMATION`)
    /// can succeed where a broader one fails.
    #[test]
    fn process_liveness_of_the_system_process_is_exists() {
        assert_eq!(process_liveness(4), ProcessLiveness::Exists);
    }

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
