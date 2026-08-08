//! Argument parsing for `prikk merge`.

use std::path::PathBuf;

use prikk_object::ObjectId;

use super::optional_path_or_current;

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
) -> std::result::Result<MergeExecuteArgs, String> {
    let mut path = None;
    let mut allow_no_audit = false;
    let mut baseline_block_id = None;
    let mut into_ref = None;
    let mut from_ref = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-no-audit" => allow_no_audit = true,
            "--baseline-block" => {
                let value = require_value(iter.next(), "merge --baseline-block")?;
                baseline_block_id = Some(value.parse::<ObjectId>().map_err(|err| {
                    format!("merge --baseline-block must be a lowercase 64-hex object id ({err})")
                })?);
            }
            "--into" => {
                into_ref = Some(require_value(iter.next(), "merge --into")?);
            }
            "--from" => {
                from_ref = Some(require_value(iter.next(), "merge --from")?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown merge argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("merge accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    if !allow_no_audit {
        return Err("merge requires --allow-no-audit".to_string());
    }
    Ok(MergeExecuteArgs {
        root: optional_path_or_current(path)?,
        baseline_block_id: baseline_block_id
            .ok_or_else(|| "merge requires --baseline-block".to_string())?,
        into_ref: into_ref.ok_or_else(|| "merge requires --into".to_string())?,
        from_ref: from_ref.ok_or_else(|| "merge requires --from".to_string())?,
    })
}

fn require_value(value: Option<String>, label: &str) -> std::result::Result<String, String> {
    let Some(value) = value else {
        return Err(format!("{label} requires a value"));
    };
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(value)
}
