use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JsonErrorKind {
    DuplicateName,
    Syntax,
}

#[derive(Debug)]
pub(crate) struct JsonError {
    // RFC 119 track B: `policy.rs`'s "json-parser" oracle suite -- the only caller that branched
    // on this field -- was removed (NEVER: it exercised only this module's own input handling on
    // files this project authors, not release policy). `kind` itself stays: it is `parse`'s own
    // classification of failure modes, exercised directly by `json/tests.rs`, independent of
    // whether any current caller consumes it -- this track's handoff is explicit that removing an
    // oracle suite is not the same as removing the module it tested.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) kind: JsonErrorKind,
    message: String,
}

impl Display for JsonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Value, JsonError> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(JsonError {
            kind: JsonErrorKind::Syntax,
            message: "JSON byte-order mark is forbidden".to_owned(),
        });
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictValue
        .deserialize(&mut deserializer)
        .map_err(classify_error)?;
    deserializer.end().map_err(classify_error)?;
    Ok(value)
}

fn classify_error(error: serde_json::Error) -> JsonError {
    let message = error.to_string();
    let kind = if message.contains("duplicate JSON object name") {
        JsonErrorKind::DuplicateName
    } else {
        JsonErrorKind::Syntax
    };
    JsonError { kind, message }
}

struct StrictValue;

impl<'de> DeserializeSeed<'de> for StrictValue {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValue)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut names = HashSet::new();
        while let Some(name) = object.next_key::<String>()? {
            if !names.insert(name.clone()) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object name: {name}"
                )));
            }
            values.insert(name, object.next_value_seed(StrictValue)?);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
#[path = "json/tests.rs"]
mod tests;
