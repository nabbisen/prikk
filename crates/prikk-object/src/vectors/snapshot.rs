//! Generated-snapshot diff test. The committed snapshot is `snapshot.txt`.
//!
//! On an intended encoding change, regenerate with
//! `PRIKK_REGEN=1 cargo test -p prikk-object` and review the diff. Regeneration
//! never touches the hard FDD vectors in `hard.rs`.
//!
//! **A snapshot diff during an identity-preservation increment (e.g. DC-55) is a stop-work
//! finding, not a regeneration trigger.** Regenerating rewrites every expected ObjectId to match
//! whatever the candidate produced, turning the test green while destroying the only signal that
//! the change was not identity-preserving. If the increment you are implementing claims to change
//! no persisted identity, a failure here means the claim is false — stop and escalate with the
//! differing rows, do not regenerate.

use super::generate_snapshot;

const COMMITTED: &str = include_str!("snapshot.txt");

// `expect` is intentional here: a failed regeneration write must abort the test.
#[allow(clippy::expect_used)]
#[test]
fn generated_snapshot_matches_committed() {
    let current = generate_snapshot();

    if std::env::var_os("PRIKK_REGEN").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/vectors/snapshot.txt");
        std::fs::write(path, &current).expect("write regenerated snapshot");
        return;
    }

    assert_eq!(
        current, COMMITTED,
        "generated identity snapshot drifted from snapshot.txt. If this is an intended encoding \
         change, regenerate with PRIKK_REGEN=1 cargo test -p prikk-object and review the diff; \
         hard FDD vectors are unaffected. If this increment claims to preserve existing identity \
         (e.g. DC-55), this drift means that claim is false: STOP, do not regenerate, and escalate \
         with the differing rows instead.",
    );
}
