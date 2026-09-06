//! The profile extractor (RFC 139 handoff §2): a pure text-to-profile transform. **Never spawns
//! `git`** -- callers capture `git log`/`git ls-tree` output themselves (the thin binary in
//! `src/bin/extract_profile.rs` reads them from files) and this module turns already-captured text
//! into a [`Profile`]. Three reasons, per the handoff: no `git` dependency enters this workspace;
//! the extraction command is recorded verbatim in the profile itself rather than buried in Rust
//! that shells out; and it makes this module testable against a small committed fixture, without a
//! repository, which is exactly the incomparability defect RFC 139 exists to retire.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::profile::{BuilderInputs, OperationKindMix, Profile, Provenance, SCHEMA_VERSION, Shape};

/// Provenance and builder-input fields the log/ls-tree text cannot supply on their own -- the
/// caller states how the text was produced and what the builder must fix for determinism.
/// `Deserialize` so the thin binary can read one straight from a small TOML context file rather
/// than a wall of CLI flags -- the context file is not itself the profile (it carries none of the
/// shape data, only what could not be derived from the log/ls-tree text), just the recipe this
/// extractor needs to fill in the fields the text alone cannot.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionContext {
    /// See [`Provenance::source_repository`].
    pub source_repository: String,
    /// See [`Provenance::revision`].
    pub revision: String,
    /// See [`Provenance::extraction_commands`].
    pub extraction_commands: Vec<String>,
    /// See [`Provenance::extraction_date`].
    pub extraction_date: String,
    /// See [`Provenance::rename_detection`].
    pub rename_detection: bool,
    /// See [`BuilderInputs`].
    pub builder_inputs: BuilderInputs,
}

/// Malformed extraction input, refused rather than silently absorbed (handoff §6 control 5): a
/// profile derived from half its input is worse than no profile, because it is comparable to
/// nothing and looks fine. Every variant names the 1-based input line it failed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// A malformed line in the `git log --name-status` text.
    Log {
        /// 1-based line number in the log text.
        line: usize,
        /// What was wrong.
        message: String,
    },
    /// A malformed line in the `git ls-tree -r -l` text.
    LsTree {
        /// 1-based line number in the ls-tree text.
        line: usize,
        /// What was wrong.
        message: String,
    },
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Log { line, message } => write!(f, "log text, line {line}: {message}"),
            Self::LsTree { line, message } => write!(f, "ls-tree text, line {line}: {message}"),
        }
    }
}

impl std::error::Error for ExtractError {}

/// Extract a [`Profile`] from already-captured `git log --pretty=format:'@@%H' --name-status
/// --no-merges` text and `git ls-tree -r -l` text. Pure: reads only its arguments, spawns nothing.
pub fn extract_profile(
    log_text: &str,
    ls_tree_text: &str,
    context: ExtractionContext,
) -> Result<Profile, ExtractError> {
    let (commit_count, files_changed_per_commit, operation_kind_mix, path_touch_counts) =
        parse_log(log_text)?;
    // RFC 139 §4's prohibition: a profile stores aggregate distributions only, never paths from
    // the source project. `path_touch_counts` above is keyed by the real path string (needed to
    // count how many *distinct* times each one was touched); collapsed here into a touch-count ->
    // distinct-path-count histogram before it ever reaches `Profile` -- no path string survives
    // past this line.
    let distinct_paths = u64::try_from(path_touch_counts.len()).unwrap_or(u64::MAX);
    let path_touches = histogram_of_values(path_touch_counts.values().copied());
    let file_sizes = parse_ls_tree(ls_tree_text)?;

    Ok(Profile {
        schema_version: SCHEMA_VERSION,
        provenance: Provenance {
            source_repository: context.source_repository,
            revision: context.revision,
            extraction_commands: context.extraction_commands,
            extraction_date: context.extraction_date,
            rename_detection: context.rename_detection,
        },
        shape: Shape {
            commit_count,
            files_changed_per_commit,
            operation_kind_mix,
            distinct_paths,
            path_touches,
            file_sizes,
        },
        builder_inputs: context.builder_inputs,
    })
}

const COMMIT_HEADER_PREFIX: &str = "@@";

#[allow(clippy::type_complexity)]
fn parse_log(
    text: &str,
) -> Result<
    (
        u64,
        BTreeMap<String, u64>,
        OperationKindMix,
        BTreeMap<String, u64>,
    ),
    ExtractError,
> {
    let mut commit_count: u64 = 0;
    let mut files_changed_histogram: BTreeMap<u64, u64> = BTreeMap::new();
    let mut mix = OperationKindMix::default();
    let mut path_touch_counts: BTreeMap<String, u64> = BTreeMap::new();

    let mut current_commit: Option<(usize, u64)> = None; // (header line, files seen)

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end_matches(['\r']);
        if line.is_empty() {
            // Blank separator between commits -- git's own `--pretty=format` combined with
            // `--name-status` inserts one; insignificant on its own, and control 5's "a commit
            // header with no file lines" is checked at the *next* header (or EOF), not here.
            continue;
        }
        if let Some(hash) = line.strip_prefix(COMMIT_HEADER_PREFIX) {
            if hash.trim().is_empty() {
                return Err(ExtractError::Log {
                    line: line_number,
                    message: "commit header carries no commit hash".to_string(),
                });
            }
            if let Some((header_line, files_seen)) = current_commit.take() {
                if files_seen == 0 {
                    return Err(ExtractError::Log {
                        line: header_line,
                        message: "commit header has no file lines before the next header"
                            .to_string(),
                    });
                }
                *files_changed_histogram.entry(files_seen).or_insert(0) += 1;
            }
            commit_count += 1;
            current_commit = Some((line_number, 0));
            continue;
        }

        let Some((_, files_seen)) = current_commit.as_mut() else {
            return Err(ExtractError::Log {
                line: line_number,
                message: "file line appears before any commit header".to_string(),
            });
        };

        let mut fields = line.split('\t');
        let status = fields.next().ok_or_else(|| ExtractError::Log {
            line: line_number,
            message: "empty status/path line".to_string(),
        })?;
        let category = status.chars().next().ok_or_else(|| ExtractError::Log {
            line: line_number,
            message: "empty status field".to_string(),
        })?;

        match category {
            'A' | 'M' | 'D' | 'T' => {
                let path = fields.next().ok_or_else(|| ExtractError::Log {
                    line: line_number,
                    message: format!("status {status:?} has no path"),
                })?;
                if fields.next().is_some() {
                    return Err(ExtractError::Log {
                        line: line_number,
                        message: format!("status {status:?} carries more than one path"),
                    });
                }
                *path_touch_counts.entry(path.to_string()).or_insert(0) += 1;
                match category {
                    'A' => mix.added += 1,
                    'M' => mix.modified += 1,
                    'D' => mix.deleted += 1,
                    'T' => mix.type_changed += 1,
                    _ => unreachable!(),
                }
            }
            'R' | 'C' => {
                let old_path = fields.next().ok_or_else(|| ExtractError::Log {
                    line: line_number,
                    message: format!("status {status:?} has no old path"),
                })?;
                let new_path = fields.next().ok_or_else(|| ExtractError::Log {
                    line: line_number,
                    message: format!("status {status:?} has no new path"),
                })?;
                if fields.next().is_some() {
                    return Err(ExtractError::Log {
                        line: line_number,
                        message: format!("status {status:?} carries more than two paths"),
                    });
                }
                *path_touch_counts.entry(old_path.to_string()).or_insert(0) += 1;
                *path_touch_counts.entry(new_path.to_string()).or_insert(0) += 1;
                if category == 'R' {
                    mix.renamed += 1;
                } else {
                    mix.copied += 1;
                }
            }
            other => {
                return Err(ExtractError::Log {
                    line: line_number,
                    message: format!("unrecognized status letter {other:?}"),
                });
            }
        }
        *files_seen += 1;
    }

    if let Some((header_line, files_seen)) = current_commit {
        if files_seen == 0 {
            return Err(ExtractError::Log {
                line: header_line,
                message: "commit header has no file lines before end of input".to_string(),
            });
        }
        *files_changed_histogram.entry(files_seen).or_insert(0) += 1;
    }

    let files_changed_per_commit = stringify_keys(files_changed_histogram);
    Ok((
        commit_count,
        files_changed_per_commit,
        mix,
        path_touch_counts,
    ))
}

/// Build a value -> occurrence-count histogram from a sequence of `u64` values -- used for both
/// `path_touches` (from each distinct path's own touch count, the path string itself discarded
/// before this point) and `file_sizes` (from each blob's own byte size).
fn histogram_of_values(values: impl Iterator<Item = u64>) -> BTreeMap<String, u64> {
    let mut histogram: BTreeMap<u64, u64> = BTreeMap::new();
    for value in values {
        *histogram.entry(value).or_insert(0) += 1;
    }
    stringify_keys(histogram)
}

/// TOML has no integer-keyed table; every histogram in [`crate::profile::Shape`] is a sparse map
/// from an integer value (rendered as a string) to how many times it was observed.
fn stringify_keys(histogram: BTreeMap<u64, u64>) -> BTreeMap<String, u64> {
    histogram
        .into_iter()
        .map(|(value, count)| (value.to_string(), count))
        .collect()
}

fn parse_ls_tree(text: &str) -> Result<BTreeMap<String, u64>, ExtractError> {
    let mut sizes_histogram: BTreeMap<u64, u64> = BTreeMap::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end_matches(['\r']);
        if line.is_empty() {
            continue;
        }
        let Some((metadata, _path)) = line.split_once('\t') else {
            return Err(ExtractError::LsTree {
                line: line_number,
                message: "no tab separating metadata from path".to_string(),
            });
        };
        let fields: Vec<&str> = metadata.split_whitespace().collect();
        let [_mode, object_type, _object_id, size_str] = fields.as_slice() else {
            return Err(ExtractError::LsTree {
                line: line_number,
                message: format!(
                    "expected 4 whitespace-separated fields (mode, type, object id, size), got {}",
                    fields.len()
                ),
            });
        };
        // `-r` recurses into trees, so only blob (regular file/symlink) leaves carry a byte size;
        // a `commit` entry (a submodule gitlink) has none and is not a malformed line, just not a
        // sized file -- skipped, not refused.
        if *object_type != "blob" {
            if *object_type != "commit" && *object_type != "tree" {
                return Err(ExtractError::LsTree {
                    line: line_number,
                    message: format!("unrecognized object type {object_type:?}"),
                });
            }
            continue;
        }
        let size: u64 = size_str.parse().map_err(|_| ExtractError::LsTree {
            line: line_number,
            message: format!("size field {size_str:?} is not a non-negative integer"),
        })?;
        *sizes_histogram.entry(size).or_insert(0) += 1;
    }
    Ok(stringify_keys(sizes_histogram))
}

#[cfg(test)]
mod tests;
