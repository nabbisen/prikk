//! Deterministic direct-inverse derivation for one supported arbitrary-span `EditText`. Split out
//! of `text_span.rs` (DC-58) — no behaviour change, all items moved verbatim. Re-exported at
//! `text_span.rs` so every existing `text_span::derive_inverse_edit_text` caller is unaffected.

use prikk_error::PrikkError;
use prikk_object::{EditText, NodeId, text_span_hash};

use super::authoring::anchor_filtered_dup_index;
use super::{compute_span_id, left_anchor, locate_text_span, right_anchor, splice_text};

/// Deterministically derive the direct inverse of one supported arbitrary-span [`EditText`].
///
/// The inverse identity is computed against the post-forward text. The derived inverse is then
/// localized back against that same post-forward text and applied, requiring exact byte recovery of
/// the pre-forward text.
#[allow(clippy::too_many_arguments)]
pub(crate) fn derive_inverse_edit_text(
    pre_text: &[u8],
    node_id: NodeId,
    span_id: &[u8; 32],
    old_span_hash: &[u8; 32],
    left_anchor_hash: &[u8; 32],
    right_anchor_hash: &[u8; 32],
    replacement_text: &[u8],
    old_span_text: &[u8],
) -> Result<(EditText, Vec<u8>), PrikkError> {
    let pre_utf8 = core::str::from_utf8(pre_text)
        .map_err(|_| PrikkError::Integrity("EditText inverse pre-text is not UTF-8".to_string()))?;
    core::str::from_utf8(old_span_text).map_err(|_| {
        PrikkError::Integrity("EditText inverse old_span_text is not UTF-8".to_string())
    })?;
    core::str::from_utf8(replacement_text).map_err(|_| {
        PrikkError::Integrity("EditText inverse replacement_text is not UTF-8".to_string())
    })?;
    if text_span_hash(old_span_text) != *old_span_hash {
        return Err(PrikkError::Integrity(
            "EditText inverse old_span_hash does not match old_span_text".to_string(),
        ));
    }
    let (start, end) = locate_text_span(
        pre_text,
        old_span_text,
        left_anchor_hash,
        right_anchor_hash,
        span_id,
        node_id,
        old_span_hash,
    )
    .map_err(|reason| {
        PrikkError::Integrity(format!(
            "EditText inverse could not localize forward span: {reason}"
        ))
    })?;
    if !pre_utf8.is_char_boundary(start) || !pre_utf8.is_char_boundary(end) {
        return Err(PrikkError::Integrity(
            "EditText inverse forward span is not on UTF-8 byte boundaries".to_string(),
        ));
    }
    let post_text = splice_text(pre_text, start, end, replacement_text).map_err(|err| {
        PrikkError::Integrity(format!("EditText inverse forward splice failed: {err}"))
    })?;
    let post_utf8 = core::str::from_utf8(&post_text).map_err(|_| {
        PrikkError::Integrity("EditText inverse post-forward text is not UTF-8".to_string())
    })?;

    let inverse_start = start;
    let inverse_end = inverse_start
        .checked_add(replacement_text.len())
        .ok_or_else(|| PrikkError::Integrity("EditText inverse range overflow".to_string()))?;
    if inverse_end > post_text.len()
        || !post_utf8.is_char_boundary(inverse_start)
        || !post_utf8.is_char_boundary(inverse_end)
    {
        return Err(PrikkError::Integrity(
            "EditText inverse replacement range is not on UTF-8 byte boundaries".to_string(),
        ));
    }

    let inverse_old_span_text = replacement_text.to_vec();
    let inverse_replacement_text = old_span_text.to_vec();
    let inverse_old_span_hash = text_span_hash(&inverse_old_span_text);
    let inverse_left_anchor_hash = left_anchor(&post_text, inverse_start);
    let inverse_right_anchor_hash = right_anchor(&post_text, inverse_end);
    let inverse_dup_index = anchor_filtered_dup_index(
        &post_text,
        &inverse_old_span_text,
        inverse_start,
        inverse_end,
        &inverse_left_anchor_hash,
        &inverse_right_anchor_hash,
    )
    .map_err(|err| {
        PrikkError::Integrity(format!(
            "EditText inverse duplicate index could not be derived: {err}"
        ))
    })?;
    let inverse_span_id = compute_span_id(
        node_id,
        &inverse_old_span_hash,
        &inverse_left_anchor_hash,
        &inverse_right_anchor_hash,
        inverse_dup_index,
    );
    let inverse = EditText {
        node_id,
        span_id: inverse_span_id,
        old_span_hash: inverse_old_span_hash,
        left_anchor_hash: inverse_left_anchor_hash,
        right_anchor_hash: inverse_right_anchor_hash,
        replacement_text: inverse_replacement_text,
        presentation_hint_line: None,
        presentation_hint_column: None,
        old_span_text: inverse_old_span_text,
    };
    let located_inverse = locate_text_span(
        &post_text,
        &inverse.old_span_text,
        &inverse.left_anchor_hash,
        &inverse.right_anchor_hash,
        &inverse.span_id,
        node_id,
        &inverse.old_span_hash,
    )
    .map_err(|reason| {
        PrikkError::Integrity(format!(
            "EditText inverse could not re-localize derived inverse: {reason}"
        ))
    })?;
    if located_inverse != (inverse_start, inverse_end) {
        return Err(PrikkError::Integrity(
            "EditText inverse localized to a different post-forward range".to_string(),
        ));
    }
    let recovered = splice_text(
        &post_text,
        located_inverse.0,
        located_inverse.1,
        &inverse.replacement_text,
    )
    .map_err(|err| {
        PrikkError::Integrity(format!("EditText inverse recovery splice failed: {err}"))
    })?;
    if recovered != pre_text {
        return Err(PrikkError::Integrity(
            "EditText inverse did not recover pre-forward text".to_string(),
        ));
    }
    Ok((inverse, post_text))
}
