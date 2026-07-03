//! Trust-store tests.

use prikk_crypto::Ed25519KeyPair;

use crate::{
    RepositoryLayout, add_trusted_maintainer, load_maintainer_trust_policy, verify_signer_trusted,
};

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
        let signer = Ed25519MaintainerSigner::from_seed("maintainer", &signing_seed);
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
