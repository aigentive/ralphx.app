use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use uuid::Uuid;

use super::*;
use crate::application::{QuestionAnswer, QuestionOption};

pub async fn request_question(
    State(state): State<HttpServerState>,
    Json(input): Json<QuestionRequestInput>,
) -> Json<QuestionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();

    // Convert input options to domain QuestionOption
    let options: Vec<QuestionOption> = input
        .options
        .iter()
        .map(|o| QuestionOption {
            value: o.value.clone(),
            label: o.label.clone(),
            description: o.description.clone(),
        })
        .collect();

    // Register in QuestionState
    state
        .app_state
        .question_state
        .register(
            request_id.clone(),
            input.session_id.clone(),
            input.question.clone(),
            input.header.clone(),
            options,
            input.multi_select,
        )
        .await;

    // Emit Tauri event to frontend
    if let Some(ref app_handle) = state.app_state.app_handle {
        let _ = app_handle.emit(
            "agent:ask_user_question",
            serde_json::json!({
                "requestId": &request_id,
                "sessionId": &input.session_id,
                "question": &input.question,
                "header": &input.header,
                "options": &input.options,
                "multiSelect": input.multi_select,
            }),
        );
    }

    Json(QuestionRequestResponse { request_id })
}

pub async fn await_question(
    State(state): State<HttpServerState>,
    Path(request_id): Path<String>,
) -> Result<Json<QuestionAnswer>, StatusCode> {
    // Get the receiver for this request
    let mut rx = {
        let pending = state.app_state.question_state.pending.lock().await;
        match pending.get(&request_id).map(|req| req.sender.subscribe()) {
            Some(rx) => rx,
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    // Wait for answer with 5 minute timeout
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    loop {
        // Check if value is Some
        let maybe_answer: Option<QuestionAnswer> = {
            let current = rx.borrow();
            current.clone()
        };

        if let Some(answer) = maybe_answer {
            // Clean up
            state.app_state.question_state.remove(&request_id).await;
            return Ok(Json(answer));
        }

        // Check timeout
        if start.elapsed() >= timeout {
            state.app_state.question_state.remove(&request_id).await;
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        // Wait for change with remaining timeout
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, rx.changed()).await {
            Ok(Ok(())) => continue,
            Ok(Err(_)) => {
                // Channel closed
                state.app_state.question_state.remove(&request_id).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(_) => {
                // Timeout
                state.app_state.question_state.remove(&request_id).await;
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        }
    }
}

pub async fn resolve_question(
    State(state): State<HttpServerState>,
    Json(input): Json<ResolveQuestionInput>,
) -> StatusCode {
    let resolved = state
        .app_state
        .question_state
        .resolve(
            &input.request_id,
            QuestionAnswer {
                selected_options: input.selected_options,
                text: input.text,
            },
        )
        .await;

    if resolved {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
