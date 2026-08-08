//! `prikk merge` command implementation.

use prikk_store::{MaintainerSigner, MergeExecutionReport, execute_merge};

use crate::args::parse_merge_execute_args;
use crate::open_repository;

/// Parse and run `prikk merge`.
pub(crate) fn run_merge(
    args: Vec<String>,
    signer: &impl MaintainerSigner,
) -> std::result::Result<MergeExecutionReport, String> {
    let args = parse_merge_execute_args(args)?;
    let layout = open_repository(args.root)?;
    execute_merge(
        &layout,
        args.baseline_block_id,
        &args.into_ref,
        &args.from_ref,
        signer,
    )
    .map_err(|err| err.to_string())
}
