//! DC-90: the unsafe-code boundary. The owner's ruling permits `unsafe` "under control with safety
//! and maintainability preserved"; this is what turns "under control" into a checked property rather
//! than a convention the first increment that needs FFI happens to invent.
//!
//! **The rule.** At most one workspace crate may omit `forbid(unsafe_code)`, named explicitly in
//! [`UNSAFE_EXEMPT_CRATES`] below — never inferred from what a crate happens to do. Every other
//! member must inherit the workspace lint table (`[lints]` / `workspace = true`), which is where
//! `unsafe_code = "forbid"` lives (root `Cargo.toml`, `[workspace.lints.rust]`).
//!
//! **Why the SAFETY-comment lint lives at the workspace root and not in the exempt crate's own
//! manifest (DC-90 ruling §3).** `clippy::undocumented_unsafe_blocks` is enabled once, in the root
//! `[workspace.lints.clippy]` table. If it were enabled in the exempt crate's own manifest instead,
//! that crate — the one permitted to write `unsafe` — could switch the requirement off by deleting
//! one line, and every other gate in the set would still pass, because none of them depend on this
//! lint. A control the controlled party can silently remove is a convention, not a control.
//!
//! **What makes this self-guarding.** A crate cannot have `[lints] workspace = true` *and* a local
//! override in the same manifest — confirmed empirically against `cargo`'s own manifest parser
//! during this increment's investigation, not assumed. So a crate has exactly two options: inherit
//! the workspace table in full (in which case `unsafe_code = "forbid"` and
//! `undocumented_unsafe_blocks = "forbid"` both apply, unconditionally), or opt out of inheritance
//! entirely. The one crate legitimately exercising the exemption necessarily takes the second path —
//! but taking it is not enough on its own to pass this check. **A crate that opts out of workspace
//! inheritance must re-declare `undocumented_unsafe_blocks = "forbid"` locally**, in its own
//! `[lints.clippy]` table, or the check fails. This is the specific rule DC-90's negative-control
//! requirement targets: the exempt crate attempting to drop inheritance without re-declaring the
//! guard is exactly the failure this boundary exists to catch.
//!
//! **Why the re-declaration must be `"forbid"`, not `"deny"` (found by review, not by the original
//! design — recorded so the mistake is not repeated).** A `deny`-level lint can be locally overridden
//! by an inner `#[allow(...)]`; a `forbid`-level one cannot — that asymmetry is exactly why
//! `unsafe_code = "forbid"` is robust in the first place, and this module originally applied the
//! principle to `unsafe_code` while setting `undocumented_unsafe_blocks` itself to `"deny"`. That
//! left a one-line escape: a crate could re-declare the lint at `"deny"` (satisfying the check as
//! first built), then add `#![allow(clippy::undocumented_unsafe_blocks)]` at its own crate root, and
//! every gate in the set — including this one — would still pass, silently, while the SAFETY-comment
//! requirement was gone. A guard weaker than the thing it guards is not a guard. `"forbid"` closes
//! this: `#[allow(...)]` against a `forbid`-level lint is a hard compile error
//! (`E0453: incompatible with previous forbid`), verified directly rather than assumed.
//!
//! **Dependency isolation, when a crate is actually named.** [`UNSAFE_EXEMPT_CRATES`]'s associated
//! third-party allowlist is separate from `placement.rs`'s `ALLOWED_THIRD_PARTY` — an unsafe-exempt
//! crate's third-party dependencies are not implicitly covered by whatever a product crate happens
//! to already be allowed, so the exception cannot become a side door around DC-51's placement gate.
//! Empty today, matching `UNSAFE_EXEMPT_CRATES` itself: no crate has an unsafe exception yet
//! (acceptance criterion 3 — this being a meaningfully checked state, not a special case that starts
//! working only once something is added).
//!
//! ## What this gate cannot see (DC-90 §4.4)
//!
//! Stated plainly, per DC-90's own standard that a passing check is not evidence of a guarantee it
//! does not test:
//!
//! - **FFI-ABI correctness.** Whether an `extern "system"` declaration's signature actually matches
//!   the real platform ABI it calls, and whether pointer, lifetime, and buffer invariants hold across
//!   that boundary, is not machine-checkable at this layer — the FFI boundary is precisely where
//!   Rust's own guarantees stop. **Review obligation:** every `SAFETY:` comment must be read by a
//!   human against the actual platform API documentation at review time.
//! - **Comment content, not merely presence.** `clippy::undocumented_unsafe_blocks` fails on a
//!   missing comment, not an inadequate one — `// SAFETY: trust me` satisfies the lint exactly as
//!   well as a real invariant justification. **Review obligation:** same as above; comment quality is
//!   a human judgment this gate cannot make.
//! - **Staleness — the limit most likely to bite, because it degrades silently and a green gate
//!   looks identical either way.** The lint checks, at compile time, that a comment exists next to an
//!   `unsafe` block as the block stands *right now*. If the block's body changes in a way that
//!   invalidates the reasoning in an unchanged comment above it, nothing here detects that the
//!   comment no longer justifies what the code now does — the gate stays green while the guarantee it
//!   implies quietly stops being true. **Review obligation:** any diff touching an existing `unsafe`
//!   block's body must be reviewed against its own comment's continued accuracy, every time, not just
//!   checked for the comment's presence once.
//! - **Dependencies' own internal unsafe is out of scope by design**, not merely unchecked. `rustix`
//!   and any other dependency's FFI is not audited by this boundary; it governs code prikk writes.

use super::{BoundaryError, PRODUCTS, push};

/// At most one entry (checked by [`check_exemption_list_size`]). Empty today.
const UNSAFE_EXEMPT_CRATES: &[&str] = &[];

const SELF_GUARDING_LINT: &str = "undocumented_unsafe_blocks";
/// `"forbid"`, not `"deny"` — see the module doc's "why the re-declaration must be forbid" section.
/// A `deny`-level lint can be locally overridden by an inner `#[allow(...)]`; `forbid` cannot.
const SELF_GUARDING_LEVEL: &str = "forbid";

pub(super) fn check(root: &std::path::Path, errors: &mut Vec<BoundaryError>) {
    check_root_lint_table(root, errors);
    check_exemption_list_size(UNSAFE_EXEMPT_CRATES, errors);
    for (crate_name, manifest_path) in all_members() {
        match std::fs::read_to_string(root.join(manifest_path)) {
            Ok(text) => check_member(crate_name, &text, UNSAFE_EXEMPT_CRATES, errors),
            Err(_) => push(
                errors,
                "unsafe-boundary",
                format!("{crate_name}: manifest unreadable"),
            ),
        }
    }
}

fn all_members() -> Vec<(&'static str, &'static str)> {
    PRODUCTS
        .into_iter()
        .chain([("prikk-release-policy", "tools/release-policy/Cargo.toml")])
        .collect()
}

fn check_root_lint_table(root: &std::path::Path, errors: &mut Vec<BoundaryError>) {
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        push(
            errors,
            "unsafe-boundary",
            "root manifest unreadable".to_owned(),
        );
        return;
    };
    if clippy_lint_level(&text, SELF_GUARDING_LINT, |value| {
        value
            .get("workspace")
            .and_then(toml::Value::as_table)
            .and_then(|workspace| workspace.get("lints"))
            .and_then(toml::Value::as_table)
    }) != Some(SELF_GUARDING_LEVEL.to_owned())
    {
        push(
            errors,
            "unsafe-boundary",
            format!(
                "root workspace.lints.clippy.{SELF_GUARDING_LINT} != \"{SELF_GUARDING_LEVEL}\""
            ),
        );
    }
}

fn check_exemption_list_size(exempt: &[&str], errors: &mut Vec<BoundaryError>) {
    if exempt.len() > 1 {
        push(
            errors,
            "unsafe-boundary",
            format!(
                "exemption list has {} entries; at most one is allowed",
                exempt.len()
            ),
        );
    }
}

fn check_member(crate_name: &str, text: &str, exempt: &[&str], errors: &mut Vec<BoundaryError>) {
    let Ok(manifest) = toml::from_str::<toml::Value>(text) else {
        push(
            errors,
            "unsafe-boundary",
            format!("{crate_name}: manifest unparseable"),
        );
        return;
    };
    let inherits_workspace_lints = manifest
        .get("lints")
        .and_then(toml::Value::as_table)
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true);
    if inherits_workspace_lints {
        // Fully protected by the root table either way -- exempt-listed or not, nothing more to
        // check: forbid(unsafe_code) and the SAFETY-comment lint both apply unconditionally.
        return;
    }
    if !exempt.contains(&crate_name) {
        push(
            errors,
            "unsafe-boundary",
            format!(
                "{crate_name}: does not inherit workspace lints and is not in the exemption list"
            ),
        );
        return;
    }
    // Exempt, and opted out of full inheritance -- the one legitimate reason to do that is to allow
    // unsafe_code, which is exactly the case the SAFETY-comment requirement must not travel with it.
    // The crate must have re-declared the self-guarding lint locally instead of relying on the
    // inheritance it just opted out of.
    if clippy_lint_level(text, SELF_GUARDING_LINT, |value| {
        value.get("lints").and_then(toml::Value::as_table)
    }) != Some(SELF_GUARDING_LEVEL.to_owned())
    {
        push(
            errors,
            "unsafe-boundary",
            format!(
                "{crate_name}: exempt from workspace lint inheritance but does not locally \
                 re-declare lints.clippy.{SELF_GUARDING_LINT} = \"{SELF_GUARDING_LEVEL}\""
            ),
        );
    }
}

/// Reads `<lints_table>.clippy.<lint>` as a plain string level (`lint = "deny"`), where
/// `lints_table` is located by `locate` (the workspace's `[workspace.lints]` for the root manifest,
/// or a member's own `[lints]` for a member manifest). Does not accept the `{ level = "...", ... }`
/// table form -- this project writes lint levels as plain strings everywhere today, and a mismatch
/// in form is worth surfacing as a failure rather than silently accepting.
fn clippy_lint_level(
    text: &str,
    lint: &str,
    locate: impl Fn(&toml::Value) -> Option<&toml::Table>,
) -> Option<String> {
    let manifest: toml::Value = toml::from_str(text).ok()?;
    locate(&manifest)?
        .get("clippy")
        .and_then(toml::Value::as_table)?
        .get(lint)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "unsafe_boundary/tests.rs"]
mod tests;
