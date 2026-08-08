//! The filesystem durability contract (DC-76): what prikk-store requires of a filesystem to
//! mutate a repository safely, stated as guarantees rather than primitives.
//!
//! **Why a trait, not prose.** A markdown description of "what we need" can drift from what the
//! code actually does, silently, the moment either changes without the other. A trait is checked
//! by the compiler: every mutation path in `anchored.rs` calls through [`DurabilityContract`], so
//! there is exactly one place that can state — and one place a future platform's implementation
//! must satisfy — what durability prikk-store depends on. `Linux` is the sole implementor today
//! ([`super::anchored::LinuxDurability`]); no platform is added by this increment.
//!
//! **Guarantee, not syscall — the whole point.** [`atomic_replace`](DurabilityContract::atomic_replace)
//! says "replace this file's content atomically, durably" — never "write a temp file and call
//! `renameat`". A method named after a primitive would already be platform-specific before a
//! second platform exists. The worked example the architect asked this contract be built around:
//! [`durable_directory_entry`](DurabilityContract::durable_directory_entry) — "once this returns,
//! every mutation made under `relative` since the last durability point survives a crash" — is
//! satisfied on Linux by `fsync` on the directory fd, and (per DC-76 addendum-1, confirmed against
//! `rustix` 1.1.4's own source) would be satisfied on macOS by `fcntl(fd, F_FULLFSYNC)`
//! (`rustix::fs::fcntl_fullfsync`) instead — `fsync` alone does not give the same guarantee there.
//! Stating the method as "fsync" would already have been wrong, before macOS was ever implemented.
//!
//! ## Cross-cutting invariants — properties every method must hold, not separate methods
//!
//! These are not enumerated as trait methods because every operation below needs all of them
//! simultaneously; stating them once here, and testing them once per operation in the conformance
//! suite (`super::tests::conformance`), is more precise than restating "...and refuses symlinks"
//! on every doc comment.
//!
//! - **G1 — root-anchored resolution.** Every path this contract accepts is resolved relative to
//!   the [`MutationRoot`] that authorized it, one path component at a time, with no-follow on
//!   every component including the last. A symlink swapped in anywhere along the path — not only
//!   at the final component — must not let a mutation escape the root.
//! - **G6 — regular-file validation.** Any operation that opens an *existing* final entry
//!   confirms it is a regular file before mutating it. A device, FIFO, or symlink that raced into
//!   the resolved path is refused, not silently operated on.
//! - **G7 — non-blocking opens.** No operation may block indefinitely because a FIFO or device was
//!   substituted at the resolved path.
//! - **G8 — concurrent-process-safe directory creation.** Two processes racing to create the same
//!   directory component must both succeed; the loser observes and validates what the winner
//!   created rather than erroring.
//!
//! ## Guarantee-to-method map
//!
//! | Guarantee | Method |
//! |---|---|
//! | G2 (atomic content replacement) | [`atomic_replace`](DurabilityContract::atomic_replace) |
//! | G3 (durable-after-return) | every method below returns only once its effect is durable; [`durable_directory_entry`](DurabilityContract::durable_directory_entry) is the guarantee in its most direct form |
//! | G4 (exclusive creation) | [`create_exclusive`](DurabilityContract::create_exclusive) |
//! | G5 (race-safe no-clobber publication) | [`publish_immutable`](DurabilityContract::publish_immutable) |
//! | G9 (mode-bit isolation) | [`set_permission_bits`](DurabilityContract::set_permission_bits) |

use std::path::Path;

use prikk_error::Result;

use super::anchored::MutationRoot;

/// What prikk-store requires of a filesystem to mutate a repository durably and safely. See the
/// module documentation for the cross-cutting invariants (G1, G6, G7, G8) every method upholds and
/// the guarantee-to-method map. `root` names the authority every path is resolved against; `relative`
/// is always relative to it, never absolute.
///
/// Deliberately **not** gated to `target_os = "linux"`, even though `LinuxDurability` is currently
/// the only implementor: the whole point of this contract is a platform-neutral statement of what
/// the store requires, and a trait that vanishes on the platforms it exists to enable would defeat
/// that (DC-76 addendum-2 B1). Off Linux it is therefore genuinely unused — `#[allow(dead_code)]`
/// states that honestly rather than suppressing it, and it is expected to stop applying the moment
/// a second platform implements this trait.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) trait DurabilityContract {
    /// Replace `relative`'s content atomically and durably: a reader never observes a partial
    /// write, and a crash mid-replace leaves either the complete previous content or the complete
    /// new content, never a mix.
    fn atomic_replace(&self, root: &MutationRoot, relative: &Path, bytes: &[u8]) -> Result<()>;

    /// Durably append `bytes` to an existing regular file, creating it first if absent.
    fn durable_append(&self, root: &MutationRoot, relative: &Path, bytes: &[u8]) -> Result<()>;

    /// Durably truncate an existing regular file to `len`.
    fn durable_truncate(&self, root: &MutationRoot, relative: &Path, len: u64) -> Result<()>;

    /// Durably create-or-truncate `relative` to empty.
    fn durable_truncate_to_empty(&self, root: &MutationRoot, relative: &Path) -> Result<()>;

    /// Create `relative` **exclusively** — refuses if any entry already exists there — write
    /// `bytes`, and durably publish it. No code path through this method can silently overwrite
    /// existing content.
    fn create_exclusive(
        &self,
        root: &MutationRoot,
        relative: &Path,
        bytes: &[u8],
    ) -> std::io::Result<()>;

    /// Set `relative`'s permission bits on an existing regular file, accepting a "recorded mode"
    /// that carries file-type bits (e.g. `0o100_755`, matching a sealed `CreateFile`/`ChangePerm`
    /// operation's own `mode` field) without letting them influence what gets applied. **Not
    /// independently testable on Linux**: `fchmod`'s mode argument already ignores non-permission
    /// bits at the kernel level (confirmed by a reverted negative control — masking `mode & 0o7777`
    /// out of `LinuxDurability::set_permission_bits` before applying it produces byte-identical
    /// results to leaving the file-type bits in), so this masking is deliberate, defensive input
    /// handling for a reader's clarity, not a guarantee whose omission is Linux-observable. Left in
    /// the contract because a future platform's `fchmod`-equivalent may not be as forgiving.
    fn set_permission_bits(&self, root: &MutationRoot, relative: &Path, mode: u32) -> Result<()>;

    /// Durably remove `relative` if present; returns whether an entry was actually removed. Absence
    /// is not an error — the durability guarantee (the parent directory entry's removal, or its
    /// confirmed prior absence, is synced) holds either way.
    fn remove_if_present(&self, root: &MutationRoot, relative: &Path) -> Result<bool>;

    /// Durably rename `source` to `destination`, both resolved under `root`, syncing the
    /// destination directory before the source directory (so a crash between the two syncs still
    /// leaves the rename durable from the destination's side).
    fn promote(&self, root: &MutationRoot, source: &Path, destination: &Path) -> Result<()>;

    /// Publish `candidate` at `relative` **without ever replacing existing content**: if a
    /// different immutable object already exists there, this is a no-op after validating the
    /// existing bytes against `candidate`; if the same content wins a creation race against
    /// another process, both processes converge is on the same durable result rather than either
    /// silently overwriting the other.
    fn publish_immutable(
        &self,
        root: &MutationRoot,
        relative: &Path,
        candidate: &[u8],
        validate_existing: impl Fn(&[u8]) -> Result<()>,
    ) -> Result<()>;

    /// Ensure a relative directory tree exists, durably, tolerating a concurrent creator (G8).
    fn ensure_directory(&self, root: &MutationRoot, relative: &Path) -> Result<()>;

    /// Durably sync an existing root-relative directory's entry-list state — the guarantee in its
    /// most direct form (G3's worked example): once this returns, every mutation made under
    /// `relative` since the last durability point survives a crash.
    fn durable_directory_entry(&self, root: &MutationRoot, relative: &Path) -> Result<()>;
}
