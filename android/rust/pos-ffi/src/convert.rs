//! Conversion between engine types and FFI records, through `serde_json::Value`.
//!
//! The FFI records mirror the engine's serde shapes (same field names after the rename rules), so
//! a conversion is a serialisation round trip. A shape that no longer matches is a contract
//! failure: it is logged and reported as [`PosError::Internal`], never a panic.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::PosError;

fn contract_error(target: &str, err: impl std::fmt::Display) -> PosError {
    let detail = format!("contract: {target}: {err}");
    tracing::error!(%detail, "FFI contract failure");
    PosError::Internal { detail }
}

/// Engine value to FFI record.
pub(crate) fn out<E: Serialize, F: DeserializeOwned>(v: &E) -> Result<F, PosError> {
    let target = std::any::type_name::<F>();
    let value = serde_json::to_value(v).map_err(|e| contract_error(target, e))?;
    serde_json::from_value(value).map_err(|e| contract_error(target, e))
}

/// FFI record to engine value.
pub(crate) fn inp<F: Serialize, E: DeserializeOwned>(v: &F) -> Result<E, PosError> {
    let target = std::any::type_name::<E>();
    let value = serde_json::to_value(v).map_err(|e| contract_error(target, e))?;
    serde_json::from_value(value).map_err(|e| contract_error(target, e))
}

/// `#[serde(with = "json_text")]` on a `String` field of an FFI record: the engine side holds a
/// `serde_json::Value`, the record holds its JSON text (`UniFFI` has no JSON value type).
pub mod json_text {
    use serde::de::Error as _;
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Record to engine: the text is parsed and written as a nested JSON value.
    pub fn serialize<S: Serializer>(text: &str, s: S) -> Result<S::Ok, S::Error> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(S::Error::custom)?;
        value.serialize(s)
    }

    /// Engine to record: the nested JSON value is rendered as text.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        serde_json::to_string(&value).map_err(D::Error::custom)
    }
}

/// Same as [`json_text`] for an `Option<String>` field (`null` is `None`).
pub mod json_text_opt {
    use serde::de::Error as _;
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(text: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        match text {
            None => s.serialize_none(),
            Some(text) => {
                let value: serde_json::Value =
                    serde_json::from_str(text).map_err(S::Error::custom)?;
                value.serialize(s)
            }
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        match Option::<serde_json::Value>::deserialize(d)? {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => serde_json::to_string(&value)
                .map(Some)
                .map_err(D::Error::custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Engine {
        id: i64,
        extra: serde_json::Value,
        note: Option<serde_json::Value>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Record {
        id: i64,
        #[serde(with = "json_text")]
        extra: String,
        #[serde(with = "json_text_opt")]
        note: Option<String>,
    }

    #[test]
    fn out_and_inp_round_trip_a_json_value_as_text() {
        let engine = Engine {
            id: 4,
            extra: serde_json::json!({"a": [1, 2]}),
            note: Some(serde_json::json!("x")),
        };
        let record: Record = out(&engine).unwrap();
        assert_eq!(
            record,
            Record {
                id: 4,
                extra: r#"{"a":[1,2]}"#.into(),
                note: Some(r#""x""#.into()),
            }
        );
        let back: Engine = inp(&record).unwrap();
        assert_eq!(back.extra, serde_json::json!({"a": [1, 2]}));
        assert_eq!(back.note, Some(serde_json::json!("x")));
    }

    #[test]
    fn a_missing_optional_text_stays_none() {
        let engine = Engine {
            id: 1,
            extra: serde_json::Value::Null,
            note: None,
        };
        let record: Record = out(&engine).unwrap();
        assert_eq!(record.note, None);
        assert_eq!(record.extra, "null");
        let back: Engine = inp(&record).unwrap();
        assert_eq!(back.note, None);
    }

    #[test]
    fn a_shape_mismatch_is_an_internal_contract_error() {
        let err = out::<_, Record>(&serde_json::json!({"id": "not a number"})).unwrap_err();
        let PosError::Internal { detail } = err else {
            panic!("an internal error");
        };
        assert!(detail.starts_with("contract: "), "{detail}");
        assert!(detail.contains("Record"), "{detail}");
    }

    #[test]
    fn invalid_json_text_is_a_contract_error() {
        let record = Record {
            id: 1,
            extra: "{not json".into(),
            note: None,
        };
        let err = inp::<_, Engine>(&record).unwrap_err();
        assert!(matches!(err, PosError::Internal { .. }));
    }
}
