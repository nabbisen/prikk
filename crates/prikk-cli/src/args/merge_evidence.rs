//! Argument parsing for read-only merge evidence and merge planning.

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

/// Parsed merge-plan command arguments.
pub(crate) type MergePlanArgs = MergeEvidenceArgs;

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
    parse_merge_args(args, "merge-evidence")
}

/// Parse `prikk merge-plan` arguments.
pub(crate) fn parse_merge_plan_args(
    args: Vec<String>,
) -> std::result::Result<MergePlanArgs, String> {
    parse_merge_args(args, "merge-plan")
}

fn parse_merge_args(
    args: Vec<String>,
    command: &str,
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
                    &format!("{command} --baseline-block"),
                )?);
            }
            "--left-block" => {
                let id = parse_object_id_arg(iter.next(), &format!("{command} --left-block"))?;
                set_merge_target(
                    &mut left_target,
                    MergeEvidenceTargetArg::Block(id),
                    "left",
                    command,
                )?;
            }
            "--right-block" => {
                let id = parse_object_id_arg(iter.next(), &format!("{command} --right-block"))?;
                set_merge_target(
                    &mut right_target,
                    MergeEvidenceTargetArg::Block(id),
                    "right",
                    command,
                )?;
            }
            "--left-ref" => {
                let ref_name =
                    parse_non_empty_value(iter.next(), &format!("{command} --left-ref"))?;
                set_merge_target(
                    &mut left_target,
                    MergeEvidenceTargetArg::Ref(ref_name),
                    "left",
                    command,
                )?;
            }
            "--right-ref" => {
                let ref_name =
                    parse_non_empty_value(iter.next(), &format!("{command} --right-ref"))?;
                set_merge_target(
                    &mut right_target,
                    MergeEvidenceTargetArg::Ref(ref_name),
                    "right",
                    command,
                )?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown {command} argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err(format!("{command} accepts at most one path"));
                }
                path = Some(arg);
            }
        }
    }
    Ok(MergeEvidenceArgs {
        root: optional_path_or_current(path)?,
        baseline_block_id: baseline_block_id
            .ok_or_else(|| format!("{command} requires --baseline-block"))?,
        left_target: left_target
            .ok_or_else(|| format!("{command} requires --left-block or --left-ref"))?,
        right_target: right_target
            .ok_or_else(|| format!("{command} requires --right-block or --right-ref"))?,
    })
}

fn set_merge_target(
    slot: &mut Option<MergeEvidenceTargetArg>,
    value: MergeEvidenceTargetArg,
    side: &str,
    command: &str,
) -> std::result::Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "{command} {side} side accepts exactly one selector"
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
