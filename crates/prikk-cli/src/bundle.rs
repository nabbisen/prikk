//! `prikk bundle` — export/import a verifiable subset of history (DC-78 §D4/§D6).
//!
//! `bundle export` writes a self-contained file: the exported ref's own RefState plus every object
//! reachable from its target Block back to genesis. `bundle import` writes those objects into the
//! local object store and records a `received` pointer (`remotes/<origin ref name>`) — it never
//! touches `refs/by-id/`, never advances a local ref, and never adopts a MAINTAINER key into the
//! local trust policy. Imported history stays present but untrusted until the operator explicitly
//! runs `trust maintainer add` for the key that sealed it; the way to gain confidence in what was
//! imported is an ordinary `prikk verify`, unmodified — the bundle format adds no new verification
//! path. Turning a received ref into local history is an ordinary `merge`, using machinery that
//! already exists; this module does not add a "pull" concept.

use std::path::PathBuf;

use prikk_store::{
    BundleImportOptions, DEFAULT_BUNDLE_MAX_OBJECT_COUNT, DEFAULT_BUNDLE_MAX_TOTAL_BYTES,
    export_bundle, import_bundle,
};

/// Dispatch `prikk bundle [export|import]`.
pub fn run_bundle(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some("export") => run_export(root, iter.collect()),
        Some("import") => run_import(root, iter.collect()),
        Some(other) => Err(format!(
            "unknown bundle subcommand: {other} (expected export or import)"
        )),
        None => Err("bundle requires a subcommand: export or import".to_string()),
    }
}

fn run_export(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let parsed = parse_export_args(args)?;
    let layout = crate::open_repository(root)?;
    let (report, bytes) =
        export_bundle(&layout, &parsed.ref_name).map_err(|err| err.to_string())?;
    std::fs::write(&parsed.output, &bytes).map_err(|err| {
        format!(
            "failed to write bundle to {}: {err}",
            parsed.output.display()
        )
    })?;
    println!("exported {}", report.ref_name);
    println!("tip block: {}", report.tip_block_id);
    println!("objects: {}", report.object_count);
    println!(
        "author key material: {} included (continuity only, not a trust decision)",
        report.author_key_count
    );
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
}

fn parse_export_args(args: Vec<String>) -> std::result::Result<ExportArgs, String> {
    let mut ref_name = None;
    let mut output = None;
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
            other => return Err(format!("unknown bundle export argument: {other}")),
        }
    }
    let ref_name = ref_name.ok_or_else(|| "bundle export requires --ref".to_string())?;
    let output = output.ok_or_else(|| "bundle export requires --output".to_string())?;
    Ok(ExportArgs { ref_name, output })
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
