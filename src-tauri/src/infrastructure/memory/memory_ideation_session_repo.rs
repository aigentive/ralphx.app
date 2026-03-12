// In-memory IdeationSessionRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, VerificationStatus,
};
use crate::domain::repositories::IdeationSessionRepository;
use crate::error::AppResult;

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
            session.verification_status = VerificationStatus::Unverified;
            session.verification_in_progress = false;
            session.verification_metadata = None;
            session.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
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

    async fn get_stale_in_progress_sessions(
        &self,
        stale_before: DateTime<Utc>,
    ) -> AppResult<Vec<IdeationSession>> {
        Ok(self
            .sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.verification_in_progress && s.updated_at < stale_before)
            .cloned()
            .collect())
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
}

#[cfg(test)]
#[path = "memory_ideation_session_repo_tests.rs"]
mod tests;
