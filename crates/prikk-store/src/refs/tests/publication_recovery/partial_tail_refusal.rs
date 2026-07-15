//! Divergent complete prefixes must never authorize partial-tail truncation.

use std::io::Write;

use prikk_object::RefUpdatePayload;

use super::root_publication;
use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::{sample_object_id, signed_ref_update_envelope, unique_temp_dir};
use crate::{RefStore, RepositoryLayout};

#[derive(Clone, Copy)]
enum Divergence {
    WrongRef,
    WrongOldLink,
    WrongSequence,
    DuplicateRecord,
    ByteDifferentExpected,
}

#[test]
fn divergent_complete_prefix_with_partial_tail_is_preserved_byte_for_byte()
-> prikk_error::Result<()> {
    for divergence in [
        Divergence::WrongRef,
        Divergence::WrongOldLink,
        Divergence::WrongSequence,
        Divergence::DuplicateRecord,
        Divergence::ByteDifferentExpected,
    ] {
        let root = unique_temp_dir("dc38-divergent-partial-tail");
        let layout = RepositoryLayout::init(root.clone())?;
        let publication = root_publication(&layout, "heads/main")?;
        let store = RefStore::new(layout.clone());
        fail_once_for_test(TestFailPoint::PromotionDestinationSync);
        assert!(store.publish(&publication).is_err());

        let proposed = publication.ref_state.object_id();
        let target = RefUpdatePayload::decode_canonical(&publication.ref_update.canonical_payload)?
            .new_target_object_id;
        let divergent = match divergence {
            Divergence::WrongRef => {
                signed_ref_update_envelope("heads/topic", None, proposed, target, 1)
            }
            Divergence::WrongOldLink => signed_ref_update_envelope(
                "heads/main",
                Some(sample_object_id("wrong-old-link")),
                proposed,
                target,
                1,
            ),
            Divergence::WrongSequence => {
                signed_ref_update_envelope("heads/main", None, proposed, target, 2)
            }
            Divergence::DuplicateRecord => publication.ref_update.clone(),
            Divergence::ByteDifferentExpected => {
                let mut envelope = publication.ref_update.clone();
                let byte = envelope
                    .signatures
                    .first_mut()
                    .and_then(|signature| signature.signature_bytes.first_mut())
                    .ok_or_else(|| {
                        prikk_error::PrikkError::Integrity(
                            "expected RefUpdate signature bytes".to_string(),
                        )
                    })?;
                *byte ^= 0xff;
                envelope
            }
        };
        super::super::super::log::append_log_record(&layout, "heads/main", &divergent)?;
        if matches!(divergence, Divergence::DuplicateRecord) {
            let bytes = std::fs::read(layout.ref_log_path("heads/main"))?;
            std::fs::OpenOptions::new()
                .append(true)
                .open(layout.ref_log_path("heads/main"))?
                .write_all(&bytes)?;
        }
        std::fs::OpenOptions::new()
            .append(true)
            .open(layout.ref_log_path("heads/main"))?
            .write_all(b"PREF")?;
        let before = std::fs::read(layout.ref_log_path("heads/main"))?;

        assert!(
            store
                .finish_interrupted_publication_for_test(&publication)
                .is_err()
        );
        assert_eq!(std::fs::read(layout.ref_log_path("heads/main"))?, before);
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}
