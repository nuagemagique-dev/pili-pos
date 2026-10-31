//! `HubRise` error response shape.
//!
//! Reference: `docs/hubrise/01-general-concepts.md` (Error Responses).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    Unauthorized,
    Forbidden,
    NotFound,
    UnprocessableEntity,
    UnsupportedMediaType,
    TooManyRequests,
    InternalError,
    RoutingError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(default)]
    pub message: Option<String>,
    pub error_type: ErrorType,
    #[serde(default)]
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(error_type: ErrorType, message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            error_type,
            errors: Vec::new(),
        }
    }
}
