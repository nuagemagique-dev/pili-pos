//! The error every FFI call returns. Kotlin sees `PosException` (sealed, one subclass per variant).
//!
//! Field names are `detail` and `params`, never `message`: UniFFI's Kotlin bindings would clash
//! with `Throwable.message`.

use std::collections::HashMap;

use pos_engine::commands::errors;

/// What an FFI call can fail with.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PosError {
    /// A refusal the screen translates: a code of `pos_engine::commands::errors` and its values
    /// (`pin_locked|seconds=30` is `code = "pin_locked"`, `params = {"seconds": "30"}`).
    #[error("{code}")]
    Coded {
        code: String,
        params: HashMap<String, String>,
    },
    /// A failure of the FDM (the text starts with `FDM`), translated from the FDM's own codes.
    #[error("{detail}")]
    Fdm { detail: String },
    /// Anything else: a bug, a broken contract, an I/O failure. For the technician, not the screen.
    #[error("internal error: {detail}")]
    Internal { detail: String },
    /// The engine was shut down (or is shutting down).
    #[error("engine closed")]
    EngineClosed,
}

impl PosError {
    pub(crate) fn internal(detail: impl Into<String>) -> Self {
        PosError::Internal {
            detail: detail.into(),
        }
    }
}

/// Whether `code` has the shape of an engine error code: `^[a-z][a-z_]*$`.
fn is_code_shape(code: &str) -> bool {
    let mut chars = code.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c == '_')
}

fn is_known_code(code: &str) -> bool {
    errors::ALL.contains(&code) || errors::hubrise::ALL.contains(&code)
}

/// Map the `String` error of an engine command to a [`PosError`].
///
/// - text starting with `FDM` (after trimming): [`PosError::Fdm`];
/// - `code|k=v|k=v` where `code` is a known code of either list: [`PosError::Coded`];
/// - anything else (including the empty text): [`PosError::Internal`].
pub fn from_engine(s: String) -> PosError {
    let text = s.trim();
    if text.starts_with("FDM") {
        return PosError::Fdm {
            detail: text.to_string(),
        };
    }
    let mut parts = text.split('|');
    let code = parts.next().unwrap_or_default();
    if is_code_shape(code) && is_known_code(code) {
        let params = parts
            .filter_map(|part| part.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        return PosError::Coded {
            code: code.to_string(),
            params,
        };
    }
    if text.is_empty() {
        return PosError::internal("empty error text from the engine");
    }
    PosError::internal(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_of_both_lists_maps_to_coded() {
        let all: Vec<&str> = errors::ALL
            .iter()
            .chain(errors::hubrise::ALL.iter())
            .copied()
            .collect();
        assert_eq!(all.len(), 125, "the engine's code lists changed size");
        for code in all {
            match from_engine(code.to_string()) {
                PosError::Coded { code: c, params } => {
                    assert_eq!(c, code);
                    assert!(params.is_empty());
                }
                other => panic!("{code} mapped to {other:?}"),
            }
        }
    }

    #[test]
    fn values_are_parsed_at_the_first_equals_sign() {
        let PosError::Coded { code, params } =
            from_engine("refund_exceeds_sold|product=12|sold=2|detail=a=b".into())
        else {
            panic!("a coded error");
        };
        assert_eq!(code, "refund_exceeds_sold");
        assert_eq!(params.len(), 3);
        assert_eq!(params["product"], "12");
        assert_eq!(params["sold"], "2");
        assert_eq!(params["detail"], "a=b");
    }

    #[test]
    fn a_part_without_equals_sign_is_ignored() {
        let PosError::Coded { params, .. } = from_engine("pin_locked|seconds=30|junk".into())
        else {
            panic!("a coded error");
        };
        assert_eq!(params.len(), 1);
        assert_eq!(params["seconds"], "30");
    }

    #[test]
    fn an_fdm_text_maps_to_fdm_even_with_a_pipe() {
        let PosError::Fdm { detail } = from_engine("FDM_ANSWER_PENDING|label=N".into()) else {
            panic!("an fdm error");
        };
        assert_eq!(detail, "FDM_ANSWER_PENDING|label=N");
        assert!(matches!(
            from_engine("  FDM error: unreachable ".into()),
            PosError::Fdm { .. }
        ));
    }

    #[test]
    fn an_unknown_code_is_internal() {
        let PosError::Internal { detail } = from_engine("no_such_code|a=b".into()) else {
            panic!("an internal error");
        };
        assert_eq!(detail, "no_such_code|a=b");
        assert!(matches!(
            from_engine("Not a code at all".into()),
            PosError::Internal { .. }
        ));
    }

    #[test]
    fn an_empty_text_is_internal() {
        assert!(matches!(
            from_engine(String::new()),
            PosError::Internal { .. }
        ));
        assert!(matches!(
            from_engine("  ".into()),
            PosError::Internal { .. }
        ));
    }

    #[test]
    fn the_code_shape() {
        assert!(is_code_shape("order_no_lines"));
        assert!(!is_code_shape(""));
        assert!(!is_code_shape("_x"));
        assert!(!is_code_shape("Order"));
        assert!(!is_code_shape("a1"));
    }
}
