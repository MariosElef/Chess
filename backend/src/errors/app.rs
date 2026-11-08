use axum::{
    Json,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::codes::ChessCode;

/// Every error a handler returns, answered as RFC 9457 problem details.
#[derive(Debug)]
pub enum AppError {
    /// A problem the client caused, with what only this occurrence knows.
    Problem {
        code: ChessCode,
        detail: Option<String>,
        fields: Vec<FieldError>,
    },
    /// A service this server depends on failed: answered with the code's title, the cause logged.
    Unavailable {
        code: ChessCode,
        cause: anyhow::Error,
    },
    /// A failure of ours: logged in full, answered as `internal-error` without the cause.
    Unexpected(anyhow::Error),
}

/// One request field at fault: its JSON pointer and the sentence for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub detail: String,
    pub pointer: String,
}

impl FieldError {
    pub fn new(pointer: &str, detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            pointer: pointer.to_owned(),
        }
    }
}

impl AppError {
    pub fn with_detail(code: ChessCode, detail: impl Into<String>) -> Self {
        Self::Problem {
            code,
            detail: Some(detail.into()),
            fields: Vec::new(),
        }
    }

    pub fn unavailable(code: ChessCode, cause: impl Into<anyhow::Error>) -> Self {
        Self::Unavailable {
            code,
            cause: cause.into(),
        }
    }

    pub fn fields(fields: Vec<FieldError>) -> Self {
        Self::Problem {
            code: ChessCode::InvalidFields,
            detail: None,
            fields,
        }
    }

    /// The code this error answers with.
    pub fn code(&self) -> ChessCode {
        match self {
            Self::Problem { code, .. } | Self::Unavailable { code, .. } => *code,
            Self::Unexpected(_) => ChessCode::InternalError,
        }
    }
}

impl From<ChessCode> for AppError {
    fn from(code: ChessCode) -> Self {
        Self::Problem {
            code,
            detail: None,
            fields: Vec::new(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::Unexpected(error)
    }
}

/// The body of `application/problem+json`.
#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    problem_type: String,
    title: &'static str,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<FieldError>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = self.code();
        let (detail, errors) = match self {
            Self::Problem { detail, fields, .. } => (detail, fields),
            Self::Unavailable { cause, .. } => {
                tracing::warn!(error = ?cause, problem = %code.problem_type(), "a dependency failed");
                (None, Vec::new())
            }
            Self::Unexpected(error) => {
                tracing::error!(error = ?error, "unexpected error");
                (None, Vec::new())
            }
        };
        let problem = Problem {
            problem_type: code.problem_type(),
            title: code.title(),
            status: code.status().as_u16(),
            detail,
            errors,
        };
        let mut response = (code.status(), Json(problem)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}
