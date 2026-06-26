//! WAL tests.

use crate::{RepositoryLayout, Wal};

use super::helpers::{signed_patch_envelope, unique_temp_dir};

#[test]
fn wal_roundtrips_signed_patch_envelope() {
    let root = unique_temp_dir("wal");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        let envelope = signed_patch_envelope();
        let seq = wal.append_patch(&envelope);
        assert_eq!(seq, Ok(1));
        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            assert_eq!(replay.trailing_partial_bytes, 0);
            assert_eq!(replay.records.len(), 1);
            let first = replay.records.first();
            assert!(first.is_some());
            if let Some(first) = first {
                assert_eq!(first.seq, 1);
                assert_eq!(first.envelope, envelope);
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn wal_rejects_unsigned_patch_envelope() {
    let root = unique_temp_dir("wal-unsigned");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        let mut envelope = signed_patch_envelope();
        envelope.signatures.clear();
        let result = wal.append_patch(&envelope);
        assert!(result.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}
