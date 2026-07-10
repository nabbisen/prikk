//! Argument parsing for read-only merge evidence.

use std::path::PathBuf;

use prikk_object::ObjectId;

use super::optional_path_or_current;

/// Parsed merge-evidence command arguments.
pub(crate) struct MergeEvidenceArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Explicit sealed baseline block.
    pub(crate) baseline_block_id: ObjectId,
    /// Left target selector.
    pub(crate) left_target: MergeEvidenceTargetArg,
    /// Right target selector.
    pub(crate) right_target: MergeEvidenceTargetArg,
}

/// Parsed merge-evidence target selector.
pub(crate) enum MergeEvidenceTargetArg {
    /// Block selector.
    Block(ObjectId),
    /// Ref selector.
    Ref(String),
}

/// Parse `prikk merge-evidence` arguments.
pub(crate) fn parse_merge_evidence_args(
    args: Vec<String>,
) -> std::result::Result<MergeEvidenceArgs, String> {
    let mut path = None;
    let mut baseline_block_id = None;
    let mut left_target = None;
    let mut right_target = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--baseline-block" => {
                baseline_block_id = Some(parse_object_id_arg(
                    iter.next(),
                    "merge-evidence --baseline-block",
                )?);
            }
            "--left-block" => {
                let id = parse_object_id_arg(iter.next(), "merge-evidence --left-block")?;
                set_merge_target(&mut left_target, MergeEvidenceTargetArg::Block(id), "left")?;
            }
            "--right-block" => {
                let id = parse_object_id_arg(iter.next(), "merge-evidence --right-block")?;
                set_merge_target(
                    &mut right_target,
                    MergeEvidenceTargetArg::Block(id),
                    "right",
                )?;
            }
            "--left-ref" => {
                let ref_name = parse_non_empty_value(iter.next(), "merge-evidence --left-ref")?;
                set_merge_target(
                    &mut left_target,
                    MergeEvidenceTargetArg::Ref(ref_name),
                    "left",
                )?;
            }
            "--right-ref" => {
                let ref_name = parse_non_empty_value(iter.next(), "merge-evidence --right-ref")?;
                set_merge_target(
                    &mut right_target,
                    MergeEvidenceTargetArg::Ref(ref_name),
                    "right",
                )?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown merge-evidence argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("merge-evidence accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(MergeEvidenceArgs {
        root: optional_path_or_current(path)?,
        baseline_block_id: baseline_block_id
            .ok_or_else(|| "merge-evidence requires --baseline-block".to_string())?,
        left_target: left_target
            .ok_or_else(|| "merge-evidence requires --left-block or --left-ref".to_string())?,
        right_target: right_target
            .ok_or_else(|| "merge-evidence requires --right-block or --right-ref".to_string())?,
    })
}

fn set_merge_target(
    slot: &mut Option<MergeEvidenceTargetArg>,
    value: MergeEvidenceTargetArg,
    side: &str,
) -> std::result::Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "merge-evidence {side} side accepts exactly one selector"
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn parse_object_id_arg(
    value: Option<String>,
    label: &str,
) -> std::result::Result<ObjectId, String> {
    let value = parse_non_empty_value(value, label)?;
    value
        .parse::<ObjectId>()
        .map_err(|err| format!("{label} must be a lowercase 64-hex object id ({err})"))
}

fn parse_non_empty_value(
    value: Option<String>,
    label: &str,
) -> std::result::Result<String, String> {
    let Some(value) = value else {
        return Err(format!("{label} requires a value"));
    };
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(value)
}
