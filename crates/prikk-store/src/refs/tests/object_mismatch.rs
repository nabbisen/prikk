//! DC-36 publication refusal when immutable object bytes disagree.

use crate::file_codec::encode_envelope_file;
use crate::test_support::{
    signed_empty_block_envelope, signed_ref_state_envelope, signed_ref_update_envelope,
    unique_temp_dir,
};
use crate::{FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout};

#[test]
fn ref_publication_does_not_advance_after_existing_object_mismatch() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-object-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
    let ref_state_id = ref_state.object_id();
    let mut different_transport = ref_state.clone();
    let signature_byte = different_transport
        .signatures
        .first_mut()
        .and_then(|signature| signature.signature_bytes.first_mut())
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity("missing test signature byte".to_string())
        })?;
    *signature_byte ^= 0xff;
    assert_eq!(different_transport.object_id(), ref_state_id);
    let path = layout.object_path(ref_state.object_type, ref_state_id);
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?,
    )?;
    std::fs::write(&path, encode_envelope_file(&different_transport)?)?;

    let publication = RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
        ref_state,
    };
    let refs = RefStore::new(layout.clone());
    assert!(refs.publish(&publication).is_err());
    assert_eq!(refs.read_current_ref_state_id("heads/main")?, None);
    assert!(refs.replay_log("heads/main")?.records.is_empty());
    assert_eq!(
        std::fs::read(path)?,
        encode_envelope_file(&different_transport)?
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
