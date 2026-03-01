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
use crate::domain::entities::{IdeationSession, IdeationSessionStatus, ProjectId};
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
    )
    .await;

    // Test passes if no panic occurred
}

// Tests for build_resume_initial_prompt

#[test]
fn test_build_resume_initial_prompt_ideation_includes_context_id_and_recovery_note() {
    let context_id = "test-session-123";
    let user_message = "hello";
    let result = build_resume_initial_prompt(ChatContextType::Ideation, context_id, user_message);
    assert!(result.contains(&format!("<context_id>{}</context_id>", context_id)));
    assert!(result.contains("<recovery_note>"));
    assert!(result.contains("get_session_messages"));
    assert!(result.contains(&format!("<user_message>{}</user_message>", user_message)));
}

#[test]
fn test_build_resume_initial_prompt_task_includes_context_id_no_recovery_note() {
    let context_id = "task-abc";
    let user_message = "hello";
    let result = build_resume_initial_prompt(ChatContextType::Task, context_id, user_message);
    assert!(result.contains(&format!("<task_id>{}</task_id>", context_id)));
    assert!(!result.contains("<recovery_note>"));
    assert!(result.contains(&format!("<user_message>{}</user_message>", user_message)));
}

#[test]
fn test_build_resume_initial_prompt_project_includes_context_id_no_recovery_note() {
    let context_id = "project-xyz";
    let user_message = "hello";
    let result = build_resume_initial_prompt(ChatContextType::Project, context_id, user_message);
    assert!(result.contains(&format!("<project_id>{}</project_id>", context_id)));
    assert!(!result.contains("<recovery_note>"));
}

#[test]
fn test_build_resume_initial_prompt_task_execution_delegates_to_initial_prompt() {
    let context_id = "task-exec-123";
    let user_message = "execute";
    let resume =
        build_resume_initial_prompt(ChatContextType::TaskExecution, context_id, user_message);
    let initial = build_initial_prompt(ChatContextType::TaskExecution, context_id, user_message);
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
    )
    .await;

    // Test passes if no panics occurred
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
