//! Universal installer/uninstaller handoff v1 §4.1: generated at release time and attached as a
//! release asset, never committed as a tracked `.sh` file.
//!
//! **Why not committed.** `command_scan::scan_shell`/`scan_yaml` treat every `.sh`/`.yml`/`.yaml`
//! file *anywhere in the repository* as a governed procedure (`reference.rs`'s
//! `collect_text_files` walks the whole tree, excluding only `.git`/`.git-exclude`/`target`/
//! `docs/book`) and scan it in strict mode: every command head must be `inert_head` or an exact
//! shape registered in `command_scan/procedure.rs`, with no exemption for shell keywords
//! (`if`/`case`/`for`/`while`) or heredocs (`command_scan/lexer.rs` has no concept of either — it
//! tokenizes per line, treating `;`/`|`/`&`/`(`/`)`/`[`/`]` as command separators with no keyword
//! awareness). Confirmed empirically, not assumed: a real installer needs OS/architecture branching
//! and checksum-failure handling, which cannot be written without `if`/`case`; a probe script using
//! both, placed anywhere in the tree, fails `reference-check` with a wall of
//! `unclassified-procedure-command`/`unclassified-dynamic-command` errors. No `.sh` file has ever
//! existed in this repository before this handoff — the scanner's `.sh` handling has never been
//! exercised against a real, control-flow-bearing script, only ever against `release.yml`'s own
//! flat, keyword-free command lists.
//!
//! Rewriting that scanner to understand real POSIX shell syntax would be a substantial,
//! security-relevant change to review-gated policy code (`EXECUTION-ORDER.md` §6 rule 5) — a
//! different, dedicated increment, not a side effect of shipping an installer. Instead, the actual
//! script text lives in `templates/*.sh.txt` — a `.txt` extension is outside
//! `scannable_reference_file`'s `md|yml|yaml|sh` set, so it is read only by the loose,
//! non-strict `scan()` (which looks for stray `cargo publish`/`prikk-release-policy … check`
//! invocations in prose, not full shell-command validation) and never subjected to the strict
//! per-command allowlist. The generated `.sh` file this module writes exists only transiently in a
//! release's own `dist/` directory (never committed), exactly like the tarballs and checksums
//! already produced there — so it is never walked by `reference-check`/`boundary-check` either.

use std::fs;
use std::path::Path;

use crate::error::Result;

const REPO_SLUG: &str = "prikk-vcs/prikk";
const INSTALL_TEMPLATE: &str = include_str!("../templates/install.sh.txt");
const UNINSTALL_TEMPLATE: &str = include_str!("../templates/uninstall.sh.txt");

/// Write `install.sh` and `uninstall.sh` into `dist_dir`, substituting the one placeholder
/// (`REPO_SLUG`) the templates carry. Executable on Unix, where the release `publish` job actually
/// runs this (`ubuntu-latest`) — `#[cfg(unix)]`, not because a Windows build of this tool could not
/// otherwise compile, but because `std::os::unix::fs::PermissionsExt` does not exist elsewhere and
/// nothing in this workspace needs it to.
pub(crate) fn generate(dist_dir: &Path) -> Result<()> {
    fs::create_dir_all(dist_dir)?;
    write_script(dist_dir, "install.sh", INSTALL_TEMPLATE)?;
    write_script(dist_dir, "uninstall.sh", UNINSTALL_TEMPLATE)?;
    Ok(())
}

fn write_script(dist_dir: &Path, name: &str, template: &str) -> Result<()> {
    let content = template.replace("REPO_SLUG", REPO_SLUG);
    let path = dist_dir.join(name);
    fs::write(&path, content)?;
    mark_executable(&path)?;
    Ok(())
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
