//! `prikk key` — generate a fresh Ed25519 seed, or derive a public key from one already held
//! (RFC 135 §2). Neither subcommand needs an open repository: a visitor must be able to generate
//! a key *before* `init`.
//!
//! **The seed is never accepted on argv (RFC 135 §9.3, a ruling, not a preference).**
//! `/proc/<pid>/cmdline` is world-readable on Linux and shells record argv in history; a
//! `--seed <hex>` flag would leak key material to every process on the machine. `key public` reads
//! the seed from a *named* environment variable instead — the name is not the secret.

use std::path::{Path, PathBuf};

use crate::arg_scan::{SetOnce, flag_value, unknown_argument};
use crate::commands::CliError;
use crate::stdout::println;
use prikk_crypto::Ed25519KeyPair;

/// Dispatch `prikk key [generate|public]`.
pub fn run_key(args: Vec<String>) -> std::result::Result<(), CliError> {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some("generate") => run_generate(iter.collect()),
        Some("public") => run_public(iter.collect()),
        Some(other) => Err(CliError::Usage(format!(
            "unknown key subcommand: {other} (expected generate or public)"
        ))),
        None => Err(CliError::Usage(
            "usage: prikk key generate [--out <path>]\n       \
             prikk key public --seed-env <NAME>"
                .to_string(),
        )),
    }
}

fn run_generate(args: Vec<String>) -> std::result::Result<(), CliError> {
    let mut out = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" => {
                let value = flag_value(&mut iter, "key generate --out")?;
                out.set_once("--out", PathBuf::from(value))?;
            }
            other => return Err(unknown_argument("key generate", other)),
        }
    }

    let seed = Ed25519KeyPair::generate_seed().map_err(|err| err.to_string())?;
    let public_key = Ed25519KeyPair::from_seed(&seed).public_key_bytes();
    let public_key_hex = prikk_hash::to_hex(&public_key);

    match out {
        Some(path) => {
            write_seed_to_path(&seed, &path)?;
            println!("wrote seed to {} (mode 0600)", path.display());
            println!("public key: {public_key_hex}");
            println!();
            println!("next steps:");
            println!(
                "  prikk trust maintainer add --key-id maintainer --public-key {public_key_hex}"
            );
            println!("  export PRIKK_MAINTAINER_KEY_ID=\"maintainer\"");
            println!(
                "  export PRIKK_MAINTAINER_SEED=\"$(cat {})\"",
                path.display()
            );
            println!(
                "note: the same seed works as an AUTHOR key instead -- export \
                 PRIKK_AUTHOR_KEY_ID/PRIKK_AUTHOR_SEED and skip the trust step"
            );
        }
        None => {
            let seed_hex = prikk_hash::to_hex(&seed);
            println!("seed: {seed_hex}");
            println!("note: this seed is now in your terminal scrollback -- treat it as a secret");
            println!("public key: {public_key_hex}");
            println!();
            println!("next steps:");
            println!(
                "  prikk trust maintainer add --key-id maintainer --public-key {public_key_hex}"
            );
            println!("  export PRIKK_MAINTAINER_KEY_ID=\"maintainer\"");
            println!("  export PRIKK_MAINTAINER_SEED=\"{seed_hex}\"");
            println!(
                "note: the same seed works as an AUTHOR key instead -- export \
                 PRIKK_AUTHOR_KEY_ID/PRIKK_AUTHOR_SEED and skip the trust step"
            );
        }
    }
    Ok(())
}

fn run_public(args: Vec<String>) -> std::result::Result<(), CliError> {
    let mut seed_env = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--seed-env" => {
                let value = flag_value(&mut iter, "key public --seed-env")?;
                seed_env.set_once("--seed-env", value)?;
            }
            other => return Err(unknown_argument("key public", other)),
        }
    }
    let seed_env = seed_env
        .ok_or_else(|| CliError::Usage("key public requires --seed-env <NAME>".to_string()))?;
    let seed = crate::read_seed_env(&seed_env)?;
    let public_key = Ed25519KeyPair::from_seed(&seed).public_key_bytes();
    println!("public key: {}", prikk_hash::to_hex(&public_key));
    Ok(())
}

/// Write `seed` to `path`: mode `0600`, refuses to overwrite an existing file, refuses any path
/// with a `.prikk` component (RFC 135 §9.2 -- prikk never invents a secret's location and never
/// manages its lifecycle, so it must not write one where it might later mistake the file for its
/// own). Shared with `prikk setup`, which writes seeds the same way.
///
/// **Windows default ruling (RFC 135 §2.1): refused.** `std::os::unix::fs::PermissionsExt` is
/// Unix-only, and an ACL-based equivalent needs Win32 FFI -- `#![forbid(unsafe_code)]` (this
/// crate) plus DC-90 (unsafe is a reviewed exception, not an import) make that its own decision.
/// Writing a secret at inherited permissions and saying nothing is not acceptable; refusing
/// outright and pointing at the print-and-place path is.
pub(crate) fn write_seed_to_path(
    seed: &[u8; prikk_crypto::ED25519_KEY_LEN],
    path: &Path,
) -> std::result::Result<(), CliError> {
    if path.components().any(|c| c.as_os_str() == ".prikk") {
        return Err(CliError::Usage(
            "the seed output path must not be inside .prikk/ -- prikk never manages a secret's \
             lifecycle"
                .to_string(),
        ));
    }
    write_seed_to_path_platform(seed, path)
}

#[cfg(windows)]
fn write_seed_to_path_platform(
    _seed: &[u8; prikk_crypto::ED25519_KEY_LEN],
    _path: &Path,
) -> std::result::Result<(), CliError> {
    Err(CliError::Failure(
        "writing a seed to a file is not yet supported on Windows -- Unix file permissions \
         (mode 0600) have no portable equivalent here without unsafe code or a new dependency, \
         and this project refuses to write a secret at inherited permissions silently. Run \
         `prikk key generate` without --out, then save the printed seed yourself."
            .to_string(),
    ))
}

#[cfg(unix)]
fn write_seed_to_path_platform(
    seed: &[u8; prikk_crypto::ED25519_KEY_LEN],
    path: &Path,
) -> std::result::Result<(), CliError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::AlreadyExists {
                CliError::Failure(format!(
                    "refusing to overwrite an existing file: {}",
                    path.display()
                ))
            } else {
                CliError::Failure(format!("failed to create {}: {err}", path.display()))
            }
        })?;
    let seed_hex = prikk_hash::to_hex(seed);
    file.write_all(seed_hex.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|err| CliError::Failure(format!("failed to write {}: {err}", path.display())))?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn write_seed_to_path_platform(
    _seed: &[u8; prikk_crypto::ED25519_KEY_LEN],
    _path: &Path,
) -> std::result::Result<(), CliError> {
    Err(CliError::Failure(
        "writing a seed to a file is not supported on this platform -- run `prikk key generate` \
         without --out, then save the printed seed yourself."
            .to_string(),
    ))
}
