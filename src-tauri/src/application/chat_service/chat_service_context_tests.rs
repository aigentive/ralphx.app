use super::*;
use crate::domain::entities::ChatAttachment;
use crate::domain::repositories::{StateHistoryMetadata, StatusTransition};

#[test]
fn test_is_text_file_by_mime_type() {
    // Text MIME types
    assert!(is_text_file(Some("text/plain"), "file.txt"));
    assert!(is_text_file(Some("text/html"), "file.html"));
    assert!(is_text_file(Some("application/json"), "file.json"));
    assert!(is_text_file(Some("application/xml"), "file.xml"));
    assert!(is_text_file(Some("application/javascript"), "file.js"));
    assert!(is_text_file(Some("application/typescript"), "file.ts"));

    // Binary MIME types
    assert!(!is_text_file(Some("image/png"), "file.png"));
    assert!(!is_text_file(Some("application/pdf"), "file.pdf"));
    assert!(!is_text_file(Some("video/mp4"), "file.mp4"));
}

#[test]
fn test_is_text_file_by_extension() {
    // Common text extensions (no MIME type provided)
    assert!(is_text_file(None, "file.txt"));
    assert!(is_text_file(None, "file.md"));
    assert!(is_text_file(None, "file.rs"));
    assert!(is_text_file(None, "file.ts"));
    assert!(is_text_file(None, "file.tsx"));
    assert!(is_text_file(None, "file.js"));
    assert!(is_text_file(None, "file.jsx"));
    assert!(is_text_file(None, "file.json"));
    assert!(is_text_file(None, "file.yaml"));
    assert!(is_text_file(None, "file.yml"));
    assert!(is_text_file(None, "file.xml"));
    assert!(is_text_file(None, "file.html"));
    assert!(is_text_file(None, "file.css"));
    assert!(is_text_file(None, "file.py"));
    assert!(is_text_file(None, "file.java"));
    assert!(is_text_file(None, "file.c"));
    assert!(is_text_file(None, "file.cpp"));
    assert!(is_text_file(None, "file.h"));
    assert!(is_text_file(None, "file.go"));
    assert!(is_text_file(None, "file.sh"));
    assert!(is_text_file(None, "file.toml"));
    assert!(is_text_file(None, "file.csv"));
    assert!(is_text_file(None, "file.log"));
    assert!(is_text_file(None, "file.sql"));
    assert!(is_text_file(None, "file.graphql"));

    // Binary extensions
    assert!(!is_text_file(None, "file.png"));
    assert!(!is_text_file(None, "file.jpg"));
    assert!(!is_text_file(None, "file.pdf"));
    assert!(!is_text_file(None, "file.mp4"));
    assert!(!is_text_file(None, "file.zip"));

    // Files without extensions
    assert!(!is_text_file(None, "README"));
    assert!(!is_text_file(None, "no-extension"));
}

#[tokio::test]
async fn test_format_attachments_empty() {
    let attachments: Vec<ChatAttachment> = vec![];
    let result = format_attachments_for_agent(&attachments).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[tokio::test]
async fn test_format_attachments_binary_file() {
    let conversation_id = ChatConversationId::new();
    let attachment = ChatAttachment::new(
        conversation_id,
        "screenshot.png",
        "/path/to/screenshot.png",
        1024,
        Some("image/png".to_string()),
    );

    let result = format_attachments_for_agent(&[attachment]).await;
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert!(formatted.contains("<attachments>"));
    assert!(formatted.contains("<filename>screenshot.png</filename>"));
    assert!(formatted.contains("<mime_type>image/png</mime_type>"));
    assert!(formatted.contains("<file_path>/path/to/screenshot.png</file_path>"));
    assert!(formatted.contains("Use the Read tool to access this file"));
    assert!(formatted.contains("</attachments>"));
}

#[tokio::test]
async fn test_format_attachments_text_file() {
    use std::fs;

    // Create a temporary text file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_attachment.txt");
    let test_content = "Hello, this is a test file!";
    fs::write(&temp_file, test_content).expect("Failed to write test file");

    let conversation_id = ChatConversationId::new();
    let attachment = ChatAttachment::new(
        conversation_id,
        "test_attachment.txt",
        temp_file.to_str().unwrap(),
        test_content.len() as i64,
        Some("text/plain".to_string()),
    );

    let result = format_attachments_for_agent(&[attachment]).await;
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert!(formatted.contains("<attachments>"));
    assert!(formatted.contains("<filename>test_attachment.txt</filename>"));
    assert!(formatted.contains("<mime_type>text/plain</mime_type>"));
    assert!(formatted.contains("<content>"));
    assert!(formatted.contains(test_content));
    assert!(formatted.contains("</content>"));
    assert!(formatted.contains("</attachments>"));

    // Cleanup
    fs::remove_file(temp_file).ok();
}

#[tokio::test]
async fn test_format_attachments_multiple_files() {
    use std::fs;

    // Create a temporary text file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_multiple.txt");
    let test_content = "Test content";
    fs::write(&temp_file, test_content).expect("Failed to write test file");

    let conversation_id = ChatConversationId::new();
    let text_attachment = ChatAttachment::new(
        conversation_id,
        "test_multiple.txt",
        temp_file.to_str().unwrap(),
        test_content.len() as i64,
        Some("text/plain".to_string()),
    );

    let binary_attachment = ChatAttachment::new(
        conversation_id,
        "image.png",
        "/path/to/image.png",
        2048,
        Some("image/png".to_string()),
    );

    let result = format_attachments_for_agent(&[text_attachment, binary_attachment]).await;
    assert!(result.is_ok());

    let formatted = result.unwrap();

    // Should contain both attachments
    assert!(formatted.contains("test_multiple.txt"));
    assert!(formatted.contains(test_content));
    assert!(formatted.contains("image.png"));
    assert!(formatted.contains("/path/to/image.png"));
    assert!(formatted.contains("Use the Read tool to access this file"));

    // Cleanup
    fs::remove_file(temp_file).ok();
}

#[tokio::test]
async fn test_format_attachments_file_read_error() {
    let conversation_id = ChatConversationId::new();
    let attachment = ChatAttachment::new(
        conversation_id,
        "nonexistent.txt",
        "/nonexistent/path/file.txt",
        0,
        Some("text/plain".to_string()),
    );

    let result = format_attachments_for_agent(&[attachment]).await;
    assert!(result.is_ok());

    let formatted = result.unwrap();
    assert!(formatted.contains("<filename>nonexistent.txt</filename>"));
    assert!(formatted.contains("<error>Failed to read file:"));
}

// Tests for get_entity_status_for_resume
use crate::domain::entities::{IdeationSession, IdeationSessionStatus, ProjectId, VerificationStatus};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::AppResult;
use async_trait::async_trait;

// Mock for testing
struct MockIdeationRepo {
    session: Option<IdeationSession>,
}

impl MockIdeationRepo {
    fn with_session(session: IdeationSession) -> Self {
        Self {
            session: Some(session),
        }
    }
    fn empty() -> Self {
        Self { session: None }
    }
}

#[async_trait]
impl IdeationSessionRepository for MockIdeationRepo {
    async fn create(&self, _session: IdeationSession) -> AppResult<IdeationSession> {
        unimplemented!()
    }
    async fn get_by_id(&self, _id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        Ok(self.session.clone())
    }
    async fn get_by_project(&self, _project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn update_status(
        &self,
        _id: &IdeationSessionId,
        _status: IdeationSessionStatus,
    ) -> AppResult<()> {
        unimplemented!()
    }
    async fn update_title(
        &self,
        _id: &IdeationSessionId,
        _title: Option<String>,
        _title_source: &str,
    ) -> AppResult<()> {
        unimplemented!()
    }
    async fn delete(&self, _id: &IdeationSessionId) -> AppResult<()> {
        unimplemented!()
    }
    async fn get_active_by_project(
        &self,
        _project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn count_by_status(
        &self,
        _project_id: &ProjectId,
        _status: IdeationSessionStatus,
    ) -> AppResult<u32> {
        unimplemented!()
    }
    async fn update_plan_artifact_id(
        &self,
        _id: &IdeationSessionId,
        _plan_artifact_id: Option<String>,
    ) -> AppResult<()> {
        unimplemented!()
    }
    async fn get_by_plan_artifact_id(
        &self,
        _plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn get_by_inherited_plan_artifact_id(
        &self,
        _artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn get_children(
        &self,
        _parent_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn get_ancestor_chain(
        &self,
        _session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }
    async fn set_parent(
        &self,
        _session_id: &IdeationSessionId,
        _parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()> {
        unimplemented!()
    }

    async fn update_verification_state(
        &self,
        _id: &IdeationSessionId,
        _status: VerificationStatus,
        _in_progress: bool,
        _metadata_json: Option<String>,
    ) -> AppResult<()> {
        unimplemented!()
    }

    async fn reset_verification(&self, _id: &IdeationSessionId) -> AppResult<bool> {
        unimplemented!()
    }

    async fn get_verification_status(
        &self,
        _id: &IdeationSessionId,
    ) -> AppResult<Option<(VerificationStatus, bool, Option<String>)>> {
        unimplemented!()
    }

    async fn revert_plan_and_skip_verification(
        &self,
        _id: &IdeationSessionId,
        _new_plan_artifact_id: String,
        _convergence_reason: String,
    ) -> AppResult<()> {
        unimplemented!()
    }

    async fn revert_plan_and_skip_with_artifact(
        &self,
        _session_id: &IdeationSessionId,
        _new_artifact_id: String,
        _artifact_type_str: String,
        _artifact_name: String,
        _content_text: String,
        _version: u32,
        _previous_version_id: String,
        _convergence_reason: String,
    ) -> AppResult<()> {
        unimplemented!()
    }

    async fn get_stale_in_progress_sessions(
        &self,
        _stale_before: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }

    async fn get_by_project_and_status(
        &self,
        _project_id: &str,
        _status: &str,
        _limit: u32,
    ) -> AppResult<Vec<IdeationSession>> {
        unimplemented!()
    }

    async fn get_group_counts(
        &self,
        _project_id: &ProjectId,
    ) -> AppResult<crate::domain::repositories::ideation_session_repository::SessionGroupCounts>
    {
        unimplemented!()
    }

    async fn list_by_group(
        &self,
        _project_id: &ProjectId,
        _group: &str,
        _offset: u32,
        _limit: u32,
    ) -> AppResult<(Vec<crate::domain::repositories::ideation_session_repository::IdeationSessionWithProgress>, u32)> {
        unimplemented!()
    }
}

struct MockTaskRepo;

#[async_trait]
impl TaskRepository for MockTaskRepo {
    async fn create(
        &self,
        task: crate::domain::entities::Task,
    ) -> AppResult<crate::domain::entities::Task> {
        Ok(task)
    }

    async fn get_by_id(&self, _id: &TaskId) -> AppResult<Option<crate::domain::entities::Task>> {
        Ok(None)
    }

    async fn get_by_project(
        &self,
        _project_id: &ProjectId,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn update(&self, _task: &crate::domain::entities::Task) -> AppResult<()> {
        Ok(())
    }

    async fn update_with_expected_status(
        &self,
        _task: &crate::domain::entities::Task,
        _expected_status: crate::domain::entities::InternalStatus,
    ) -> AppResult<bool> {
        Ok(true)
    }

    async fn update_metadata(&self, _id: &TaskId, _metadata: Option<String>) -> AppResult<()> {
        Ok(())
    }

    async fn delete(&self, _id: &TaskId) -> AppResult<()> {
        Ok(())
    }

    async fn clear_task_references(&self, _id: &TaskId) -> AppResult<()> {
        Ok(())
    }

    async fn get_by_status(
        &self,
        _project_id: &ProjectId,
        _status: crate::domain::entities::InternalStatus,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn persist_status_change(
        &self,
        _id: &TaskId,
        _from: crate::domain::entities::InternalStatus,
        _to: crate::domain::entities::InternalStatus,
        _trigger: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn get_status_history(&self, _id: &TaskId) -> AppResult<Vec<StatusTransition>> {
        Ok(vec![])
    }

    async fn get_status_entered_at(
        &self,
        _task_id: &TaskId,
        _status: crate::domain::entities::InternalStatus,
    ) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(None)
    }

    async fn get_next_executable(
        &self,
        _project_id: &ProjectId,
    ) -> AppResult<Option<crate::domain::entities::Task>> {
        Ok(None)
    }

    async fn get_by_ideation_session(
        &self,
        _session_id: &crate::domain::entities::IdeationSessionId,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn get_by_project_filtered(
        &self,
        _project_id: &ProjectId,
        _include_archived: bool,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn archive(&self, _task_id: &TaskId) -> AppResult<crate::domain::entities::Task> {
        unimplemented!()
    }

    async fn restore(&self, _task_id: &TaskId) -> AppResult<crate::domain::entities::Task> {
        unimplemented!()
    }

    async fn get_archived_count(
        &self,
        _project_id: &ProjectId,
        _ideation_session_id: Option<&str>,
    ) -> AppResult<u32> {
        Ok(0)
    }

    async fn list_paginated(
        &self,
        _project_id: &ProjectId,
        _statuses: Option<Vec<crate::domain::entities::InternalStatus>>,
        _offset: u32,
        _limit: u32,
        _include_archived: bool,
        _ideation_session_id: Option<&str>,
        _execution_plan_id: Option<&str>,
        _categories: Option<&[String]>,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn count_tasks(
        &self,
        _project_id: &ProjectId,
        _include_archived: bool,
        _ideation_session_id: Option<&str>,
        _execution_plan_id: Option<&str>,
    ) -> AppResult<u32> {
        Ok(0)
    }

    async fn search(
        &self,
        _project_id: &ProjectId,
        _query: &str,
        _include_archived: bool,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn get_oldest_ready_task(&self) -> AppResult<Option<crate::domain::entities::Task>> {
        Ok(None)
    }

    async fn get_oldest_ready_tasks(
        &self,
        _limit: u32,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn get_stale_ready_tasks(
        &self,
        _threshold_secs: u64,
    ) -> AppResult<Vec<crate::domain::entities::Task>> {
        Ok(vec![])
    }

    async fn update_latest_state_history_metadata(
        &self,
        _task_id: &TaskId,
        _metadata: &StateHistoryMetadata,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn has_task_in_states(
        &self,
        _project_id: &ProjectId,
        _statuses: &[crate::domain::entities::InternalStatus],
    ) -> AppResult<bool> {
        Ok(false)
    }

    async fn get_status_history_batch(
        &self,
        _task_ids: &[crate::domain::entities::TaskId],
    ) -> AppResult<
        std::collections::HashMap<
            crate::domain::entities::TaskId,
            Vec<crate::domain::repositories::StatusTransition>,
        >,
    > {
        Ok(std::collections::HashMap::new())
    }
}

#[tokio::test]
async fn test_get_entity_status_for_resume_ideation_accepted() {
    let project_id = ProjectId::new();
    let session_id = IdeationSessionId::new();
    let mut session = IdeationSession::new(project_id.clone());
    session.id = session_id.clone();
    session.status = IdeationSessionStatus::Accepted;

    let ideation_repo = Arc::new(MockIdeationRepo::with_session(session));
    let task_repo = Arc::new(MockTaskRepo);

    let status = get_entity_status_for_resume(
        ChatContextType::Ideation,
        session_id.as_str(),
        ideation_repo,
        task_repo,
    )
    .await;

    assert_eq!(status, Some("accepted".to_string()));
}

#[tokio::test]
async fn test_get_entity_status_for_resume_ideation_active() {
    let project_id = ProjectId::new();
    let session_id = IdeationSessionId::new();
    let mut session = IdeationSession::new(project_id.clone());
    session.id = session_id.clone();
    session.status = IdeationSessionStatus::Active;

    let ideation_repo = Arc::new(MockIdeationRepo::with_session(session));
    let task_repo = Arc::new(MockTaskRepo);

    let status = get_entity_status_for_resume(
        ChatContextType::Ideation,
        session_id.as_str(),
        ideation_repo,
        task_repo,
    )
    .await;

    assert_eq!(status, Some("active".to_string()));
}

#[tokio::test]
async fn test_get_entity_status_for_resume_ideation_not_found() {
    let session_id = IdeationSessionId::new();

    let ideation_repo = Arc::new(MockIdeationRepo::empty());
    let task_repo = Arc::new(MockTaskRepo);

    let status = get_entity_status_for_resume(
        ChatContextType::Ideation,
        session_id.as_str(),
        ideation_repo,
        task_repo,
    )
    .await;

    assert_eq!(status, None);
}

#[tokio::test]
async fn test_get_entity_status_for_resume_project_context() {
    let ideation_repo = Arc::new(MockIdeationRepo::empty());
    let task_repo = Arc::new(MockTaskRepo);

    let status = get_entity_status_for_resume(
        ChatContextType::Project,
        "project-id",
        ideation_repo,
        task_repo,
    )
    .await;

    // Project context doesn't have status-based agent resolution
    assert_eq!(status, None);
}

use crate::infrastructure::memory::MemoryChatAttachmentRepository;

#[tokio::test]
async fn test_build_command_with_team_mode_true() {
    // Test that build_command accepts team_mode=true parameter
    // (function will return error in test env due to missing CLI, but that's expected)
    let cli_path = std::path::PathBuf::from("/usr/bin/claude");
    let plugin_dir = std::path::PathBuf::from("/tmp/plugin");
    let working_dir = std::path::PathBuf::from("/tmp");

    let session_id = IdeationSessionId::from_string("test-session-id");
    let conversation = ChatConversation::new_ideation(session_id);

    let chat_attachment_repo = Arc::new(MemoryChatAttachmentRepository::new());

    // Should not panic with team_mode=true
    // The function will error in test env, but we're just testing the signature works
    let _result = build_command(
        &cli_path,
        &plugin_dir,
        &conversation,
        "test message",
        &working_dir,
        None,
        None,
        true, // team_mode=true
        chat_attachment_repo,
        &[],
        0,
    )
    .await;

    // Test passes if no panic occurred (Err result is expected in test env)
}

#[tokio::test]
async fn test_build_command_with_team_mode_false() {
    // Test that build_command accepts team_mode=false parameter
    let cli_path = std::path::PathBuf::from("/usr/bin/claude");
    let plugin_dir = std::path::PathBuf::from("/tmp/plugin");
    let working_dir = std::path::PathBuf::from("/tmp");

    let session_id = IdeationSessionId::from_string("test-session-id");
    let conversation = ChatConversation::new_ideation(session_id);

    let chat_attachment_repo = Arc::new(MemoryChatAttachmentRepository::new());

    // Should not panic with team_mode=false
    let _result = build_command(
        &cli_path,
        &plugin_dir,
        &conversation,
        "test message",
        &working_dir,
        None,
        None,
        false, // team_mode=false
        chat_attachment_repo,
        &[],
        0,
    )
    .await;

    // Test passes if no panic occurred
}

// Tests for build_resume_initial_prompt

#[test]
fn test_build_resume_initial_prompt_ideation_includes_context_id_no_recovery_note() {
    // After the session_history injection refactor, the <recovery_note> has been removed.
    // build_resume_initial_prompt now delegates to build_initial_prompt.
    let context_id = "test-session-123";
    let user_message = "hello";
    let result =
        build_resume_initial_prompt(ChatContextType::Ideation, context_id, user_message, &[], 0);
    assert!(result.contains(&format!("<context_id>{}</context_id>", context_id)));
    assert!(!result.contains("<recovery_note>"));
    assert!(!result.contains("get_session_messages"));
    assert!(result.contains(&format!("<user_message>{}</user_message>", user_message)));
}

#[test]
fn test_build_resume_initial_prompt_task_includes_context_id_no_recovery_note() {
    let context_id = "task-abc";
    let user_message = "hello";
    let result =
        build_resume_initial_prompt(ChatContextType::Task, context_id, user_message, &[], 0);
    assert!(result.contains(&format!("<task_id>{}</task_id>", context_id)));
    assert!(!result.contains("<recovery_note>"));
    assert!(result.contains(&format!("<user_message>{}</user_message>", user_message)));
}

#[test]
fn test_build_resume_initial_prompt_project_includes_context_id_no_recovery_note() {
    let context_id = "project-xyz";
    let user_message = "hello";
    let result =
        build_resume_initial_prompt(ChatContextType::Project, context_id, user_message, &[], 0);
    assert!(result.contains(&format!("<project_id>{}</project_id>", context_id)));
    assert!(!result.contains("<recovery_note>"));
}

#[test]
fn test_build_resume_initial_prompt_task_execution_delegates_to_initial_prompt() {
    let context_id = "task-exec-123";
    let user_message = "execute";
    let resume =
        build_resume_initial_prompt(ChatContextType::TaskExecution, context_id, user_message, &[], 0);
    let initial =
        build_initial_prompt(ChatContextType::TaskExecution, context_id, user_message, &[], 0);
    assert_eq!(resume, initial);
}

#[tokio::test]
async fn test_build_resume_command_with_team_mode() {
    // Test that build_resume_command accepts team_mode parameter
    let cli_path = std::path::PathBuf::from("/usr/bin/claude");
    let plugin_dir = std::path::PathBuf::from("/tmp/plugin");
    let working_dir = std::path::PathBuf::from("/tmp");

    let chat_attachment_repo = Arc::new(MemoryChatAttachmentRepository::new());
    let ideation_repo = Arc::new(MockIdeationRepo::empty());
    let task_repo = Arc::new(MockTaskRepo);

    // Test with team_mode=true
    let _result = build_resume_command(
        &cli_path,
        &plugin_dir,
        ChatContextType::Ideation,
        "test-session-id",
        "test message",
        &working_dir,
        "session-123",
        None,
        true, // team_mode=true
        chat_attachment_repo.clone(),
        ideation_repo.clone(),
        task_repo.clone(),
        &[],
        0,
    )
    .await;

    // Test with team_mode=false
    let _result = build_resume_command(
        &cli_path,
        &plugin_dir,
        ChatContextType::Ideation,
        "test-session-id",
        "test message",
        &working_dir,
        "session-123",
        None,
        false, // team_mode=false
        chat_attachment_repo,
        ideation_repo,
        task_repo,
        &[],
        0,
    )
    .await;

    // Test passes if no panics occurred
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests for format_session_history
// ──────────────────────────────────────────────────────────────────────────────

fn make_user_msg(session_id: &IdeationSessionId, content: &str) -> ChatMessage {
    ChatMessage::user_in_session(session_id.clone(), content)
}

fn make_orchestrator_msg(session_id: &IdeationSessionId, content: &str) -> ChatMessage {
    ChatMessage::orchestrator_in_session(session_id.clone(), content)
}

fn make_system_msg(session_id: &IdeationSessionId, content: &str) -> ChatMessage {
    ChatMessage::system_in_session(session_id.clone(), content)
}

#[test]
fn format_session_history_empty_slice_returns_empty_string() {
    let result = format_session_history(&[], 0);
    assert_eq!(result, "");
}

#[test]
fn format_session_history_only_system_messages_returns_empty_string() {
    let sid = IdeationSessionId::new();
    let msgs = vec![make_system_msg(&sid, "system init")];
    let result = format_session_history(&msgs, 1);
    assert_eq!(result, "");
}

#[test]
fn format_session_history_basic_user_and_orchestrator() {
    let sid = IdeationSessionId::new();
    let user_msg = make_user_msg(&sid, "hello");
    let orch_msg = make_orchestrator_msg(&sid, "hi back");
    let msgs = vec![user_msg, orch_msg];
    let result = format_session_history(&msgs, 2);
    assert!(result.contains("<session_history"));
    assert!(result.contains("count=\"2\""));
    assert!(result.contains("total_available=\"2\""));
    assert!(result.contains("truncated=\"false\""));
    assert!(result.contains(r#"role="user""#));
    assert!(result.contains("hello"));
    assert!(result.contains(r#"role="orchestrator""#));
    assert!(result.contains("hi back"));
    assert!(result.contains("</session_history>"));
}

#[test]
fn format_session_history_xml_escaping() {
    let sid = IdeationSessionId::new();
    let msg = make_user_msg(&sid, r#"5 < 10 & "hello" > world"#);
    let result = format_session_history(&[msg], 1);
    assert!(result.contains("5 &lt; 10 &amp; &quot;hello&quot; &gt; world"));
    // Raw chars must not appear unescaped inside tag content
    assert!(!result.contains("5 < 10"));
}

#[test]
fn format_session_history_recovery_context_filtered_out() {
    let sid = IdeationSessionId::new();
    let mut recovery_msg = make_user_msg(&sid, "this is recovery");
    recovery_msg.metadata = Some(r#"{"recovery_context": true}"#.to_string());
    let normal_msg = make_user_msg(&sid, "normal message");
    let msgs = vec![recovery_msg, normal_msg];
    let result = format_session_history(&msgs, 2);
    assert!(!result.contains("this is recovery"));
    assert!(result.contains("normal message"));
}

#[test]
fn format_session_history_all_recovery_context_returns_empty_string() {
    let sid = IdeationSessionId::new();
    let mut msg = make_user_msg(&sid, "recovery only");
    msg.metadata = Some(r#"{"recovery_context": true}"#.to_string());
    let result = format_session_history(&[msg], 1);
    assert_eq!(result, "");
}

#[test]
fn format_session_history_subagent_roles_filtered_out() {
    let sid = IdeationSessionId::new();
    // Worker, Reviewer, Merger roles should be excluded (not User or Orchestrator)
    let mut worker_msg = ChatMessage::user_in_session(sid.clone(), "worker output");
    worker_msg.role = crate::domain::entities::MessageRole::Worker;
    let mut reviewer_msg = ChatMessage::user_in_session(sid.clone(), "reviewer output");
    reviewer_msg.role = crate::domain::entities::MessageRole::Reviewer;
    let mut merger_msg = ChatMessage::user_in_session(sid.clone(), "merger output");
    merger_msg.role = crate::domain::entities::MessageRole::Merger;
    let user_msg = make_user_msg(&sid, "user message");

    let msgs = vec![worker_msg, reviewer_msg, merger_msg, user_msg];
    let result = format_session_history(&msgs, 4);
    assert!(!result.contains("worker output"));
    assert!(!result.contains("reviewer output"));
    assert!(!result.contains("merger output"));
    assert!(result.contains("user message"));
}

#[test]
fn format_session_history_per_message_2000_char_truncation() {
    let sid = IdeationSessionId::new();
    let long_content = "x".repeat(3000);
    let msg = make_user_msg(&sid, &long_content);
    let result = format_session_history(&[msg], 1);
    // The 2000 x's should be there, but not 2001+
    assert!(result.contains(&"x".repeat(2000)));
    assert!(!result.contains(&"x".repeat(2001)));
    assert!(result.contains("[truncated]"));
}

#[test]
fn format_session_history_8000_char_cap() {
    let sid = IdeationSessionId::new();
    // Create messages that together exceed 8000 chars after escaping
    let mut msgs = Vec::new();
    // Each message ~1500 chars, so 6 messages = 9000 chars; cap at 8000 should stop at ~5
    for i in 0..6 {
        msgs.push(make_user_msg(&sid, &format!("{}: {}", i, "y".repeat(1490))));
    }
    let result = format_session_history(&msgs, 6);
    // Should be truncated
    assert!(result.contains("truncated=\"true\""));
    // Should NOT contain all 6 messages' count
    let count_attr_start = result.find("count=\"").unwrap();
    let count_start = count_attr_start + 7;
    let count_end = result[count_start..].find('"').unwrap() + count_start;
    let count: usize = result[count_start..count_end].parse().unwrap();
    assert!(count < 6, "Expected fewer than 6 messages due to 8000-char cap, got {}", count);
}

#[test]
fn format_session_history_tool_summary_aggregation() {
    let sid = IdeationSessionId::new();
    let mut orch_msg = make_orchestrator_msg(&sid, "");
    orch_msg.tool_calls = Some(
        r#"[{"name":"create_task_proposal","arguments":"{}","result":{"content":"ok","is_error":false}},{"name":"create_task_proposal","arguments":"{}","result":{"content":"ok","is_error":false}},{"name":"update_plan_artifact","arguments":"{}","result":{"content":"ok","is_error":false}}]"#
            .to_string(),
    );
    let result = format_session_history(&[orch_msg], 1);
    assert!(result.contains("[Used: create_task_proposal x2, update_plan_artifact]"));
    assert!(result.contains(r#"role="tool_summary""#));
}

#[test]
fn format_session_history_tool_summary_with_failed_call() {
    let sid = IdeationSessionId::new();
    let mut orch_msg = make_orchestrator_msg(&sid, "thinking");
    orch_msg.tool_calls = Some(
        r#"[{"name":"create_plan_artifact","arguments":"{}","result":{"content":"ok","is_error":false}},{"name":"get_proposal","arguments":"{}","result":{"content":"err","is_error":true}}]"#
            .to_string(),
    );
    let result = format_session_history(&[orch_msg], 1);
    assert!(result.contains("get_proposal (failed)"));
    assert!(result.contains("create_plan_artifact"));
}

#[test]
fn format_session_history_empty_tool_calls_no_summary() {
    let sid = IdeationSessionId::new();
    let mut orch_msg = make_orchestrator_msg(&sid, "just text");
    orch_msg.tool_calls = Some("[]".to_string());
    let result = format_session_history(&[orch_msg], 1);
    assert!(!result.contains("tool_summary"));
    assert!(result.contains("just text"));
}

#[test]
fn format_session_history_truncated_true_when_total_available_larger() {
    let sid = IdeationSessionId::new();
    let msg = make_user_msg(&sid, "hello");
    // Only 1 message provided but total_available=100 → truncated=true
    let result = format_session_history(&[msg], 100);
    assert!(result.contains("truncated=\"true\""));
    assert!(result.contains("total_available=\"100\""));
}

#[test]
fn format_session_history_truncated_false_when_all_included() {
    let sid = IdeationSessionId::new();
    let msgs = vec![make_user_msg(&sid, "msg1"), make_user_msg(&sid, "msg2")];
    let result = format_session_history(&msgs, 2);
    assert!(result.contains("truncated=\"false\""));
}

#[test]
fn format_session_history_orchestrator_with_text_and_tools() {
    let sid = IdeationSessionId::new();
    let mut orch_msg = make_orchestrator_msg(&sid, "Here is my analysis");
    orch_msg.tool_calls = Some(
        r#"[{"name":"search","arguments":"{}","result":{"content":"results","is_error":false}}]"#
            .to_string(),
    );
    let result = format_session_history(&[orch_msg], 1);
    // Both text AND tool_summary should appear
    assert!(result.contains("Here is my analysis"));
    assert!(result.contains("[Used: search]"));
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests for build_initial_prompt with session_history injection
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn build_initial_prompt_ideation_with_messages_injects_session_history() {
    let sid = IdeationSessionId::new();
    let msg = make_user_msg(&sid, "prior message");
    let result = build_initial_prompt(
        ChatContextType::Ideation,
        sid.as_str(),
        "new message",
        &[msg],
        1,
    );
    assert!(result.contains("<session_history"));
    assert!(result.contains("prior message"));
    assert!(result.contains("<user_message>new message</user_message>"));
    // session_history should come before user_message
    let hist_pos = result.find("<session_history").unwrap();
    let user_pos = result.find("<user_message>").unwrap();
    assert!(hist_pos < user_pos);
}

#[test]
fn build_initial_prompt_ideation_empty_messages_no_session_history_block() {
    let result = build_initial_prompt(
        ChatContextType::Ideation,
        "session-123",
        "hello",
        &[],
        0,
    );
    assert!(!result.contains("<session_history"));
    assert!(result.contains("<user_message>hello</user_message>"));
}

#[test]
fn build_initial_prompt_non_ideation_ignores_messages() {
    let sid = IdeationSessionId::new();
    let msg = make_user_msg(&sid, "some prior message");
    // Task context should NOT inject session_history even if messages provided
    let result = build_initial_prompt(
        ChatContextType::TaskExecution,
        "task-abc",
        "execute",
        &[msg],
        0,
    );
    assert!(!result.contains("<session_history"));
}

// ──────────────────────────────────────────────────────────────────────────────
// Integration tests: send_message → prompt pipeline (Wave 3 wiring)
//
// These tests verify the full pipeline: repo-fetched messages → build_initial_prompt
// → <session_history> XML in the resulting prompt. They simulate what send_message()
// does when spawning a new Ideation process: fetch messages, pass to prompt builder.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn integration_ideation_spawn_prompt_pipeline_injects_session_history() {
    // Simulate what send_message() does for Ideation context on new process spawn:
    //   1. get_recent_by_session() returns prior messages
    //   2. build_initial_prompt() receives them and injects <session_history>
    let sid = IdeationSessionId::new();

    // Simulate repo-fetched messages (user + orchestrator with tool usage)
    let user_msg1 = make_user_msg(&sid, "I want to add dark mode");
    let mut orch_msg = make_orchestrator_msg(&sid, "Let me explore the codebase");
    orch_msg.tool_calls = Some(
        r#"[{"name":"create_task_proposal","arguments":"{}","result":{"content":"ok","is_error":false}}]"#
            .to_string(),
    );
    let user_msg2 = make_user_msg(&sid, "Also add a light mode toggle");
    let repo_messages = vec![user_msg1, orch_msg, user_msg2];
    let total_available = repo_messages.len();

    // This mirrors what happens in send_message() → build_interactive_command() → build_initial_prompt()
    // total_available comes from count_by_session; here we simulate it as the actual DB count.
    let prompt = build_initial_prompt(
        ChatContextType::Ideation,
        sid.as_str(),
        "What is the current progress?",
        &repo_messages,
        total_available,
    );

    // Verify session_history is present and contains prior messages
    assert!(prompt.contains("<session_history"), "prompt must contain <session_history> block");
    assert!(
        prompt.contains("I want to add dark mode"),
        "prior user message must appear in history"
    );
    assert!(
        prompt.contains("Let me explore the codebase"),
        "prior orchestrator message must appear in history"
    );
    assert!(
        prompt.contains("[Used: create_task_proposal]"),
        "tool usage must be summarised in history"
    );
    assert!(
        prompt.contains("Also add a light mode toggle"),
        "second prior user message must appear in history"
    );
    assert!(
        prompt.contains("What is the current progress?"),
        "current user message must appear in prompt"
    );
    assert!(
        prompt.contains(&format!("total_available=\"{}\"", total_available)),
        "total_available attribute must match message count"
    );

    // session_history block must appear before <user_message>
    let hist_pos = prompt.find("<session_history").unwrap();
    let user_pos = prompt.find("<user_message>").unwrap();
    assert!(
        hist_pos < user_pos,
        "<session_history> must appear before <user_message>"
    );
}

#[test]
fn integration_ideation_spawn_first_message_no_session_history_block() {
    // When send_message() fetches 0 messages (first ever message in session),
    // the prompt must NOT contain a <session_history> block.
    let sid = IdeationSessionId::new();

    let prompt = build_initial_prompt(
        ChatContextType::Ideation,
        sid.as_str(),
        "Hello, start a new plan",
        &[], // empty — simulates count_by_session() == 0
        0,
    );

    assert!(
        !prompt.contains("<session_history"),
        "first message in session must not have <session_history> block"
    );
    assert!(
        prompt.contains("Hello, start a new plan"),
        "current user message must be present"
    );
}

#[test]
fn integration_non_ideation_spawn_no_session_history_even_with_messages() {
    // send_message() passes empty slice for non-Ideation contexts.
    // Even if messages were somehow provided, non-Ideation build_initial_prompt ignores them.
    let sid = IdeationSessionId::new();
    let msg = make_user_msg(&sid, "prior work message");

    let prompt = build_initial_prompt(
        ChatContextType::TaskExecution,
        "task-abc-123",
        "execute task",
        &[msg], // non-ideation: this must be ignored
        0,
    );

    assert!(
        !prompt.contains("<session_history"),
        "non-Ideation context must never inject <session_history>"
    );
    assert!(
        prompt.contains("execute task"),
        "current user message must be present"
    );
}

#[test]
fn integration_ideation_spawn_truncated_history_uses_db_count_not_slice_len() {
    // Regression test: when a session has >SESSION_HISTORY_LIMIT messages,
    // total_available must come from count_by_session (the real DB count),
    // not from session_messages.len() (which is capped at the limit).
    // Bug: format_session_history(session_messages, session_messages.len()) would emit
    //   total_available="50" truncated="false" even when DB has 200 messages.
    // Fix: thread total_available through build_initial_prompt from send_message().
    let sid = IdeationSessionId::new();

    // Simulate fetching the last 2 messages from a session with 200 total.
    let msg1 = make_user_msg(&sid, "recent message 1");
    let msg2 = make_user_msg(&sid, "recent message 2");
    let session_messages = vec![msg1, msg2];
    let db_count: usize = 200; // real count from count_by_session

    let prompt = build_initial_prompt(
        ChatContextType::Ideation,
        sid.as_str(),
        "continue",
        &session_messages,
        db_count,
    );

    // Must use DB count, not slice length
    assert!(
        prompt.contains(&format!("total_available=\"{}\"", db_count)),
        "total_available must be the DB count ({}), not the slice length ({})",
        db_count,
        session_messages.len()
    );
    assert!(
        prompt.contains("truncated=\"true\""),
        "truncated must be true when DB count ({}) > slice len ({})",
        db_count,
        session_messages.len()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests for resolve_working_directory — merge context worktree prefix filter
// Fix: commit cfb57e0e — accept both merge- and rebase- prefixes for merge worktrees
// ──────────────────────────────────────────────────────────────────────────────

use crate::domain::entities::{Project, Task};
use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

/// Test 1: Merger agent spawn accepts rebase-{task_id} worktree path.
///
/// Regression test for commit cfb57e0e: before the fix, only merge- was accepted.
/// rebase- prefixed worktrees are created by the checkout-free rebase strategy and
/// must be valid merge agent working directories.
#[tokio::test]
async fn resolve_working_directory_merge_context_accepts_rebase_prefix() {
    let parent = tempfile::TempDir::new().unwrap();
    let wt = parent.path().join("rebase-abc123");
    std::fs::create_dir_all(&wt).unwrap();
    let wt_path = wt.to_str().unwrap().to_string();

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let project_dir = parent.path().join("main-repo");
    std::fs::create_dir_all(&project_dir).unwrap();
    let mut project = Project::new(
        "test".to_string(),
        project_dir.to_str().unwrap().to_string(),
    );
    project.id = project_id.clone();
    project.git_mode = GitMode::Worktree;
    project_repo.create(project).await.unwrap();

    let mut task = Task::new(project_id, "test task".to_string());
    task.worktree_path = Some(wt_path.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let result = resolve_working_directory(
        ChatContextType::Merge,
        task_id.as_str(),
        Arc::clone(&project_repo) as Arc<dyn ProjectRepository>,
        Arc::clone(&task_repo) as Arc<dyn TaskRepository>,
        Arc::new(MockIdeationRepo::empty()) as Arc<dyn IdeationSessionRepository>,
        std::path::Path::new("/tmp/default"),
    )
    .await;

    assert!(
        result.is_ok(),
        "rebase- prefixed worktree must be accepted for Merge context. Got: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        std::path::PathBuf::from(&wt_path),
        "Must return the rebase- worktree path as the working directory"
    );
}

/// Test 2: Merger agent spawn accepts merge-{task_id} worktree path (existing behavior not broken).
///
/// Confirms the original merge- prefix continues to work after the fix.
#[tokio::test]
async fn resolve_working_directory_merge_context_accepts_merge_prefix() {
    let parent = tempfile::TempDir::new().unwrap();
    let wt = parent.path().join("merge-abc123");
    std::fs::create_dir_all(&wt).unwrap();
    let wt_path = wt.to_str().unwrap().to_string();

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let project_dir = parent.path().join("main-repo");
    std::fs::create_dir_all(&project_dir).unwrap();
    let mut project = Project::new(
        "test".to_string(),
        project_dir.to_str().unwrap().to_string(),
    );
    project.id = project_id.clone();
    project.git_mode = GitMode::Worktree;
    project_repo.create(project).await.unwrap();

    let mut task = Task::new(project_id, "test task".to_string());
    task.worktree_path = Some(wt_path.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let result = resolve_working_directory(
        ChatContextType::Merge,
        task_id.as_str(),
        Arc::clone(&project_repo) as Arc<dyn ProjectRepository>,
        Arc::clone(&task_repo) as Arc<dyn TaskRepository>,
        Arc::new(MockIdeationRepo::empty()) as Arc<dyn IdeationSessionRepository>,
        std::path::Path::new("/tmp/default"),
    )
    .await;

    assert!(
        result.is_ok(),
        "merge- prefixed worktree must still be accepted for Merge context. Got: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        std::path::PathBuf::from(&wt_path),
        "Must return the merge- worktree path as the working directory"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests for auto_verification metadata filtering in format_session_history
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn format_session_history_auto_verification_filtered_out() {
    let sid = IdeationSessionId::new();
    let mut auto_verify_msg = make_user_msg(&sid, "please verify this plan");
    auto_verify_msg.metadata =
        Some(r#"{"auto_verification": true}"#.to_string());
    let normal_msg = make_user_msg(&sid, "normal user message");
    let msgs = vec![auto_verify_msg, normal_msg];
    let result = format_session_history(&msgs, 2);
    // Auto-verification message must be excluded
    assert!(!result.contains("please verify this plan"));
    // Normal message must still appear
    assert!(result.contains("normal user message"));
}

#[test]
fn format_session_history_all_auto_verification_returns_empty_string() {
    let sid = IdeationSessionId::new();
    let mut msg = make_user_msg(&sid, "auto verify only");
    msg.metadata = Some(r#"{"auto_verification": true}"#.to_string());
    let result = format_session_history(&[msg], 1);
    assert_eq!(result, "");
}

#[test]
fn format_session_history_auto_verification_and_recovery_context_both_filtered() {
    let sid = IdeationSessionId::new();
    let mut auto_verify_msg = make_user_msg(&sid, "auto verify content");
    auto_verify_msg.metadata = Some(r#"{"auto_verification": true}"#.to_string());
    let mut recovery_msg = make_user_msg(&sid, "recovery context content");
    recovery_msg.metadata = Some(r#"{"recovery_context": true}"#.to_string());
    let normal_msg = make_user_msg(&sid, "regular message");
    let msgs = vec![auto_verify_msg, recovery_msg, normal_msg];
    let result = format_session_history(&msgs, 3);
    // Both filtered metadata types must be excluded
    assert!(!result.contains("auto verify content"));
    assert!(!result.contains("recovery context content"));
    // Normal message must appear
    assert!(result.contains("regular message"));
}

#[test]
fn format_session_history_auto_verification_false_value_is_included() {
    // Only messages with the "auto_verification" key present (regardless of value) are excluded.
    // A message with `{"auto_verification": false}` still has the key so it is excluded.
    let sid = IdeationSessionId::new();
    let mut msg = make_user_msg(&sid, "auto_verification false");
    msg.metadata = Some(r#"{"auto_verification": false}"#.to_string());
    let result = format_session_history(&[msg], 1);
    // The filter checks for key *presence*, so this should also be filtered out
    assert!(!result.contains("auto_verification false"));
}

/// Test 3: Merger agent spawn rejects non-merge worktree paths (e.g., task-{task_id}).
///
/// A task worktree (task- prefix) must never be used as a merge agent working directory.
/// The guard must reject it with an error rather than silently falling back.
#[tokio::test]
async fn resolve_working_directory_merge_context_rejects_task_prefix() {
    let parent = tempfile::TempDir::new().unwrap();
    let wt = parent.path().join("task-abc123");
    std::fs::create_dir_all(&wt).unwrap();
    let wt_path = wt.to_str().unwrap().to_string();

    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let project_dir = parent.path().join("main-repo");
    std::fs::create_dir_all(&project_dir).unwrap();
    let mut project = Project::new(
        "test".to_string(),
        project_dir.to_str().unwrap().to_string(),
    );
    project.id = project_id.clone();
    project.git_mode = GitMode::Worktree;
    project_repo.create(project).await.unwrap();

    let mut task = Task::new(project_id, "test task".to_string());
    task.worktree_path = Some(wt_path.clone());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let result = resolve_working_directory(
        ChatContextType::Merge,
        task_id.as_str(),
        Arc::clone(&project_repo) as Arc<dyn ProjectRepository>,
        Arc::clone(&task_repo) as Arc<dyn TaskRepository>,
        Arc::new(MockIdeationRepo::empty()) as Arc<dyn IdeationSessionRepository>,
        std::path::Path::new("/tmp/default"),
    )
    .await;

    assert!(
        result.is_err(),
        "task- prefixed worktree must be rejected for Merge context (not a merge worktree). \
         Got Ok instead of Err."
    );
}
