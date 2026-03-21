// In-memory IdeationSessionRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, SessionOrigin,
    VerificationMetadata, VerificationStatus,
};
use crate::domain::repositories::ideation_session_repository::{
    IdeationSessionWithProgress, SessionGroupCounts,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::{AppError, AppResult};

/// In-memory implementation of IdeationSessionRepository for testing
pub struct MemoryIdeationSessionRepository {
    sessions: RwLock<HashMap<String, IdeationSession>>,
}

impl MemoryIdeationSessionRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryIdeationSessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdeationSessionRepository for MemoryIdeationSessionRepository {
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
        self.sessions
            .write()
            .unwrap()
            .insert(session.id.to_string(), session.clone());
        Ok(session)
    }

    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        Ok(self.sessions.read().unwrap().get(&id.to_string()).cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
        let mut sessions: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id)
            .cloned()
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.status = status;
            session.updated_at = Utc::now();
            match status {
                IdeationSessionStatus::Archived => {
                    session.archived_at = Some(Utc::now());
                    session.verification_in_progress = false;
                }
                IdeationSessionStatus::Accepted => {
                    session.converted_at = Some(Utc::now());
                }
                IdeationSessionStatus::Active => {
                    session.archived_at = None;
                    session.converted_at = None;
                }
            }
        }
        Ok(())
    }

    async fn update_title(
        &self,
        id: &IdeationSessionId,
        title: Option<String>,
        title_source: &str,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.title = title;
            session.title_source = Some(title_source.to_string());
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.plan_artifact_id =
                plan_artifact_id.map(crate::domain::entities::ArtifactId::from_string);
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
        self.sessions.write().unwrap().remove(&id.to_string());
        Ok(())
    }

    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let mut sessions: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id && s.status == IdeationSessionStatus::Active)
            .cloned()
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| &s.project_id == project_id && s.status == status)
            .count() as u32)
    }

    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.plan_artifact_id.as_ref().map(|id| id.as_str()) == Some(plan_artifact_id))
            .cloned()
            .collect())
    }

    async fn get_by_inherited_plan_artifact_id(
        &self,
        artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.inherited_plan_artifact_id
                    .as_ref()
                    .map(|id| id.as_str())
                    == Some(artifact_id)
            })
            .cloned()
            .collect())
    }

    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>> {
        let mut children: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.parent_session_id.as_ref() == Some(parent_id))
            .cloned()
            .collect();
        children.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(children)
    }

    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        let mut chain = Vec::new();
        let sessions_lock = self.sessions.read().unwrap();
        let mut current_id = session_id.clone();

        // Walk up the parent chain
        loop {
            if let Some(session) = sessions_lock.get(&current_id.to_string()) {
                if let Some(parent_id) = &session.parent_session_id {
                    current_id = parent_id.clone();
                    if let Some(parent) = sessions_lock.get(&current_id.to_string()) {
                        chain.push(parent.clone());
                    } else {
                        // Parent doesn't exist, stop here
                        break;
                    }
                } else {
                    // No parent, end of chain
                    break;
                }
            } else {
                // Session doesn't exist, stop
                break;
            }
        }

        Ok(chain)
    }

    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.parent_session_id = parent_id.cloned();
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_verification_state(
        &self,
        id: &IdeationSessionId,
        status: VerificationStatus,
        in_progress: bool,
        metadata_json: Option<String>,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.verification_status = status;
            session.verification_in_progress = in_progress;
            session.verification_metadata = metadata_json;
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn reset_verification(&self, id: &IdeationSessionId) -> AppResult<bool> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            if session.verification_in_progress {
                return Ok(false);
            }
            // ImportedVerified sessions are never reset — their pre-verified status must be preserved.
            if session.verification_status == VerificationStatus::ImportedVerified {
                return Ok(false);
            }
            session.verification_status = VerificationStatus::Unverified;
            session.verification_in_progress = false;
            session.verification_metadata = None;
            session.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn reset_and_begin_reverify(
        &self,
        session_id: &str,
    ) -> AppResult<(i32, VerificationMetadata)> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            crate::error::AppError::Database(format!("Session not found: {}", session_id))
        })?;

        // Parse existing metadata (or use default), then clear all stale fields
        let mut metadata: VerificationMetadata = session
            .verification_metadata
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        metadata.current_gaps = vec![];
        metadata.rounds = vec![];
        metadata.convergence_reason = None;
        metadata.best_round_index = None;
        metadata.current_round = 0;
        metadata.parse_failures = vec![];

        let new_gen = session.verification_generation + 1;

        session.verification_status = VerificationStatus::Reviewing;
        session.verification_in_progress = true;
        session.verification_generation = new_gen;
        session.verification_metadata = serde_json::to_string(&metadata).ok();
        session.updated_at = chrono::Utc::now();

        Ok((new_gen, metadata))
    }

    async fn get_verification_status(
        &self,
        id: &IdeationSessionId,
    ) -> AppResult<Option<(VerificationStatus, bool, Option<String>)>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .get(&id.to_string())
            .map(|s| (s.verification_status, s.verification_in_progress, s.verification_metadata.clone())))
    }

    async fn revert_plan_and_skip_verification(
        &self,
        id: &IdeationSessionId,
        new_plan_artifact_id: String,
        convergence_reason: String,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&id.to_string()) {
            session.plan_artifact_id =
                Some(crate::domain::entities::ArtifactId::from_string(new_plan_artifact_id));
            session.verification_status = VerificationStatus::Skipped;
            session.verification_in_progress = false;
            session.verification_metadata = Some(
                serde_json::json!({
                    "v": 1,
                    "current_round": 0,
                    "max_rounds": 0,
                    "rounds": [],
                    "current_gaps": [],
                    "convergence_reason": convergence_reason,
                    "best_round_index": null,
                    "parse_failures": []
                })
                .to_string(),
            );
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn revert_plan_and_skip_with_artifact(
        &self,
        session_id: &IdeationSessionId,
        new_artifact_id: String,
        _artifact_type_str: String,
        _artifact_name: String,
        _content_text: String,
        _version: u32,
        _previous_version_id: String,
        convergence_reason: String,
    ) -> AppResult<()> {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id.to_string()) {
            session.plan_artifact_id =
                Some(crate::domain::entities::ArtifactId::from_string(new_artifact_id));
            session.verification_status = VerificationStatus::Skipped;
            session.verification_in_progress = false;
            session.verification_metadata = Some(
                serde_json::json!({
                    "v": 1,
                    "current_round": 0,
                    "max_rounds": 0,
                    "rounds": [],
                    "current_gaps": [],
                    "convergence_reason": convergence_reason,
                    "best_round_index": null,
                    "parse_failures": []
                })
                .to_string(),
            );
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn increment_verification_generation(
        &self,
        _session_id: &IdeationSessionId,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn get_stale_in_progress_sessions(
        &self,
        stale_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.verification_in_progress
                    && s.updated_at < stale_before
                    && s.status != IdeationSessionStatus::Archived
            })
            .cloned()
            .collect())
    }

    async fn get_all_in_progress_sessions(&self) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.verification_in_progress && s.status != IdeationSessionStatus::Archived
            })
            .cloned()
            .collect())
    }

    async fn get_verification_children(
        &self,
        parent_session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>> {
        use crate::domain::entities::ideation::SessionPurpose;
        let mut children: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.parent_session_id.as_ref() == Some(parent_session_id)
                    && s.session_purpose == SessionPurpose::Verification
                    && s.status != IdeationSessionStatus::Archived
            })
            .cloned()
            .collect();
        children.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        children.truncate(1);
        Ok(children)
    }

    async fn get_by_project_and_status(
        &self,
        project_id: &str,
        status: &str,
        limit: u32,
    ) -> AppResult<Vec<IdeationSession>> {
        let mut sessions: Vec<_> = self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.project_id.as_str() == project_id && s.status.to_string() == status)
            .cloned()
            .collect();
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        sessions.truncate(limit as usize);
        Ok(sessions)
    }

    async fn get_group_counts(&self, project_id: &ProjectId) -> AppResult<SessionGroupCounts> {
        use crate::domain::entities::ideation::SessionPurpose;
        let sessions = self.sessions.read().unwrap();
        // Exclude verification child sessions from counts
        let project_sessions: Vec<_> = sessions
            .values()
            .filter(|s| &s.project_id == project_id && s.session_purpose != SessionPurpose::Verification)
            .collect();

        let drafts = project_sessions
            .iter()
            .filter(|s| s.status == IdeationSessionStatus::Active)
            .count() as u32;
        let archived = project_sessions
            .iter()
            .filter(|s| s.status == IdeationSessionStatus::Archived)
            .count() as u32;
        // Simplified: memory repo can't classify in_progress/done sub-groups without task repo access.
        // All accepted sessions are counted under accepted.
        let accepted = project_sessions
            .iter()
            .filter(|s| s.status == IdeationSessionStatus::Accepted)
            .count() as u32;

        Ok(SessionGroupCounts {
            drafts,
            in_progress: 0,
            accepted,
            done: 0,
            archived,
        })
    }

    async fn list_by_group(
        &self,
        project_id: &ProjectId,
        group: &str,
        offset: u32,
        limit: u32,
    ) -> AppResult<(Vec<IdeationSessionWithProgress>, u32)> {
        // Validate group
        if !matches!(group, "drafts" | "in_progress" | "accepted" | "done" | "archived") {
            return Err(AppError::Validation(format!(
                "Unknown session group: '{}'. Valid groups: drafts, in_progress, accepted, done, archived",
                group
            )));
        }

        use crate::domain::entities::ideation::SessionPurpose;
        let sessions = self.sessions.read().unwrap();

        // Simplified classification: no task repo access, so in_progress/done always empty
        // Exclude verification child sessions from results
        let mut matching: Vec<_> = sessions
            .values()
            .filter(|s| {
                if s.project_id != *project_id {
                    return false;
                }
                if s.session_purpose == SessionPurpose::Verification {
                    return false;
                }
                match group {
                    "drafts" => s.status == IdeationSessionStatus::Active,
                    "archived" => s.status == IdeationSessionStatus::Archived,
                    "accepted" => s.status == IdeationSessionStatus::Accepted,
                    "in_progress" | "done" => false, // requires task repo — not available in memory repo
                    _ => false,
                }
            })
            .cloned()
            .collect();

        matching.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let total = matching.len() as u32;

        // Count verification children for each session
        let page: Vec<_> = matching
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|session| {
                let verification_child_count = sessions
                    .values()
                    .filter(|s| {
                        s.parent_session_id.as_ref() == Some(&session.id)
                            && s.session_purpose == SessionPurpose::Verification
                    })
                    .count() as u32;
                IdeationSessionWithProgress {
                    session,
                    progress: None,
                    parent_session_title: None,
                    verification_child_count,
                }
            })
            .collect();

        Ok((page, total))
    }

    fn set_expected_proposal_count_sync(
        _conn: &Connection,
        _session_id: &str,
        _count: u32,
    ) -> AppResult<()>
    where
        Self: Sized,
    {
        Err(AppError::Infrastructure(
            "set_expected_proposal_count_sync not supported in memory repo".to_string(),
        ))
    }

    async fn set_auto_accept_status(
        &self,
        session_id: &str,
        status: &str,
        _auto_accept_started_at: Option<String>,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.values_mut().find(|s| s.id.as_str() == session_id) {
            session.auto_accept_status = Some(status.to_string());
        }
        Ok(())
    }

    fn count_active_by_session_sync(
        _conn: &Connection,
        _session_id: &str,
    ) -> AppResult<i64>
    where
        Self: Sized,
    {
        Err(AppError::Infrastructure(
            "count_active_by_session_sync not supported in memory repo".to_string(),
        ))
    }

    async fn get_by_idempotency_key(
        &self,
        api_key_id: &str,
        idempotency_key: &str,
    ) -> AppResult<Option<IdeationSession>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions
            .values()
            .find(|s| {
                s.api_key_id.as_deref() == Some(api_key_id)
                    && s.idempotency_key.as_deref() == Some(idempotency_key)
            })
            .cloned())
    }

    async fn update_external_activity_phase(
        &self,
        id: &IdeationSessionId,
        phase: &str,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(id.as_str()) {
            session.external_activity_phase = Some(phase.to_string());
            session.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_external_last_read_message_id(
        &self,
        id: &IdeationSessionId,
        message_id: &str,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(id.as_str()) {
            session.external_last_read_message_id = Some(message_id.to_string());
            session.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn list_active_external_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        let sessions = self.sessions.read().unwrap();
        let mut result: Vec<IdeationSession> = sessions
            .values()
            .filter(|s| {
                s.project_id == *project_id
                    && s.status == IdeationSessionStatus::Active
                    && s.origin == SessionOrigin::External
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn list_active_external_sessions_for_archival(
        &self,
        stale_before: Option<DateTime<Utc>>,
    ) -> AppResult<Vec<IdeationSession>> {
        let sessions = self.sessions.read().unwrap();
        let mut result: Vec<IdeationSession> = sessions
            .values()
            .filter(|s| {
                if s.origin != SessionOrigin::External || s.status != IdeationSessionStatus::Active
                {
                    return false;
                }
                let phase_matches = matches!(
                    s.external_activity_phase.as_deref(),
                    Some("created") | Some("error")
                );
                if !phase_matches {
                    return false;
                }
                if let Some(cutoff) = stale_before {
                    s.created_at < cutoff
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(result)
    }

    async fn list_stalled_external_sessions(
        &self,
        stalled_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        let sessions = self.sessions.read().unwrap();
        let mut result: Vec<IdeationSession> = sessions
            .values()
            .filter(|s| {
                if s.origin != SessionOrigin::External || s.status != IdeationSessionStatus::Active
                {
                    return false;
                }
                let phase_eligible = match s.external_activity_phase.as_deref() {
                    None => false,
                    Some("error") | Some("stalled") => false,
                    Some(_) => true,
                };
                phase_eligible && s.updated_at < stalled_before
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        Ok(result)
    }

    async fn set_dependencies_acknowledged(&self, session_id: &str) -> AppResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.values_mut().find(|s| s.id.as_str() == session_id) {
            session.dependencies_acknowledged = true;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_ideation_session_repo_tests.rs"]
mod tests;
