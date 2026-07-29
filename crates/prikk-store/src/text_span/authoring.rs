//! Deterministic arbitrary-span text-edit authoring: selecting and identifying one span from an
//! old/new byte-content diff. Split out of `text_span.rs` (DC-58) — no behaviour change, all items
//! moved verbatim. Re-exported at `text_span.rs` so every existing `text_span::plan_authored_text_span`
//! caller is unaffected.

use std::fmt;

use prikk_object::NodeId;

use super::{compute_span_id, left_anchor, locate_text_span, right_anchor};
use prikk_object::text_span_hash;

/// A deterministic authoring plan for one span-anchored text edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoredTextSpan {
    pub(crate) old_start: usize,
    pub(crate) old_end: usize,
    pub(crate) new_start: usize,
    pub(crate) new_end: usize,
    pub(crate) old_span_text: Vec<u8>,
    pub(crate) replacement_text: Vec<u8>,
    pub(crate) old_span_hash: [u8; 32],
    pub(crate) left_anchor_hash: [u8; 32],
    pub(crate) right_anchor_hash: [u8; 32],
    pub(crate) dup_index: u32,
    pub(crate) span_id: [u8; 32],
}

/// Why deterministic text-span authoring failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextSpanSelectionError {
    OldTextNotUtf8,
    NewTextNotUtf8,
    InvalidWidenedRange,
    SelectedRangeNotFound,
    DuplicateIndexOverflow,
}

impl fmt::Display for TextSpanSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::OldTextNotUtf8 => "old text is not well-formed UTF-8",
            Self::NewTextNotUtf8 => "new text is not well-formed UTF-8",
            Self::InvalidWidenedRange => "widened text-span range is invalid",
            Self::SelectedRangeNotFound => {
                "selected range was not found in the anchor-filtered occurrence list"
            }
            Self::DuplicateIndexOverflow => "anchor-filtered duplicate index exceeds u32",
        };
        f.write_str(s)
    }
}

impl std::error::Error for TextSpanSelectionError {}

/// Select and identify one deterministic arbitrary text span for authoring.
///
/// The selected span starts from byte-level LCP/LCS, then widens to enclosing UTF-8 character
/// boundaries. All identity-bearing bytes are computed here so authoring has no local anchor,
/// hash, duplicate-index, or span-id logic.
pub(crate) fn plan_authored_text_span(
    old: &[u8],
    new: &[u8],
    node_id: NodeId,
) -> Result<Option<AuthoredTextSpan>, TextSpanSelectionError> {
    if old == new {
        return Ok(None);
    }
    let old_text = core::str::from_utf8(old).map_err(|_| TextSpanSelectionError::OldTextNotUtf8)?;
    let new_text = core::str::from_utf8(new).map_err(|_| TextSpanSelectionError::NewTextNotUtf8)?;

    let prefix = common_prefix_len(old, new);
    let suffix = common_suffix_len(old, new, prefix);
    let mut old_start = prefix;
    let mut new_start = prefix;
    let mut old_end = old.len() - suffix;
    let mut new_end = new.len() - suffix;

    while !old_text.is_char_boundary(old_start) {
        old_start = old_start
            .checked_sub(1)
            .ok_or(TextSpanSelectionError::InvalidWidenedRange)?;
        new_start = new_start
            .checked_sub(1)
            .ok_or(TextSpanSelectionError::InvalidWidenedRange)?;
    }
    while !old_text.is_char_boundary(old_end) {
        old_end = old_end
            .checked_add(1)
            .ok_or(TextSpanSelectionError::InvalidWidenedRange)?;
        new_end = new_end
            .checked_add(1)
            .ok_or(TextSpanSelectionError::InvalidWidenedRange)?;
        if old_end > old.len() || new_end > new.len() {
            return Err(TextSpanSelectionError::InvalidWidenedRange);
        }
    }

    if old_start > old_end
        || old_end > old.len()
        || new_start > new_end
        || new_end > new.len()
        || !old_text.is_char_boundary(old_start)
        || !old_text.is_char_boundary(old_end)
        || !new_text.is_char_boundary(new_start)
        || !new_text.is_char_boundary(new_end)
    {
        return Err(TextSpanSelectionError::InvalidWidenedRange);
    }

    let old_span_text = old
        .get(old_start..old_end)
        .ok_or(TextSpanSelectionError::InvalidWidenedRange)?
        .to_vec();
    let replacement_text = new
        .get(new_start..new_end)
        .ok_or(TextSpanSelectionError::InvalidWidenedRange)?
        .to_vec();
    let left_anchor_hash = left_anchor(old, old_start);
    let right_anchor_hash = right_anchor(old, old_end);
    let old_span_hash = text_span_hash(&old_span_text);
    let dup_index = anchor_filtered_dup_index(
        old,
        &old_span_text,
        old_start,
        old_end,
        &left_anchor_hash,
        &right_anchor_hash,
    )?;
    let span_id = compute_span_id(
        node_id,
        &old_span_hash,
        &left_anchor_hash,
        &right_anchor_hash,
        dup_index,
    );

    debug_assert_eq!(
        locate_text_span(
            old,
            &old_span_text,
            &left_anchor_hash,
            &right_anchor_hash,
            &span_id,
            node_id,
            &old_span_hash,
        ),
        Ok((old_start, old_end))
    );

    Ok(Some(AuthoredTextSpan {
        old_start,
        old_end,
        new_start,
        new_end,
        old_span_text,
        replacement_text,
        old_span_hash,
        left_anchor_hash,
        right_anchor_hash,
        dup_index,
        span_id,
    }))
}

fn common_prefix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(a, b)| a == b)
        .count()
}

fn common_suffix_len(left: &[u8], right: &[u8], prefix: usize) -> usize {
    let max = left.len().min(right.len()).saturating_sub(prefix);
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take(max)
        .take_while(|(a, b)| a == b)
        .count()
}

/// Shared with `text_span::inverse`, which derives an inverse edit's duplicate index the same way.
pub(super) fn anchor_filtered_dup_index(
    text: &[u8],
    old_span_text: &[u8],
    selected_start: usize,
    selected_end: usize,
    left: &[u8; 32],
    right: &[u8; 32],
) -> Result<u32, TextSpanSelectionError> {
    let mut dup_index = 0_u32;
    let span_len = old_span_text.len();
    for start in super::occurrences(text, old_span_text) {
        let end = start + span_len;
        if left_anchor(text, start) == *left && right_anchor(text, end) == *right {
            if start == selected_start && end == selected_end {
                return Ok(dup_index);
            }
            dup_index = dup_index
                .checked_add(1)
                .ok_or(TextSpanSelectionError::DuplicateIndexOverflow)?;
        }
    }
    Err(TextSpanSelectionError::SelectedRangeNotFound)
}
