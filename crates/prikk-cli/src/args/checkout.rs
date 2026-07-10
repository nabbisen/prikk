//! Argument parsing for checkout commands.

use std::path::PathBuf;

use prikk_store::DEFAULT_CHECKOUT_REF;

use super::optional_path_or_current;

/// Parsed checkout command arguments.
pub(crate) struct CheckoutArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
    /// Checkout mode.
    pub(crate) mode: CheckoutMode,
}

/// Checkout command mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckoutMode {
    /// Read-only general checkout planning.
    PlanOnly,
    /// Read-only snapshot manifest validation.
    SnapshotPlan,
    /// Opt-in snapshot materialization.
    SnapshotMaterialize,
    /// Read-only supported patch replay planning.
    PatchPlan,
    /// Opt-in materialization from supported patch replay.
    PatchMaterialize,
    /// Read-only deletion plan for explicit patch-removed files.
    PatchDeletePlan,
    /// Opt-in patch materialization plus explicit patch-removed file deletion.
    PatchMaterializeDelete,
}

/// Parse `prikk checkout` arguments.
pub(crate) fn parse_checkout_args(args: Vec<String>) -> std::result::Result<CheckoutArgs, String> {
    let mut mode = None;
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--plan-only" => set_checkout_mode(&mut mode, CheckoutMode::PlanOnly)?,
            "--snapshot-plan" => set_checkout_mode(&mut mode, CheckoutMode::SnapshotPlan)?,
            "--snapshot-materialize" => {
                set_checkout_mode(&mut mode, CheckoutMode::SnapshotMaterialize)?
            }
            "--patch-plan" => set_checkout_mode(&mut mode, CheckoutMode::PatchPlan)?,
            "--patch-materialize" => set_checkout_mode(&mut mode, CheckoutMode::PatchMaterialize)?,
            "--patch-delete-plan" => set_checkout_mode(&mut mode, CheckoutMode::PatchDeletePlan)?,
            "--patch-materialize-delete" => {
                set_checkout_mode(&mut mode, CheckoutMode::PatchMaterializeDelete)?
            }
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("checkout --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("checkout --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown checkout argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("checkout accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    let Some(mode) = mode else {
        return Err(concat!(
            "checkout requires one mode flag: `--plan-only`, `--snapshot-plan`, ",
            "`--snapshot-materialize`, `--patch-plan`, `--patch-materialize`, ",
            "`--patch-delete-plan`, or `--patch-materialize-delete`",
        )
        .to_string());
    };
    Ok(CheckoutArgs {
        root: optional_path_or_current(path)?,
        ref_name,
        mode,
    })
}

fn set_checkout_mode(
    mode: &mut Option<CheckoutMode>,
    next: CheckoutMode,
) -> std::result::Result<(), String> {
    if mode.is_some() {
        return Err("checkout accepts only one mode flag".to_string());
    }
    *mode = Some(next);
    Ok(())
}
