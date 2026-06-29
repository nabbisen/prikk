//! Deterministic canonical encoding helpers.
//!
//! The encoding is a minimal TLV seed for Prikk identity bytes. Field tags are local to each
//! payload definition. This module is intentionally small and explicit: callers must encode fields
//! in the order defined by their payload contracts.

use prikk_error::{PrikkError, Result};

/// Canonical field value type — FDD-03 §7.1 `value_type`. (The Rust type keeps the
/// historical name `WireType`; its codes and semantics are the normative
/// §7.1 table.) The `u8` code is part of every field record and therefore part of
/// object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WireType {
    /// Boolean as one byte, 0 or 1.
    Bool = 0x01,
    /// Unsigned 16-bit integer, big-endian.
    U16 = 0x02,
    /// Unsigned 32-bit integer, big-endian.
    U32 = 0x03,
    /// Unsigned 64-bit integer, big-endian.
    U64 = 0x04,
    /// Discriminated `enum_u16` value, big-endian.
    EnumU16 = 0x05,
    /// UTF-8 string bytes.
    String = 0x10,
    /// Opaque byte string.
    Bytes = 0x11,
    /// 32-byte object identifier.
    ObjectId = 0x12,
    /// Normalized repo-relative UTF-8 path.
    RepoPath = 0x13,
    /// Nested canonical record bytes.
    Record = 0x20,
    /// Item of a repeated record list.
    RecordListItem = 0x21,
}

/// Trait for values that can emit Prikk canonical bytes.
pub trait CanonicalEncode {
    /// Encode this value into canonical bytes.
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()>;

    /// Return canonical bytes for this value.
    fn to_canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut writer = CanonicalWriter::new();
        self.encode_canonical(&mut writer)?;
        Ok(writer.finish())
    }
}

/// Canonical TLV writer.
#[derive(Debug, Default)]
pub struct CanonicalWriter {
    bytes: Vec<u8>,
    last_tag: Option<u16>,
}

impl CanonicalWriter {
    /// Create an empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Finish and return the canonical byte buffer.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    /// Emit a string field.
    pub fn field_string(&mut self, tag: u16, value: &str) -> Result<()> {
        self.field_raw(tag, WireType::String, value.as_bytes())
    }

    /// Emit an optional string field.
    pub fn field_string_opt(&mut self, tag: u16, value: Option<&str>) -> Result<()> {
        if let Some(value) = value {
            self.field_string(tag, value)?;
        }
        Ok(())
    }

    /// Emit a bytes field.
    pub fn field_bytes(&mut self, tag: u16, value: &[u8]) -> Result<()> {
        self.field_raw(tag, WireType::Bytes, value)
    }

    /// Emit a u32 field.
    pub fn field_u32(&mut self, tag: u16, value: u32) -> Result<()> {
        self.field_raw(tag, WireType::U32, &value.to_be_bytes())
    }

    /// Emit a u64 field.
    pub fn field_u64(&mut self, tag: u16, value: u64) -> Result<()> {
        self.field_raw(tag, WireType::U64, &value.to_be_bytes())
    }

    /// Emit a bool field.
    pub fn field_bool(&mut self, tag: u16, value: bool) -> Result<()> {
        let encoded = if value { [1_u8] } else { [0_u8] };
        self.field_raw(tag, WireType::Bool, &encoded)
    }

    /// Emit a u16 field.
    pub fn field_u16(&mut self, tag: u16, value: u16) -> Result<()> {
        self.field_raw(tag, WireType::U16, &value.to_be_bytes())
    }

    /// Emit an `enum_u16` field (discriminated enum value).
    pub fn field_enum_u16(&mut self, tag: u16, value: u16) -> Result<()> {
        self.field_raw(tag, WireType::EnumU16, &value.to_be_bytes())
    }

    /// Emit an object-id field (32 bytes, `value_type` `object_id`).
    pub fn field_object_id(&mut self, tag: u16, value: &crate::ObjectId) -> Result<()> {
        self.field_raw(tag, WireType::ObjectId, value.as_bytes())
    }

    /// Emit a repo-path field. The caller passes an already-normalized
    /// repo-relative UTF-8 path.
    pub fn field_repo_path(&mut self, tag: u16, value: &str) -> Result<()> {
        self.field_raw(tag, WireType::RepoPath, value.as_bytes())
    }

    /// Emit a nested canonical record.
    pub fn field_record<T: CanonicalEncode>(&mut self, tag: u16, value: &T) -> Result<()> {
        let mut nested = CanonicalWriter::new();
        value.encode_canonical(&mut nested)?;
        self.field_raw(tag, WireType::Record, &nested.finish())
    }

    /// Emit a repeated nested record field. Each item is emitted with the same tag.
    pub fn repeated_record<T: CanonicalEncode>(&mut self, tag: u16, values: &[T]) -> Result<()> {
        for value in values {
            self.field_record(tag, value)?;
        }
        Ok(())
    }

    /// Emit a single record-list item (`value_type` `record_list_item`).
    pub fn field_record_list_item<T: CanonicalEncode>(
        &mut self,
        tag: u16,
        value: &T,
    ) -> Result<()> {
        let mut nested = CanonicalWriter::new();
        value.encode_canonical(&mut nested)?;
        self.field_raw(tag, WireType::RecordListItem, &nested.finish())
    }

    /// Emit a repeated record list using `record_list_item` framing. Each item is
    /// emitted with the same tag.
    pub fn repeated_record_list<T: CanonicalEncode>(
        &mut self,
        tag: u16,
        values: &[T],
    ) -> Result<()> {
        for value in values {
            self.field_record_list_item(tag, value)?;
        }
        Ok(())
    }

    /// Emit a repeated string field. Each item is emitted with the same tag.
    pub fn repeated_string(&mut self, tag: u16, values: &[String]) -> Result<()> {
        for value in values {
            self.field_string(tag, value)?;
        }
        Ok(())
    }

    /// Emit a repeated object-id field. Each item is emitted with the same tag,
    /// using the `object_id` value_type (FDD-03 §7.1).
    pub fn repeated_object_id(&mut self, tag: u16, values: &[crate::ObjectId]) -> Result<()> {
        for value in values {
            self.field_object_id(tag, value)?;
        }
        Ok(())
    }

    /// Emit a raw field.
    pub fn field_raw(&mut self, tag: u16, wire_type: WireType, value: &[u8]) -> Result<()> {
        if tag == 0 {
            return Err(PrikkError::CanonicalEncoding(
                "field tag 0 is reserved".to_string(),
            ));
        }
        if let Some(last) = self.last_tag {
            if tag < last {
                return Err(PrikkError::CanonicalEncoding(format!(
                    "field tag order violation: {tag} after {last}"
                )));
            }
        }
        self.last_tag = Some(tag);
        self.bytes.extend_from_slice(&tag.to_be_bytes());
        self.bytes.push(wire_type as u8);
        self.bytes
            .extend_from_slice(&(value.len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(value);
        Ok(())
    }
}

/// Return true if the slice is strictly sorted and contains no duplicates.
#[must_use]
pub fn is_strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| {
        let mut items = pair.iter();
        match (items.next(), items.next()) {
            (Some(left), Some(right)) => left < right,
            _ => true,
        }
    })
}

/// Return true if a sequence is exactly `1..=n`.
#[must_use]
pub fn is_contiguous_op_seq(values: &[u32]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(idx, value)| *value as usize == idx + 1)
}

#[cfg(test)]
mod tests {
    use super::CanonicalWriter;

    #[test]
    fn rejects_decreasing_tags() {
        let mut writer = CanonicalWriter::new();
        assert!(writer.field_u32(2, 1).is_ok());
        assert!(writer.field_u32(1, 1).is_err());
    }
}
