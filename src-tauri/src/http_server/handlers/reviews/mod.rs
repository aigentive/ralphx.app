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
    ActivityEvent, ActivityEventRole, ActivityEventType, IssueCategory, IssueSeverity,
    InternalStatus, Review, ReviewIssue as ReviewNoteIssue, ReviewIssueEntity, ReviewNote,
    ReviewOutcome, ReviewScopeMetadata, ReviewerType, ScopeDriftStatus, TaskId, TaskStepId,
};
use crate::domain::services::running_agent_registry::RunningAgentKey;
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::transition_handler::{
    deferred_merge_cleanup, set_no_code_changes_metadata, set_pending_cleanup_metadata,
};
use crate::domain::tools::complete_review::{ReviewToolOutcome, ScopeDriftClassification};
use crate::http_server::handlers::session_linking::create_child_session_impl;
use crate::http_server::helpers::{
    compute_out_of_scope_blocker_fingerprint, get_task_context_impl,
};
use crate::http_server::project_scope::{ProjectScope, ProjectScopeGuard};
use crate::http_server::types::{CreateChildSessionRequest, ReviewIssueRequest};

mod complete;
mod human;
mod notes;

pub use complete::complete_review;
pub use human::{approve_task, request_task_changes};
pub use notes::get_review_notes;
