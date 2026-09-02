//! Argument parsing for `prikk merge`.

use std::path::PathBuf;

use prikk_object::ObjectId;

use super::optional_path_or_current;
use crate::arg_scan::{SetOnce, flag_value, mark_seen, unknown_argument};
use crate::commands::CliError;

/// Parsed `merge` command arguments.
pub(crate) struct MergeExecuteArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Explicit sealed baseline block confluence is proven against.
    pub(crate) baseline_block_id: ObjectId,
    /// Ref advanced by the merge.
    pub(crate) into_ref: String,
    /// Ref merged in.
    pub(crate) from_ref: String,
}

/// Parse `prikk merge` arguments.
pub(crate) fn parse_merge_execute_args(
    args: Vec<String>,
) -> std::result::Result<MergeExecuteArgs, CliError> {
    let mut path = None;
    let mut allow_no_audit = false;
    let mut baseline_block_id = None;
    let mut into_ref = None;
    let mut from_ref = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-no-audit" => mark_seen(&mut allow_no_audit, "--allow-no-audit")?,
            "--baseline-block" => {
                let value = require_value(&mut iter, "merge --baseline-block")?;
                let id = value.parse::<ObjectId>().map_err(|err| {
                    CliError::Usage(format!(
                        "merge --baseline-block must be a lowercase 64-hex object id ({err})"
                    ))
                })?;
                baseline_block_id.set_once("--baseline-block", id)?;
            }
            "--into" => {
                let value = require_value(&mut iter, "merge --into")?;
                into_ref.set_once("--into", value)?;
            }
            "--from" => {
                let value = require_value(&mut iter, "merge --from")?;
                from_ref.set_once("--from", value)?;
            }
            other if other.starts_with('-') => return Err(unknown_argument("merge", other)),
            _ => {
                if path.is_some() {
                    return Err(CliError::Usage(
                        "merge accepts at most one path".to_string(),
                    ));
                }
                path = Some(arg);
            }
        }
    }
    if !allow_no_audit {
        return Err(CliError::Usage(
            "merge requires --allow-no-audit".to_string(),
        ));
    }
    Ok(MergeExecuteArgs {
        root: optional_path_or_current(path)?,
        baseline_block_id: baseline_block_id
            .ok_or_else(|| CliError::Usage("merge requires --baseline-block".to_string()))?,
        into_ref: into_ref.ok_or_else(|| CliError::Usage("merge requires --into".to_string()))?,
        from_ref: from_ref.ok_or_else(|| CliError::Usage("merge requires --from".to_string()))?,
    })
}

fn require_value(
    iter: &mut std::vec::IntoIter<String>,
    label: &str,
) -> std::result::Result<String, CliError> {
    let value = flag_value(iter, label)?;
    if value.trim().is_empty() {
        return Err(CliError::Usage(format!("{label} must not be empty")));
    }
    Ok(value)
}
