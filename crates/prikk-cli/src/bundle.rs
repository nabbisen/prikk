//! `prikk bundle` — export/import/verify a verifiable subset of history (DC-78 §D4/§D6; `verify`
//! is DC-44 increment 1, `bundle-offline-verify-handoff-v1.md`).
//!
//! `bundle export` writes a self-contained file: the exported ref's own RefState plus every object
//! reachable from its target Block back to genesis. **Refuses an existing file at the destination
//! unless `--force` is passed** (DC-44 increment 2,
//! `bundle-export-durability-handoff-v1.md` §3.1 — destroying a backup must never be silent), and
//! writes atomically and durably (§3.2 — `crate::durable_output`, not the anchored contract, which
//! is confined to repository-internal paths and does not apply to an arbitrary destination). `bundle
//! import` writes those objects into the
//! local object store and records a `received` pointer (`remotes/<origin ref name>`) — it never
//! touches `refs/by-id/`, never advances a local ref, and never adopts a MAINTAINER key into the
//! local trust policy. Imported history stays present but untrusted until the operator explicitly
//! runs `trust maintainer add` for the key that sealed it; the way to gain confidence in what was
//! imported is an ordinary `prikk verify`, unmodified — the bundle format adds no new verification
//! path. Turning a received ref into local history is an ordinary `merge`, using machinery that
//! already exists; this module does not add a "pull" concept.
//!
//! `bundle verify` answers "is this backup any good?" without restoring it: it reads a bundle file
//! and reports whether it is structurally sound and internally consistent, writing nothing and
//! needing no repository — `run_verify` below never calls `crate::open_repository`, unlike
//! `run_export`/`run_import`. It shares `import`'s own decode and closure-validation path
//! (`prikk_store::verify_bundle`, DC-44 increment 1 §2) rather than a second decoder, so the two
//! cannot silently drift apart on what counts as well-formed.
//!
//! **`bundle export` and `bundle verify` both print the bundle's self-describing manifest**
//! (DC-44 increment 3, `bundle-manifest-handoff-v1.md`): the on-disk repository format and tool
//! version that produced it, and a fixed note stating this bundle is one ref's closure only --
//! other refs in the source repository, if any, are not included, and the bundle makes no claim
//! about them. A `PBNDL001`/`PBNDL002` bundle predates the manifest section, so `verify` says so
//! plainly rather than printing absent fields as if they were checked and found empty.

use std::path::PathBuf;

use prikk_store::{
    BundleImportOptions, BundleManifest, BundleScope, DEFAULT_BUNDLE_MAX_OBJECT_COUNT,
    DEFAULT_BUNDLE_MAX_TOTAL_BYTES, export_bundle, import_bundle, verify_bundle,
};

/// Dispatch `prikk bundle [export|import|verify]`.
pub fn run_bundle(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some("export") => run_export(root, iter.collect()),
        Some("import") => run_import(root, iter.collect()),
        Some("verify") => run_verify(iter.collect()),
        Some(other) => Err(format!(
            "unknown bundle subcommand: {other} (expected export, import, or verify)"
        )),
        None => Err("bundle requires a subcommand: export, import, or verify".to_string()),
    }
}

fn run_export(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_export_args(args)?;
    // DC-44 increment 2 §3.1: checked before opening the repository at all, so the common
    // "there is already a backup at this path" case fails fast rather than after a read. Not
    // race-free against a file created after this check -- see `durable_output`'s own doc
    // comment for why that is an accepted, stated limit rather than a gap this pretends to close.
    if !parsed.force && crate::durable_output::destination_exists(&parsed.output) {
        return Err(format!(
            "refusing to overwrite existing file at {} (pass --force to overwrite it \
             intentionally)",
            parsed.output.display()
        ));
    }
    let layout = crate::open_repository(root)?;
    let (report, bytes) =
        export_bundle(&layout, &parsed.ref_name).map_err(|err| err.to_string())?;
    // DC-44 increment 2 §3.2: atomic and durable -- a failure here leaves whatever was previously
    // at `parsed.output` untouched, never a partial file. See `durable_output`'s own doc comment
    // for exactly what this does and does not guarantee.
    crate::durable_output::write_new_file_durably(&parsed.output, &bytes)?;
    println!("exported {}", report.ref_name);
    println!("tip block: {}", report.tip_block_id);
    println!("objects: {}", report.object_count);
    println!(
        "author key material: {} included (continuity only, not a trust decision)",
        report.author_key_count
    );
    print_manifest(&report.manifest);
    println!("wrote {}", parsed.output.display());
    Ok(())
}

fn run_import(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_import_args(args)?;
    let layout = crate::open_repository(root)?;
    let bytes = std::fs::read(&parsed.input).map_err(|err| {
        format!(
            "failed to read bundle from {}: {err}",
            parsed.input.display()
        )
    })?;
    let options = bundle_import_options_from_env()?;
    let report = import_bundle(&layout, &bytes, &options).map_err(|err| err.to_string())?;
    println!("received {}", report.ref_name);
    println!("RefState: {}", report.ref_state_id);
    println!("objects: {}", report.object_count);
    println!("new objects: {}", report.written_object_count);
    println!(
        "author key material: {} recorded (continuity only, not a trust decision)",
        report.recorded_author_key_count
    );
    println!(
        "note: no local ref was created or advanced, and no MAINTAINER key was trusted; run \
         `trust maintainer add` to trust the sealing key, then `merge` to incorporate this history"
    );
    Ok(())
}

fn run_verify(args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_verify_args(args)?;
    let bytes = std::fs::read(&parsed.input).map_err(|err| {
        format!(
            "failed to read bundle from {}: {err}",
            parsed.input.display()
        )
    })?;
    let options = bundle_import_options_from_env()?;
    let report = verify_bundle(&bytes, &options).map_err(|err| err.to_string())?;
    println!("bundle verifies: {}", report.ref_name);
    println!("RefState: {}", report.ref_state_id);
    println!("tip block: {}", report.tip_block_id);
    println!("objects: {}", report.object_count);
    println!(
        "author key material: {} present (continuity only, not a trust decision)",
        report.author_key_count
    );
    match &report.manifest {
        Some(manifest) => print_manifest(manifest),
        None => println!(
            "manifest: not present (this bundle predates the PBNDL003 manifest section -- \
             repository format, tool version, and scope are unknown)"
        ),
    }
    println!(
        "note: this checks structural and internal consistency only -- no signature is \
         cryptographically verified (a standalone bundle carries no trust material to check one \
         against), and this bundle's own author-key section is recorded here, never \
         independently verified, the same as at import. A verified bundle is not yet a trusted \
         one -- import it and run `prikk verify` for that."
    );
    Ok(())
}

/// DC-44 increment 3 §4.3: printed by both `bundle export` and `bundle verify` for a
/// manifest-bearing (`PBNDL003`) bundle. `BundleScope` has exactly one variant today, so the
/// `match` prints one fixed sentence -- kept as a match, not an `if let`, so a future second
/// variant forces this print site to be revisited rather than silently keep the old wording.
fn print_manifest(manifest: &BundleManifest) {
    println!("repository format: {}", manifest.repository_format);
    println!("tool version: {}", manifest.tool_version);
    match manifest.scope {
        BundleScope::SingleRef => println!(
            "note: this bundle contains one ref's closure only -- other refs in the source \
             repository, if any, are not included, and this bundle makes no claim about them"
        ),
    }
}

struct VerifyArgs {
    input: PathBuf,
}

fn parse_verify_args(args: Vec<String>) -> std::result::Result<VerifyArgs, String> {
    let mut input = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                let Some(value) = iter.next() else {
                    return Err("bundle verify --input requires a value".to_string());
                };
                input = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown bundle verify argument: {other}")),
        }
    }
    let input = input.ok_or_else(|| "bundle verify requires --input".to_string())?;
    Ok(VerifyArgs { input })
}

/// DC-86: `BundleImportOptions` from `PRIKK_BUNDLE_MAX_OBJECTS`/`PRIKK_BUNDLE_MAX_BYTES`, in
/// `ActivePatchThresholds::from_env`'s exact shape (DC-57) — absent means the documented default;
/// present but non-numeric or zero is a hard error, never a silent fallback to the default.
fn bundle_import_options_from_env() -> std::result::Result<BundleImportOptions, String> {
    let max_object_count =
        parse_bundle_limit_env("PRIKK_BUNDLE_MAX_OBJECTS", DEFAULT_BUNDLE_MAX_OBJECT_COUNT)?;
    let max_total_bytes =
        parse_bundle_limit_env("PRIKK_BUNDLE_MAX_BYTES", DEFAULT_BUNDLE_MAX_TOTAL_BYTES)?;
    Ok(BundleImportOptions::default_limits()
        .with_max_object_count(max_object_count)
        .with_max_total_bytes(max_total_bytes))
}

fn parse_bundle_limit_env(name: &str, default: usize) -> std::result::Result<usize, String> {
    let Ok(raw) = std::env::var(name) else {
        return Ok(default);
    };
    let trimmed = raw.trim();
    let value: usize = trimmed
        .parse()
        .map_err(|_| format!("{name} must be a positive integer, got {raw:?}"))?;
    if value == 0 {
        return Err(format!("{name} must be greater than zero, got 0"));
    }
    Ok(value)
}

struct ExportArgs {
    ref_name: String,
    output: PathBuf,
    force: bool,
}

fn parse_export_args(args: Vec<String>) -> std::result::Result<ExportArgs, String> {
    let mut ref_name = None;
    let mut output = None;
    let mut force = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("bundle export --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("bundle export --ref must not be empty".to_string());
                }
                ref_name = Some(value);
            }
            "--output" => {
                let Some(value) = iter.next() else {
                    return Err("bundle export --output requires a value".to_string());
                };
                output = Some(PathBuf::from(value));
            }
            "--force" => force = true,
            other => return Err(format!("unknown bundle export argument: {other}")),
        }
    }
    let ref_name = ref_name.ok_or_else(|| "bundle export requires --ref".to_string())?;
    let output = output.ok_or_else(|| "bundle export requires --output".to_string())?;
    Ok(ExportArgs {
        ref_name,
        output,
        force,
    })
}

struct ImportArgs {
    input: PathBuf,
}

fn parse_import_args(args: Vec<String>) -> std::result::Result<ImportArgs, String> {
    let mut input = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                let Some(value) = iter.next() else {
                    return Err("bundle import --input requires a value".to_string());
                };
                input = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown bundle import argument: {other}")),
        }
    }
    let input = input.ok_or_else(|| "bundle import requires --input".to_string())?;
    Ok(ImportArgs { input })
}
