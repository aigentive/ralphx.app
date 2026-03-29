use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::error;

use crate::domain::entities::{IdeationSessionId, VerificationStatus};
use crate::domain::services::{emit_verification_started, emit_verification_status_changed};
use crate::error::AppError;
use crate::infrastructure::agents::claude::verification_config;
use crate::infrastructure::sqlite::SqliteIdeationSessionRepository as SessionRepo;

use super::super::types::{
    AutoAcceptVerificationRequest, ConfirmVerificationRequest, ConfirmationStatusResponse,
    DismissVerificationRequest, HttpError, HttpServerState, SpecialistEntryResponse,
    SpecialistsResponse, VerificationActionResponse,
};

mod auto_accept;
mod confirmation_status;
mod confirm;
mod dismiss;
mod specialist_registry;

pub use auto_accept::set_auto_accept_verification;
pub use confirmation_status::get_confirmation_status;
pub use confirm::confirm_verification;
pub use dismiss::dismiss_verification;
pub use specialist_registry::get_verification_specialists;

/// Map an AppError to an HttpError for verification handler responses.
fn map_app_err_local(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        AppError::Conflict(msg) => HttpError {
            status: StatusCode::CONFLICT,
            message: Some(msg),
        },
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}
