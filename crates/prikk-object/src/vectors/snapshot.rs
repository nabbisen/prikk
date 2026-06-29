//! Generated-snapshot diff test. The committed snapshot is `snapshot.txt`.
//!
//! On an intended change, regenerate with
//! `PRIKK_REGEN=1 cargo test -p prikk-object` and review the diff. Regeneration
//! never touches the hard FDD vectors in `hard.rs`.

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
        "generated identity snapshot drifted from snapshot.txt. If intended, \
         regenerate with PRIKK_REGEN=1 cargo test -p prikk-object and review the \
         diff; hard FDD vectors are unaffected.",
    );
}
