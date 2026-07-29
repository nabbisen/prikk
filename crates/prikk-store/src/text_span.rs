//! Shared FDD-01 §5.1 text-span identity primitives (DC-09 Phase 4.4-2c-4, carry-forward C1).
//!
//! These compute **identity-bearing** bytes for content-anchored text edits: the bounded left/right
//! anchor hashes, the `span_id`, the overlapping occurrence enumeration, the anchor-filtered span
//! localization (Option A), the post-localization splice, and the derived `BlobPayload(Text, …)`
//! content `ObjectId`. They are the single source of truth for both authoritative lifecycle replay
//! and (later) worktree authoring — the two MUST NOT compute any of these through separate
//! implementations, or they would drift on identity bytes.
//!
//! The full identity chain is, in stages (kept separable for testing and conformance vectors):
//!   1. localization: current text + `EditText` record → `(start, end)` ([`locate_text_span`]);
//!   2. splice:       `(text, start, end, replacement)` → `new_text` ([`splice_text`]);
//!   3. identity:     `new_text` → [`text_blob_id`].
//!
//! Conformance is pinned by golden vectors in [`vectors`].
//!
//! Split across three files (DC-58): this file keeps the identity primitives shared by both replay
//! and authoring (`locate_text_span`, `splice_text`, `text_blob_id`, `occurrences`,
//! `compute_span_id`, the anchor hashes); `authoring.rs` holds deterministic span selection for
//! worktree authoring; `inverse.rs` holds direct-inverse derivation. Both `pub(crate) use`
//! re-exported here so every existing `text_span::plan_authored_text_span` /
//! `text_span::derive_inverse_edit_text` caller is unaffected. No behaviour change.

use std::fmt;

use prikk_error::PrikkError;
use prikk_object::{BlobKind, BlobPayload, CanonicalEncode, NodeId, ObjectId, ObjectType};

mod authoring;
mod inverse;

pub(crate) use authoring::plan_authored_text_span;
pub(crate) use inverse::derive_inverse_edit_text;

/// Canonical anchor context window (FDD-01 §5.1 anchor-window clarification): up to 64 bytes of raw
/// text on each side of the span, byte-exact, no normalization.
pub(crate) const TEXT_ANCHOR_WINDOW: usize = 64;

/// Why anchor-filtered span localization failed (FDD-01 §5.1). Shared with replay (which wraps it
/// with `node_id`/`span_id` context) and, later, worktree authoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextSpanResolutionFailure {
    /// No occurrence's anchors matched the record's anchor hashes.
    AnchorMismatch,
    /// Anchors matched, but no anchor-filtered occurrence reproduced the record's `span_id`.
    NoMatchingSpanId,
    /// More than one occurrence reproduced the record's `span_id` (defensive; needs a collision).
    Ambiguous,
}

impl fmt::Display for TextSpanResolutionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AnchorMismatch => "no occurrence's anchors matched the record",
            Self::NoMatchingSpanId => "no anchor-filtered occurrence reproduced the span_id",
            Self::Ambiguous => "more than one occurrence reproduced the span_id",
        };
        f.write_str(s)
    }
}

/// Invalid byte range handed to [`splice_text`]. `locate_text_span` never produces one; the guard
/// exists for the later authoring caller (E1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSpanSpliceError {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text_len: usize,
}

impl fmt::Display for TextSpanSpliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid text-span splice range start={} end={} for text of length {}",
            self.start, self.end, self.text_len
        )
    }
}

impl std::error::Error for TextSpanSpliceError {}

/// `ObjectId` of the canonical `BlobPayload(Text, content)` — the content identity recorded for a
/// text node (FDD-03 §10.2 `node_payload`). Uses the existing blob encoding; no new identity bytes.
pub(crate) fn text_blob_id(content: &[u8]) -> Result<ObjectId, PrikkError> {
    let payload = BlobPayload::new(BlobKind::Text, content.to_vec());
    let bytes = payload.to_canonical_bytes()?;
    Ok(ObjectId::from_canonical_payload(
        ObjectType::Blob,
        1,
        &bytes,
    ))
}

/// SHA-256 of the bounded left context (up to 64 bytes preceding `start`), FDD-01 §5.1.
pub(crate) fn left_anchor(text: &[u8], start: usize) -> [u8; 32] {
    let lo = start.saturating_sub(TEXT_ANCHOR_WINDOW);
    anchor_hash(
        b"PRIKK-TEXT-LEFT-ANCHOR-v1",
        text.get(lo..start).unwrap_or(&[]),
    )
}

/// SHA-256 of the bounded right context (up to 64 bytes following `end`), FDD-01 §5.1.
pub(crate) fn right_anchor(text: &[u8], end: usize) -> [u8; 32] {
    let hi = end.saturating_add(TEXT_ANCHOR_WINDOW).min(text.len());
    anchor_hash(
        b"PRIKK-TEXT-RIGHT-ANCHOR-v1",
        text.get(end..hi).unwrap_or(&[]),
    )
}

fn anchor_hash(domain: &[u8], context: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(domain.len() + 4 + context.len());
    preimage.extend_from_slice(domain);
    preimage.extend_from_slice(&(context.len() as u32).to_be_bytes());
    preimage.extend_from_slice(context);
    prikk_hash::sha256(&preimage)
}

/// `span_id` per FDD-01 §5.1.
pub(crate) fn compute_span_id(
    node_id: NodeId,
    old_span_hash: &[u8; 32],
    left: &[u8; 32],
    right: &[u8; 32],
    dup_index: u32,
) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(18 + 32 * 4 + 4);
    preimage.extend_from_slice(b"PRIKK-TEXT-SPAN-v1");
    preimage.extend_from_slice(node_id.as_bytes());
    preimage.extend_from_slice(old_span_hash);
    preimage.extend_from_slice(left);
    preimage.extend_from_slice(right);
    preimage.extend_from_slice(&dup_index.to_be_bytes());
    prikk_hash::sha256(&preimage)
}

/// Canonical-order occurrences of `needle` in `text` (overlapping). For an empty needle, the
/// insertion positions `0..=len`.
pub(crate) fn occurrences(text: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() {
        return (0..=text.len()).collect();
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= text.len() {
        if text.get(i..i + needle.len()) == Some(needle) {
            out.push(i);
        }
        i += 1;
    }
    out
}

/// Localize an `EditText` span in `text` (FDD-01 §5.1, Option A): among canonical-order
/// occurrences of `old_span_text` whose 64-byte anchor hashes match the record, find the one whose
/// recomputed `span_id` (using its zero-based index within that anchor-filtered list) equals the
/// record's. Requires exactly one. The stored `span_id` is recomputed, never trusted directly.
#[allow(clippy::too_many_arguments)]
pub(crate) fn locate_text_span(
    text: &[u8],
    old_span_text: &[u8],
    record_left: &[u8; 32],
    record_right: &[u8; 32],
    record_span_id: &[u8; 32],
    node_id: NodeId,
    old_span_hash: &[u8; 32],
) -> Result<(usize, usize), TextSpanResolutionFailure> {
    let span_len = old_span_text.len();
    let anchor_matching: Vec<(usize, usize)> = occurrences(text, old_span_text)
        .into_iter()
        .map(|start| (start, start + span_len))
        .filter(|&(start, end)| {
            left_anchor(text, start) == *record_left && right_anchor(text, end) == *record_right
        })
        .collect();

    if anchor_matching.is_empty() {
        return Err(TextSpanResolutionFailure::AnchorMismatch);
    }

    let mut matches = Vec::new();
    for (dup_index, &(start, end)) in anchor_matching.iter().enumerate() {
        let sid = compute_span_id(
            node_id,
            old_span_hash,
            record_left,
            record_right,
            dup_index as u32,
        );
        if sid == *record_span_id {
            matches.push((start, end));
        }
    }

    match matches.as_slice() {
        [] => Err(TextSpanResolutionFailure::NoMatchingSpanId),
        [one] => Ok(*one),
        _ => Err(TextSpanResolutionFailure::Ambiguous),
    }
}

/// Splice `replacement` into `text` over the located byte range `[start, end)`, returning
/// `new_text = text[..start] ‖ replacement ‖ text[end..]`. Rejects an invalid range (E1) rather
/// than clamping or panicking. The output bytes are the input to [`text_blob_id`], so this is the
/// single shared splice both replay and authoring must use.
pub(crate) fn splice_text(
    text: &[u8],
    start: usize,
    end: usize,
    replacement: &[u8],
) -> Result<Vec<u8>, TextSpanSpliceError> {
    if start > end || end > text.len() {
        return Err(TextSpanSpliceError {
            start,
            end,
            text_len: text.len(),
        });
    }
    let mut new_text = Vec::with_capacity(text.len() - (end - start) + replacement.len());
    new_text.extend_from_slice(text.get(..start).unwrap_or(&[]));
    new_text.extend_from_slice(replacement);
    new_text.extend_from_slice(text.get(end..).unwrap_or(&[]));
    Ok(new_text)
}

#[cfg(test)]
mod vectors;
