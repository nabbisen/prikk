//! Trust-store tests.

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
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.key_id, "maintainer_1");
            assert_eq!(
                loaded.public_key,
                Ed25519KeyPair::from_seed(&seed).public_key_bytes()
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

#[test]
fn policy_rejects_multikey_shape() {
    let root = unique_temp_dir("trust-multikey");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(
            std::fs::write(
                layout.trust_policy_path(),
                "[maintainer]\nrequired = 1\nkeys = [\"a\", \"b\"]\n",
            )
            .is_ok()
        );
        assert!(load_maintainer_trust_policy(&layout).is_err());
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
            assert_eq!(loaded.key_id, "old");
        }
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
        let retried = load_maintainer_trust_policy(&layout);
        assert!(retried.is_ok());
        if let Ok(retried) = retried {
            assert_eq!(retried.key_id, "new");
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
        fail_after_for_test(TestFailPoint::MutableParentSync, 1);
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_err());
        let loaded = load_maintainer_trust_policy(&layout);
        assert!(loaded.is_ok());
        if let Ok(loaded) = loaded {
            assert_eq!(loaded.key_id, "new");
        }
        assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
        let retried = load_maintainer_trust_policy(&layout);
        assert!(retried.is_ok());
        if let Ok(retried) = retried {
            assert_eq!(retried.key_id, "new");
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn same_key_replacement_failure_exposes_retryable_replaced_key() -> prikk_error::Result<()> {
    let root = unique_temp_dir("trust-same-key-sync-failure");
    let layout = RepositoryLayout::init(root.clone())?;
    let old_key = public_key_hex(&[7_u8; 32]);
    let new_key = public_key_hex(&[8_u8; 32]);
    assert!(add_trusted_maintainer(&layout, "maintainer", &old_key).is_ok());

    fail_once_for_test(TestFailPoint::MutableParentSync);
    assert!(add_trusted_maintainer(&layout, "maintainer", &new_key).is_err());
    let loaded = load_maintainer_trust_policy(&layout)?;
    assert_eq!(
        loaded.public_key,
        Ed25519KeyPair::from_seed(&[8_u8; 32]).public_key_bytes()
    );
    assert!(add_trusted_maintainer(&layout, "maintainer", &new_key).is_ok());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn trust_reads_and_updates_remain_on_retained_repository_root() -> prikk_error::Result<()> {
    let root = unique_temp_dir("trust-root-replacement");
    let layout = RepositoryLayout::init(root.clone())?;
    let original = public_key_hex(&[7_u8; 32]);
    let updated = public_key_hex(&[8_u8; 32]);
    assert!(add_trusted_maintainer(&layout, "maintainer", &original).is_ok());
    let displaced = root.join(".prikk-displaced");
    std::fs::rename(layout.prikk_dir(), &displaced)?;
    std::fs::create_dir_all(root.join(".prikk/trust/keys/maintainer"))?;
    std::fs::write(
        root.join(".prikk/trust/policy.toml"),
        "[maintainer]\nrequired = 1\nkeys = [\"replacement\"]\n",
    )?;

    assert_eq!(load_maintainer_trust_policy(&layout)?.key_id, "maintainer");
    assert!(add_trusted_maintainer(&layout, "maintainer", &updated).is_ok());
    assert_eq!(
        load_maintainer_trust_policy(&layout)?.public_key,
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
            assert_eq!(retained.key_id, "old");
            assert_eq!(
                retained.public_key,
                Ed25519KeyPair::from_seed(&[7_u8; 32]).public_key_bytes()
            );
            assert!(add_trusted_maintainer(&layout, "new", &new_key).is_ok());
            assert_eq!(load_maintainer_trust_policy(&layout)?.key_id, "new");

            let _ = std::fs::remove_dir_all(root);
        }
    }
    Ok(())
}
