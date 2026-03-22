// Ideation tool handlers for MCP orchestrator-ideation agent

use axum::{http::StatusCode, Json};

mod dependency_analysis;
mod proposals;
mod runtime;
mod verification;

pub use dependency_analysis::*;
pub use proposals::*;
pub use runtime::*;
pub use verification::*;

type JsonError = (StatusCode, Json<serde_json::Value>);

fn json_error(status: StatusCode, error: impl Into<String>) -> JsonError {
    (status, Json(serde_json::json!({ "error": error.into() })))
}
