//! Immutable no-clobber file publication.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use prikk_error::{PrikkError, Result};

use super::directory::{MutationRoot, prepare_directory_required};
use super::regular::{
    open_existing_regular_if_exists, open_new_regular, required_file_name, required_parent,
};
use super::{failpoints, io_error};
use crate::fsutil::temporary_path;

#[cfg(target_os = "linux")]
use rustix::fs::{self, AtFlags, OFlags};

/// Publish immutable bytes without replacing an existing final entry.
pub(crate) fn publish_immutable_file(
    root: &MutationRoot,
    relative: &Path,
    candidate: &[u8],
    validate_existing: impl Fn(&[u8]) -> Result<()>,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let parent = required_parent(relative)?;
        let directory = prepare_directory_required(root, parent)?;
        let destination = required_file_name(relative)?;
        if compare_existing(&directory, destination, candidate, &validate_existing)? {
            failpoints::immutable_cleanup_sync()?;
            directory.sync()?;
            return Ok(());
        }

        let temp_path = temporary_path(relative)?;
        let temp_name = required_file_name(&temp_path)?;
        let fd = open_new_regular(&directory.fd, temp_name).map_err(io_error)?;
        let mut temp = File::from(fd);
        temp.write_all(candidate)?;
        failpoints::immutable_file_sync()?;
        temp.sync_all()?;
        drop(temp);

        failpoints::wait_at_immutable_install();
        failpoints::immutable_install()?;
        let install = if let Some(error) = failpoints::immutable_install_error() {
            Err(error)
        } else {
            fs::linkat(
                &directory.fd,
                temp_name,
                &directory.fd,
                destination,
                AtFlags::empty(),
            )
        };
        match install {
            Ok(()) => {
                failpoints::immutable_install_sync()?;
                directory.sync()?;
            }
            Err(rustix::io::Errno::EXIST) => {
                if !compare_existing(&directory, destination, candidate, &validate_existing)? {
                    return Err(PrikkError::Integrity(
                        "no-clobber install reported an absent winner".to_string(),
                    ));
                }
            }
            Err(error) => return Err(classify_install_error(error)),
        }

        failpoints::immutable_temp_unlink()?;
        fs::unlinkat(&directory.fd, temp_name, AtFlags::empty()).map_err(io_error)?;
        failpoints::immutable_cleanup_sync()?;
        directory.sync()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, candidate, validate_existing);
        Err(PrikkError::Io(
            "immutable no-clobber publication is unsupported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "linux")]
fn classify_install_error(error: rustix::io::Errno) -> PrikkError {
    match error {
        rustix::io::Errno::OPNOTSUPP | rustix::io::Errno::NOSYS | rustix::io::Errno::PERM => {
            PrikkError::Io(format!(
                "immutable no-clobber install is unsupported by filesystem or policy: {error}"
            ))
        }
        _ => io_error(error),
    }
}

#[cfg(target_os = "linux")]
fn compare_existing(
    directory: &super::directory::AnchoredDirectory,
    destination: &std::ffi::OsStr,
    candidate: &[u8],
    validate_existing: &impl Fn(&[u8]) -> Result<()>,
) -> Result<bool> {
    let Some(fd) = open_existing_regular_if_exists(&directory.fd, destination, OFlags::RDONLY)?
    else {
        return Ok(false);
    };
    let mut bytes = Vec::new();
    File::from(fd).read_to_end(&mut bytes)?;
    validate_existing(&bytes)?;
    if bytes != candidate {
        return Err(PrikkError::Integrity(
            "existing immutable object bytes differ from candidate".to_string(),
        ));
    }
    Ok(true)
}
