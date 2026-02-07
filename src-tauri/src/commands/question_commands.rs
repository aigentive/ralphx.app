// Tauri commands for question resolution
// Allows frontend to resolve pending questions from agents (AskUserQuestion)

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::{PendingQuestionInfo, QuestionAnswer};
use crate::AppState;

/// Arguments for resolving a question
#[derive(Debug, Deserialize)]
pub struct ResolveQuestionArgs {
    pub request_id: String,
    pub selected_options: Vec<String>,
    pub text: Option<String>,
}

/// Response for resolve_user_question command
#[derive(Debug, Serialize)]
pub struct ResolveQuestionResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Resolve a pending question with the user's answer
///
/// Called by the frontend AskUserQuestionCard when the user submits their answer.
/// Signals the waiting MCP long-poll request with the answer.
#[tauri::command]
pub async fn resolve_user_question(
    state: State<'_, AppState>,
    args: ResolveQuestionArgs,
) -> Result<ResolveQuestionResponse, String> {
    let answer = QuestionAnswer {
        selected_options: args.selected_options,
        text: args.text,
    };

    let resolved = state
        .question_state
        .resolve(&args.request_id, answer)
        .await;

    if resolved {
        Ok(ResolveQuestionResponse {
            success: true,
            message: Some(format!("Question {} resolved", args.request_id)),
        })
    } else {
        Err(format!(
            "Question request '{}' not found",
            args.request_id
        ))
    }
}

/// Get information about all pending questions
///
/// Used by the frontend to display any pending questions that might have been
/// missed (e.g., if the chat view was just opened while an agent was asking).
#[tauri::command]
pub async fn get_pending_questions(
    state: State<'_, AppState>,
) -> Result<Vec<PendingQuestionInfo>, String> {
    let pending = state.question_state.get_pending_info().await;
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_question_args_deserialize() {
        let json = r#"{"request_id": "abc-123", "selected_options": ["opt1", "opt2"], "text": "Custom answer"}"#;
        let args: ResolveQuestionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.request_id, "abc-123");
        assert_eq!(args.selected_options, vec!["opt1", "opt2"]);
        assert_eq!(args.text, Some("Custom answer".to_string()));
    }

    #[test]
    fn test_resolve_question_args_without_text() {
        let json = r#"{"request_id": "abc-123", "selected_options": ["opt1"]}"#;
        let args: ResolveQuestionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.request_id, "abc-123");
        assert_eq!(args.selected_options, vec!["opt1"]);
        assert!(args.text.is_none());
    }

    #[test]
    fn test_resolve_question_response_serialize() {
        let response = ResolveQuestionResponse {
            success: true,
            message: Some("Resolved".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Resolved\""));
    }
}
