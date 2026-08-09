//! Trust-store tests.
//!
//! DC-78 changed `add_trusted_maintainer` from "add or replace the single key" to "add a new key,
//! confirm idempotently if it already matches, refuse if it conflicts" — every test below that
//! previously exercised "adding a second key id replaces the first" or "re-adding the same key id
//! with a different public key replaces it" has been rewritten, not merely patched: those behaviors
//! no longer exist, on purpose (§D5's TOFU enforcement).

#![allow(clippy::indexing_slicing)]

use prikk_crypto::Ed25519KeyPair;

use crate::{
    RepositoryLayout, add_trusted_maintainer, load_maintainer_trust_policy, verify_signer_trusted,
};

use crate::fsutil::{TestFailPoint, fail_after_for_test, fail_once_for_test};
use crate::maintainer_signing::Ed25519MaintainerSigner;
use crate::test_support::unique_temp_dir;

fn public_key_hex(seed: &[u8; 32]) -> String {
    let key_pair = Ed25519KeyPair::from_seed(seed);
    key_pair
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn add_and_load_single_maintainer_policy() {
    let root = unique_temp_dir("trust-add-load");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let seed = [7_u8; 32];
        let public_key = public_key_hex(&seed);
        let added = add_trusted_maintainer(&layout, "maintainer_1", &public_key);
        assert!(added.is_ok());
        if let Ok((adopted, newly_added)) = added {
            assert!(newly_added);
            assert_eq!(adopted.key_id, "maintainer_1");
        }
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.keys.len(), 1);
            assert_eq!(loaded.keys[0].key_id, "maintainer_1");
            assert_eq!(
                loaded.keys[0].public_key,
                Ed25519KeyPair::from_seed(&seed).public_key_bytes()
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn adopting_a_second_key_id_keeps_the_first_trusted() {
    let root = unique_temp_dir("trust-add-second");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first_key = public_key_hex(&[1_u8; 32]);
        let second_key = public_key_hex(&[2_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "alice", &first_key).is_ok());
        assert!(add_trusted_maintainer(&layout, "bob", &second_key).is_ok());
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.keys.len(), 2);
            assert_eq!(loaded.keys[0].key_id, "alice");
            assert_eq!(loaded.keys[1].key_id, "bob");
            assert_eq!(
                loaded.keys[0].public_key,
                Ed25519KeyPair::from_seed(&[1_u8; 32]).public_key_bytes()
            );
            assert_eq!(
                loaded.keys[1].public_key,
                Ed25519KeyPair::from_seed(&[2_u8; 32]).public_key_bytes()
            );
        }
        // Both remain independently verifiable through the signer-trust path.
        if let Ok(alice) = Ed25519MaintainerSigner::from_seed("alice", &[1_u8; 32]) {
            assert!(verify_signer_trusted(&layout, &alice).is_ok());
        }
        if let Ok(bob) = Ed25519MaintainerSigner::from_seed("bob", &[2_u8; 32]) {
            assert!(verify_signer_trusted(&layout, &bob).is_ok());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn readopting_the_same_key_id_and_key_is_idempotent() {
    let root = unique_temp_dir("trust-readopt-same");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let key = public_key_hex(&[3_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "maintainer", &key).is_ok());
        let second_call = add_trusted_maintainer(&layout, "maintainer", &key);
        assert!(second_call.is_ok());
        if let Ok((_, newly_added)) = second_call {
            assert!(!newly_added);
        }
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(
                loaded.keys.len(),
                1,
                "idempotent re-adoption must not duplicate"
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-78 negative control 3: a changed public key for an already-adopted key id is refused, not
/// re-prompted or silently replaced.
#[test]
fn readopting_an_existing_key_id_with_a_different_key_is_refused() {
    let root = unique_temp_dir("trust-readopt-conflict");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let original = public_key_hex(&[4_u8; 32]);
        let conflicting = public_key_hex(&[5_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "maintainer", &original).is_ok());
        assert!(add_trusted_maintainer(&layout, "maintainer", &conflicting).is_err());
        // The refusal must not have changed anything.
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.keys.len(), 1);
            assert_eq!(
                loaded.keys[0].public_key,
                Ed25519KeyPair::from_seed(&[4_u8; 32]).public_key_bytes()
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn policy_rejects_unsafe_key_id() {
    let root = unique_temp_dir("trust-unsafe-key");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let public_key = public_key_hex(&[7_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "../bad", &public_key).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-78 §D7.1: the multi-key parser stays strict and fixed-shape. Two keys are now accepted;
/// malformed two-key syntax is still rejected — this is the inverse of the pre-DC-78 test of the
/// same name, kept as the same test identity because the property worth protecting (malformed
/// syntax fails closed) is the same, only the boundary of "malformed" moved.
#[test]
fn policy_accepts_two_keys_and_rejects_malformed_two_key_syntax() {
    let root = unique_temp_dir("trust-multikey");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let keys_dir = layout.maintainer_trust_keys_dir();
        assert!(std::fs::create_dir_all(&keys_dir).is_ok());
        assert!(
            std::fs::write(
                keys_dir.join("a.pub"),
                format!("{}\n", public_key_hex(&[1_u8; 32]))
            )
            .is_ok()
        );
        assert!(
            std::fs::write(
                keys_dir.join("b.pub"),
                format!("{}\n", public_key_hex(&[2_u8; 32]))
            )
            .is_ok()
        );
        assert!(
            std::fs::write(
                layout.trust_policy_path(),
                "[maintainer]\nrequired = 1\nkeys = [\"a\", \"b\"]\n",
            )
            .is_ok()
        );
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok(), "well-formed two-key policy must load");
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.keys.len(), 2);
        }

        for malformed in [
            "[maintainer]\nrequired = 1\nkeys = [\"a\",\"b\"]\n",
            "[maintainer]\nrequired = 1\nkeys = [\"a\", \"a\"]\n",
            "[maintainer]\nrequired = 1\nkeys = [\"a\", ]\n",
            "[maintainer]\nrequired = 1\nkeys = []\n",
        ] {
            assert!(std::fs::write(layout.trust_policy_path(), malformed).is_ok());
            assert!(
                load_maintainer_trust_policy(&layout).is_err(),
                "malformed policy must be rejected: {malformed:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn signer_trust_binding_rejects_seed_public_key_mismatch() {
    let root = unique_temp_dir("trust-seed-mismatch");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let trusted_seed = [7_u8; 32];
        let signing_seed = [8_u8; 32];
        let public_key = public_key_hex(&trusted_seed);
        assert!(add_trusted_maintainer(&layout, "maintainer", &public_key).is_ok());
        let signer = match Ed25519MaintainerSigner::from_seed("maintainer", &signing_seed) {
            Ok(signer) => signer,
            Err(error) => panic!("test maintainer signer should be constructible: {error}"),
        };
        assert!(verify_signer_trusted(&layout, &signer).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn failed_key_parent_sync_keeps_previous_effective_policy() {
    let root = unique_temp_dir("trust-key-sync-failure");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let old_key = public_key_hex(&[7_u8; 32]);
        let new_key = public_key_hex(&[8_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "old", &old_key).is_ok());
        fail_once_for_test(TestFailPoint::MutableParentSync);
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_err());
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.keys.len(), 1);
            assert_eq!(loaded.keys[0].key_id, "old");
        }
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
        let retried = load_maintainer_trust_policy(&layout);
        assert!(retried.is_ok());
        if let Ok(retried) = retried {
            assert_eq!(retried.keys.len(), 2);
            assert_eq!(retried.keys[0].key_id, "old");
            assert_eq!(retried.keys[1].key_id, "new");
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn failed_policy_parent_sync_exposes_retryable_new_effective_policy() {
    let root = unique_temp_dir("trust-policy-sync-failure");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let old_key = public_key_hex(&[7_u8; 32]);
        let new_key = public_key_hex(&[8_u8; 32]);
        assert!(add_trusted_maintainer(&layout, "old", &old_key).is_ok());
        // Fail on the *second* MutableParentSync this add triggers: the key file's own sync
        // succeeds (skip = 1 means the first call is allowed through), so "new"'s key file lands on
        // disk, and the policy rewrite's *rename* also completes — parent-directory sync happens
        // strictly after rename in `write_file_atomically`, so a sync failure there reports an
        // error without undoing the rename that already made "new" the effective policy content.
        // That is what "exposes retryable new effective policy" names: the failure is honestly
        // reported (durability wasn't confirmed), but the content already moved.
        fail_after_for_test(TestFailPoint::MutableParentSync, 1);
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_err());
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(
                loaded.keys.len(),
                2,
                "the policy rename already completed before the parent sync failed"
            );
            assert_eq!(loaded.keys[1].key_id, "new");
        }
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
        let retried = load_maintainer_trust_policy(&layout);
        assert!(retried.is_ok());
        if let Ok(retried) = retried {
            assert_eq!(retried.keys.len(), 2);
            assert_eq!(retried.keys[1].key_id, "new");
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn trust_reads_and_updates_remain_on_retained_repository_root() -> prikk_error::Result<()> {
    let root = unique_temp_dir("trust-root-replacement");
    let layout = RepositoryLayout::init(root.clone())?;
    let first = public_key_hex(&[7_u8; 32]);
    let second = public_key_hex(&[8_u8; 32]);
    assert!(add_trusted_maintainer(&layout, "maintainer", &first).is_ok());
    let displaced = root.join(".prikk-displaced");
    std::fs::rename(layout.prikk_dir(), &displaced)?;
    std::fs::create_dir_all(root.join(".prikk/trust/keys/maintainer"))?;
    std::fs::write(
        root.join(".prikk/trust/policy.toml"),
        "[maintainer]\nrequired = 1\nkeys = [\"replacement\"]\n",
    )?;

    assert_eq!(
        load_maintainer_trust_policy(&layout)?.keys[0].key_id,
        "maintainer"
    );
    assert!(add_trusted_maintainer(&layout, "second", &second).is_ok());
    let loaded = load_maintainer_trust_policy(&layout)?;
    assert_eq!(loaded.keys.len(), 2);
    assert_eq!(loaded.keys[1].key_id, "second");
    assert_eq!(
        loaded.keys[1].public_key,
        Ed25519KeyPair::from_seed(&[8_u8; 32]).public_key_bytes()
    );
    assert!(root.join(".prikk/trust/policy.toml").is_file());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn trust_file_sync_and_rename_failures_pin_effective_state_and_retry() -> prikk_error::Result<()> {
    for point in [TestFailPoint::MutableFileSync, TestFailPoint::MutableRename] {
        for (boundary, skip) in [("key", 0), ("policy", 1)] {
            let root = unique_temp_dir(&format!("trust-{boundary}-{point:?}"));
            let layout = RepositoryLayout::init(root.clone())?;
            let old_key = public_key_hex(&[7_u8; 32]);
            let new_key = public_key_hex(&[8_u8; 32]);
            assert!(add_trusted_maintainer(&layout, "old", &old_key).is_ok());

            fail_after_for_test(point, skip);
            assert!(add_trusted_maintainer(&layout, "new", &new_key).is_err());
            let retained = load_maintainer_trust_policy(&layout)?;
            assert_eq!(retained.keys.len(), 1, "boundary {boundary}");
            assert_eq!(retained.keys[0].key_id, "old");
            assert_eq!(
                retained.keys[0].public_key,
                Ed25519KeyPair::from_seed(&[7_u8; 32]).public_key_bytes()
            );
            assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
            let retried = load_maintainer_trust_policy(&layout)?;
            assert_eq!(retried.keys.len(), 2, "boundary {boundary}");
            assert_eq!(retried.keys[1].key_id, "new");

            let _ = std::fs::remove_dir_all(root);
        }
    }
    Ok(())
}
