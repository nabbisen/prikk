use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::{BoundaryError, PRODUCTS, push};
use crate::error::{Error, Result};
use crate::json;

/// Published-crate-posture handoff v1 §5: never correct in a published crate's description.
/// A wordlist ages badly in general, but these are never right in a released manifest --
/// `scaffold`/`placeholder`/`TODO`/`WIP` are exactly the provisional language that let
/// `prikk`/`prikk-store` describe themselves as scaffolding for two releases after they stopped
/// being one. Case-insensitive substring match: descriptions here are short and hand-written, not
/// user-generated text, so the false-positive surface is negligible for these four -- deliberately
/// substring, not whole-word, so it still catches `scaffolding` and not just `scaffold`.
const PROVISIONAL_SUBSTRINGS: [&str; 4] = ["scaffold", "placeholder", "todo", "wip"];

/// `initial` is checked separately, as a whole word, not a substring (review of this handoff's
/// v1 report): unlike the four above, `initial` is a common English prefix in exactly the
/// vocabulary this project's own crates legitimately use -- "repository initialization",
/// "initialize the WAL" -- and a substring match rejects those alongside the real defect. A
/// whole-word match still catches the actual origin case (`"Prikk CLI initial scaffold."`, where
/// `scaffold` alone already flags it) while leaving `initialize`/`initialization` alone.
const PROVISIONAL_WHOLE_WORDS: [&str; 1] = ["initial"];

pub(super) fn check(root: &Path, errors: &mut Vec<BoundaryError>) -> Result<()> {
    for (package, _) in PRODUCTS {
        let output = Command::new("cargo")
            .args([
                "package",
                "--locked",
                "--allow-dirty",
                "--list",
                "-p",
                package,
            ])
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            push(
                errors,
                "package-contents",
                format!(
                    "{package}: cargo package failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            continue;
        }
        for path in String::from_utf8_lossy(&output.stdout).lines() {
            if path.starts_with("release/oracle/") || path.starts_with("tools/release-policy/") {
                push(errors, "package-contents", format!("{package}:{path}"));
            }
        }
    }
    check_source_tree(root, errors);
    check_descriptions(root, errors);
    check_readmes(root, errors);
    Ok(())
}

fn check_descriptions(root: &Path, errors: &mut Vec<BoundaryError>) {
    for (crate_name, manifest_path) in PRODUCTS {
        let Ok(text) = fs::read_to_string(root.join(manifest_path)) else {
            push(
                errors,
                "package-description",
                format!("{crate_name}: manifest unreadable"),
            );
            continue;
        };
        let Ok(manifest) = toml::from_str::<toml::Value>(&text) else {
            push(
                errors,
                "package-description",
                format!("{crate_name}: manifest unparseable"),
            );
            continue;
        };
        let description = manifest
            .get("package")
            .and_then(|package| package.get("description"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        if description.trim().is_empty() {
            push(
                errors,
                "package-description",
                format!("{crate_name}: missing description"),
            );
            continue;
        }
        for word in provisional_words_in(description) {
            push(
                errors,
                "package-description",
                format!("{crate_name}: provisional word {word:?}"),
            );
        }
    }
}

/// Crate-README-currency handoff v1 §4: extends the same wordlist to each published crate's
/// `readme` target, and §1's actual rule -- a README must not restate its own description -- made
/// mechanical rather than editorial. Covers all eight `PRODUCTS`, `prikk` included: its `readme`
/// points at the workspace root `README.md`, a much longer document, but a scan of it today finds
/// none of the five words, so there is no false-positive cost to including it (the trigger the
/// handoff named for scoping this down to seven).
fn check_readmes(root: &Path, errors: &mut Vec<BoundaryError>) {
    for (crate_name, manifest_path) in PRODUCTS {
        let Ok(manifest_text) = fs::read_to_string(root.join(manifest_path)) else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: manifest unreadable"),
            );
            continue;
        };
        let Ok(manifest) = toml::from_str::<toml::Value>(&manifest_text) else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: manifest unparseable"),
            );
            continue;
        };
        let Some(package) = manifest.get("package") else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: manifest has no [package] table"),
            );
            continue;
        };
        let Some(readme_field) = package.get("readme").and_then(toml::Value::as_str) else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: manifest has no readme field"),
            );
            continue;
        };
        let Some(manifest_dir) = root.join(manifest_path).parent().map(Path::to_path_buf) else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: manifest path has no parent"),
            );
            continue;
        };
        let Ok(readme_text) = fs::read_to_string(manifest_dir.join(readme_field)) else {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: readme unreadable ({readme_field})"),
            );
            continue;
        };
        for word in provisional_words_in(&readme_text) {
            push(
                errors,
                "package-readme",
                format!("{crate_name}: provisional word {word:?} in readme"),
            );
        }
        // `prikk`'s `readme` is the workspace root `README.md`, not a crate-local file -- and its
        // description was deliberately *sourced from* that document's own opening sentence (this
        // handoff's review named that a technique worth reusing precisely because the description
        // cannot overclaim relative to the README). That is the opposite direction from the seven
        // library crates' defect (a lazy three-line README copying the description), so the
        // duplication check below does not apply to it.
        if crate_name == "prikk" {
            continue;
        }
        let description = package
            .get("description")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let readme_flat = readme_text.split_whitespace().collect::<Vec<_>>().join(" ");
        for sentence in description_sentences(description) {
            if readme_flat.to_lowercase().contains(&sentence) {
                push(
                    errors,
                    "package-readme-duplication",
                    format!("{crate_name}: readme repeats description sentence {sentence:?}"),
                );
            }
        }
    }
}

/// Case-insensitive words/substrings this check flags in either surface, matched against `text`.
fn provisional_words_in(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut hits: Vec<&'static str> = PROVISIONAL_SUBSTRINGS
        .into_iter()
        .filter(|word| lower.contains(word))
        .collect();
    hits.extend(
        PROVISIONAL_WHOLE_WORDS
            .into_iter()
            .filter(|word| contains_word(&lower, word)),
    );
    hits
}

/// Whether `word` appears in `haystack` as a standalone token -- split on non-alphanumeric
/// boundaries, not a raw substring search. `haystack` must already be lowercased; `word` is
/// compared as given.
fn contains_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|character: char| !character.is_alphanumeric())
        .any(|token| token == word)
}

/// A description's sentences, normalized for a literal-substring duplication check against a
/// README: split on `". "`, lowercased, trailing period trimmed, trivially short fragments (e.g.
/// stray semicolon clauses) dropped so they cannot produce a spurious match.
fn description_sentences(description: &str) -> Vec<String> {
    description
        .split(". ")
        .map(|sentence| sentence.trim().trim_end_matches('.').to_lowercase())
        .filter(|sentence| sentence.len() >= 15)
        .collect()
}

fn check_source_tree(root: &Path, errors: &mut Vec<BoundaryError>) {
    for path in [
        "tools/release-policy/Cargo.toml",
        "tools/release-policy/src/main.rs",
        "tools/release-policy/self-test-responsibility-map-v1.json",
        "release/schemas/release-evidence-v1.schema.json",
        "release/oracle/oracle-manifest-v1.json",
        "release/oracle/oracle-manifest-v1.schema.json",
        "release/oracle/coverage-inventory-v1.json",
        "release/oracle/python-observations-v1.json",
        "release/oracle/reason-map-v1.json",
        "release/release-policy-command-inventory-v1.json",
        "release/publication-command-inventory-v1.json",
        "release/oracle/packs/release-evidence-v1.json",
        // RFC 119 track A: parked, not deleted -- moved out of the active pack registry when
        // every case referencing it was parked (release/oracle/parked-cases-v1.json).
        "release/oracle/parked-packs/signer-challenge-v1.json",
        // RFC 119 track B: parked, not deleted -- 16 of release-evidence's 73 cases were parked
        // (the ones exercising the embedded DC-35 signer-governance sub-object), orphaning these
        // 36 entries (release/oracle/parked-cases-v1.json's second batch). release-state's own
        // pack is absent from this list entirely: that suite was removed outright (NEVER, not
        // LATER), so its pack file was deleted, not parked.
        "release/oracle/parked-packs/release-evidence-governance-v1.json",
    ] {
        if !root.join(path).is_file() {
            push(errors, "source-archive-contents", path.to_owned());
        }
    }
    let manifest_path = root.join("release/oracle/oracle-manifest-v1.json");
    let manifest = fs::read(&manifest_path)
        .map_err(Error::from)
        .and_then(|bytes| {
            json::parse(&bytes)
                .map_err(|error| Error::new(format!("source manifest JSON: {error}")))
        });
    match manifest {
        Ok(manifest) => check_direct_inputs(root, &manifest, errors),
        Err(error) => push(
            errors,
            "source-archive-contents",
            format!("oracle manifest: {error}"),
        ),
    }
}

fn check_direct_inputs(root: &Path, manifest: &Value, errors: &mut Vec<BoundaryError>) {
    let Some(cases) = manifest.get("cases").and_then(Value::as_array) else {
        push(
            errors,
            "source-archive-contents",
            "oracle manifest cases".to_owned(),
        );
        return;
    };
    for path in cases
        .iter()
        .filter_map(|case| case.get("inputs").and_then(Value::as_array))
        .flatten()
        .filter_map(|input| input.get("location"))
        .filter(|location| location.get("kind").and_then(Value::as_str) == Some("direct"))
        .filter_map(|location| location.get("path").and_then(Value::as_str))
    {
        if !root.join(path).is_file() {
            push(errors, "source-archive-contents", path.to_owned());
        }
    }
}

#[cfg(test)]
#[path = "package/tests.rs"]
mod tests;
