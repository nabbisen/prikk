//! `prikk setup` — one command over a first-class sequence (RFC 135 §3/§9.8.2).
//!
//! `key generate`, `trust maintainer add`, and `init` stay first-class and documented — that is
//! the sequence a reader follows to *understand* what this command does. `setup` composes them
//! for someone who wants a working repository now, without weakening any of the five binding
//! properties the individual commands already establish:
//!
//! 1. One command reaches a working repository.
//! 2. prikk invents no location for a secret -- the user names every output path.
//! 3. No secret reaches scrollback when the user provides an output path for it.
//! 4. The trust decision is shown -- registering a maintainer key is a trust act, and this
//!    composition may remove the *typing*, never the *seeing*.
//! 5. Seeds never on argv. Paths are fine.

use std::path::PathBuf;

use prikk_store::{RepositoryLayout, add_trusted_maintainer};

use crate::arg_scan::{SetOnce, flag_value, unknown_argument};
use crate::commands::CliError;
use crate::key::write_seed_to_path;
use crate::stdout::println;
use prikk_crypto::Ed25519KeyPair;

const AUTHOR_KEY_ID: &str = "author";
const MAINTAINER_KEY_ID: &str = "maintainer";

/// One generated seed's disposition: printed (with its hex, for the export block) or written to a
/// user-named path (never printed).
enum SeedOutput {
    Printed(String),
    WrittenTo(PathBuf),
}

pub fn run_setup(args: Vec<String>) -> std::result::Result<(), CliError> {
    let mut path = None;
    let mut author_seed_out = None;
    let mut maintainer_seed_out = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--author-seed-out" => {
                let value = flag_value(&mut iter, "setup --author-seed-out")?;
                author_seed_out.set_once("--author-seed-out", PathBuf::from(value))?;
            }
            "--maintainer-seed-out" => {
                let value = flag_value(&mut iter, "setup --maintainer-seed-out")?;
                maintainer_seed_out.set_once("--maintainer-seed-out", PathBuf::from(value))?;
            }
            other if other.starts_with('-') => return Err(unknown_argument("setup", other)),
            _ => {
                if path.is_some() {
                    return Err(CliError::Usage(
                        "setup accepts at most one repository path".to_string(),
                    ));
                }
                path = Some(arg);
            }
        }
    }
    let root = match path {
        Some(path) => PathBuf::from(path),
        None => crate::args::current_dir()?,
    };

    // Property 1: one command reaches a working repository, without the user running anything
    // else first -- `RepositoryLayout::init` itself does not create a missing leading directory
    // (the same is true of plain `prikk init`), so `setup` must, or naming a path that does not
    // yet exist would silently reintroduce a step this command exists to remove.
    std::fs::create_dir_all(&root)
        .map_err(|err| format!("failed to create {}: {err}", root.display()))?;

    let layout = RepositoryLayout::init(root.clone()).map_err(|err| err.to_string())?;
    println!(
        "initialized Prikk repository at {}",
        root.join(".prikk").display()
    );

    let author_seed = Ed25519KeyPair::generate_seed().map_err(|err| err.to_string())?;
    let author_output = match author_seed_out {
        Some(path) => {
            write_seed_to_path(&author_seed, &path)?;
            SeedOutput::WrittenTo(path)
        }
        None => SeedOutput::Printed(prikk_hash::to_hex(&author_seed)),
    };

    let maintainer_seed = Ed25519KeyPair::generate_seed().map_err(|err| err.to_string())?;
    let maintainer_public_key = Ed25519KeyPair::from_seed(&maintainer_seed).public_key_bytes();
    let maintainer_public_key_hex = prikk_hash::to_hex(&maintainer_public_key);
    let maintainer_output = match maintainer_seed_out {
        Some(path) => {
            write_seed_to_path(&maintainer_seed, &path)?;
            SeedOutput::WrittenTo(path)
        }
        None => SeedOutput::Printed(prikk_hash::to_hex(&maintainer_seed)),
    };

    // Property 4: the trust decision is shown, not performed invisibly -- this is the one step in
    // the composed sequence that is a trust act, and `setup` must print it exactly as `trust
    // maintainer add` itself would, not fold it silently into "repository ready."
    let (adopted, _newly_added) =
        add_trusted_maintainer(&layout, MAINTAINER_KEY_ID, &maintainer_public_key_hex)
            .map_err(|err| err.to_string())?;
    println!("trusted maintainer key: {}", adopted.key_id);
    println!("policy: required=1");

    let any_printed = matches!(author_output, SeedOutput::Printed(_))
        || matches!(maintainer_output, SeedOutput::Printed(_));

    println!();
    println!("export these before committing:");
    println!("  export PRIKK_AUTHOR_KEY_ID=\"{AUTHOR_KEY_ID}\"");
    match &author_output {
        SeedOutput::Printed(hex) => println!("  export PRIKK_AUTHOR_SEED=\"{hex}\""),
        SeedOutput::WrittenTo(path) => {
            println!("  export PRIKK_AUTHOR_SEED=\"$(cat {})\"", path.display());
        }
    }
    println!("  export PRIKK_MAINTAINER_KEY_ID=\"{MAINTAINER_KEY_ID}\"");
    match &maintainer_output {
        SeedOutput::Printed(hex) => println!("  export PRIKK_MAINTAINER_SEED=\"{hex}\""),
        SeedOutput::WrittenTo(path) => {
            println!(
                "  export PRIKK_MAINTAINER_SEED=\"$(cat {})\"",
                path.display()
            );
        }
    }
    if any_printed {
        println!(
            "note: at least one seed above is now in your terminal scrollback -- treat it as a \
             secret"
        );
    }
    println!();
    println!("next steps:");
    println!("  prikk commit -m \"<message>\"");
    println!(
        "  prikk seal --allow-no-audit  # no audit trust policy is configured yet; see \
         `prikk seal --help`"
    );
    Ok(())
}
