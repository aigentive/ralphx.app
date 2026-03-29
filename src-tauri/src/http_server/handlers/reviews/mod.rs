use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tauri::Emitter;

use super::*;
use crate::application::{GitService, TaskSchedulerService, TaskTransitionService};
use crate::domain::entities::{
    ActivityEvent, ActivityEventRole, ActivityEventType, InternalStatus, Review,
    ReviewIssue as ReviewNoteIssue, ReviewIssueEntity, ReviewNote, ReviewOutcome,
    ReviewScopeMetadata, ReviewerType, TaskId,
};
use crate::domain::services::running_agent_registry::RunningAgentKey;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::{
    deferred_merge_cleanup, set_no_code_changes_metadata, set_pending_cleanup_metadata,
};
use crate::domain::review::{
    build_unrelated_drift_followup_prompt, compute_out_of_scope_blocker_fingerprint,
    parse_review_decision, parse_review_issues, review_outcome_for_tool,
    validate_complete_review_policy, RawReviewIssueInput,
};
use crate::domain::tools::complete_review::{ReviewToolOutcome, ScopeDriftClassification};
use crate::http_server::handlers::session_linking::create_child_session_impl;
use crate::http_server::helpers::get_task_context_impl;
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use crate::http_server::types::CreateChildSessionRequest;

mod complete;
mod human;
mod notes;

pub use complete::complete_review;
pub use human::{approve_task, request_task_changes};
pub use notes::get_review_notes;
