// TeamService — wraps TeamStateTracker + AppHandle to emit events on mutations
//
// Pattern: service wrapper (like TransitionHandler). Every mutation delegates to
// TeamStateTracker then emits the corresponding team:* event via AppHandle.
// Read-only methods delegate directly without emission.
// Persistence: fire-and-forget writes to session/message repos (tracing::warn on failure).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use tauri::AppHandle;
use tracing::warn;

use crate::domain::entities::team::{
    TeamMessageRecord, TeamSession, TeamSessionId, TeammateSnapshot,
};
use crate::domain::repositories::{TeamMessageRepository, TeamSessionRepository};

use super::team_events;
use super::team_state_tracker::{
    TeamMessage, TeamMessageResponse, TeamMessageType, TeamStateTracker, TeamStatusResponse,
    TeamTrackerError, TeammateCost, TeammateCostResponse, TeammateHandle, TeammateStatus,
};

/// Service layer wrapping TeamStateTracker with event emission.
///
/// Holds an `Arc<TeamStateTracker>` (shared state) and an optional `AppHandle`
/// for emitting Tauri events to the frontend. When `app_handle` is `None`
/// (e.g. in tests), mutations still succeed but events are silently skipped.
/// Repos are optional — when present, mutations persist to DB (fire-and-forget).
pub struct TeamService {
    tracker: Arc<TeamStateTracker>,
    app_handle: Option<AppHandle>,
    session_repo: Option<Arc<dyn TeamSessionRepository>>,
    message_repo: Option<Arc<dyn TeamMessageRepository>>,
    /// Cache: team_name → TeamSessionId (avoids repeated DB lookups)
    session_id_cache: Arc<RwLock<HashMap<String, TeamSessionId>>>,
}

impl TeamService {
    /// Create a new TeamService with event emission.
    pub fn new(tracker: Arc<TeamStateTracker>, app_handle: AppHandle) -> Self {
        Self {
            tracker,
            app_handle: Some(app_handle),
            session_repo: None,
            message_repo: None,
            session_id_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a TeamService with event emission and persistence repos.
    pub fn new_with_repos(
        tracker: Arc<TeamStateTracker>,
        app_handle: AppHandle,
        session_repo: Arc<dyn TeamSessionRepository>,
        message_repo: Arc<dyn TeamMessageRepository>,
    ) -> Self {
        Self {
            tracker,
            app_handle: Some(app_handle),
            session_repo: Some(session_repo),
            message_repo: Some(message_repo),
            session_id_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a TeamService without event emission (for tests).
    pub fn new_without_events(tracker: Arc<TeamStateTracker>) -> Self {
        Self {
            tracker,
            app_handle: None,
            session_repo: None,
            message_repo: None,
            session_id_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Access the underlying tracker (for read-only delegation or handle ops).
    pub fn tracker(&self) -> &TeamStateTracker {
        &self.tracker
    }

    /// Get an Arc reference to the underlying tracker (for spawning stream processors).
    pub fn tracker_arc(&self) -> Arc<TeamStateTracker> {
        self.tracker.clone()
    }

    // ========================================================================
    // Mutation methods (tracker op + event emit)
    // ========================================================================

    /// Create a new team and emit team:created.
    pub async fn create_team(
        &self,
        name: &str,
        context_id: &str,
        context_type: &str,
    ) -> Result<(), TeamTrackerError> {
        self.tracker.create_team(name, context_id, context_type).await?;

        // Persist session to DB
        if let Some(ref repo) = self.session_repo {
            let session = TeamSession::new(name, context_id, context_type);
            let sid = session.id.clone();
            if let Err(e) = repo.create(session).await {
                warn!("Failed to persist team session: {e}");
            } else {
                self.session_id_cache.write().await.insert(name.to_string(), sid);
            }
        }

        if let Some(ref handle) = self.app_handle {
            team_events::emit_team_created(handle, name, context_id, context_type);
        }
        Ok(())
    }

    /// Add a teammate and emit team:teammate_spawned.
    pub async fn add_teammate(
        &self,
        team_name: &str,
        name: &str,
        color: &str,
        model: &str,
        role: &str,
    ) -> Result<(), TeamTrackerError> {
        self.tracker
            .add_teammate(team_name, name, color, model, role)
            .await?;

        self.persist_teammates(team_name).await;

        if let Some(ref handle) = self.app_handle {
            let (ctx_type, ctx_id) = self.get_team_context(team_name).await?;
            team_events::emit_teammate_spawned(
                handle, team_name, name, color, model, role, &ctx_type, &ctx_id,
            );
        }
        Ok(())
    }

    /// Update teammate status and emit the corresponding status event.
    pub async fn update_teammate_status(
        &self,
        team_name: &str,
        teammate_name: &str,
        status: TeammateStatus,
    ) -> Result<(), TeamTrackerError> {
        self.tracker
            .update_teammate_status(team_name, teammate_name, status)
            .await?;

        self.persist_teammates(team_name).await;

        if let Some(ref handle) = self.app_handle {
            let (ctx_type, ctx_id) = self.get_team_context(team_name).await?;
            team_events::emit_teammate_status_change(
                handle,
                team_name,
                teammate_name,
                status,
                &ctx_type,
                &ctx_id,
            );
        }
        Ok(())
    }

    /// Update teammate cost and emit team:cost_update.
    pub async fn update_teammate_cost(
        &self,
        team_name: &str,
        teammate_name: &str,
        cost: TeammateCost,
    ) -> Result<(), TeamTrackerError> {
        let input_tokens = cost.input_tokens;
        let output_tokens = cost.output_tokens;
        let estimated_usd = cost.estimated_usd;

        self.tracker
            .update_teammate_cost(team_name, teammate_name, cost)
            .await?;

        self.persist_teammates(team_name).await;

        if let Some(ref handle) = self.app_handle {
            let (ctx_type, ctx_id) = self.get_team_context(team_name).await?;
            team_events::emit_team_cost_update(
                handle,
                team_name,
                teammate_name,
                input_tokens,
                output_tokens,
                estimated_usd,
                &ctx_type,
                &ctx_id,
            );
        }
        Ok(())
    }

    /// Send a user message and emit team:message.
    pub async fn send_user_message(
        &self,
        team_name: &str,
        content: &str,
    ) -> Result<TeamMessage, TeamTrackerError> {
        let msg = self.tracker.send_user_message(team_name, content).await?;

        self.persist_message(team_name, &msg).await;

        if let Some(ref handle) = self.app_handle {
            let (ctx_type, ctx_id) = self.get_team_context(team_name).await?;
            team_events::emit_team_message(handle, &msg, &ctx_type, &ctx_id);
        }
        Ok(msg)
    }

    /// Add a teammate message and emit team:message.
    pub async fn add_teammate_message(
        &self,
        team_name: &str,
        sender: &str,
        recipient: Option<&str>,
        content: &str,
        message_type: TeamMessageType,
    ) -> Result<TeamMessage, TeamTrackerError> {
        let msg = self
            .tracker
            .add_teammate_message(team_name, sender, recipient, content, message_type)
            .await?;

        self.persist_message(team_name, &msg).await;

        if let Some(ref handle) = self.app_handle {
            let (ctx_type, ctx_id) = self.get_team_context(team_name).await?;
            team_events::emit_team_message(handle, &msg, &ctx_type, &ctx_id);
        }
        Ok(msg)
    }

    /// Stop a specific teammate and emit team:teammate_shutdown.
    pub async fn stop_teammate(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<(), TeamTrackerError> {
        // Capture context before the mutation (team must exist)
        let ctx = if self.app_handle.is_some() {
            Some(self.get_team_context(team_name).await?)
        } else {
            None
        };

        self.tracker.stop_teammate(team_name, teammate_name).await?;

        if let (Some(ref handle), Some((ctx_type, ctx_id))) = (&self.app_handle, ctx) {
            team_events::emit_teammate_shutdown(
                handle,
                team_name,
                teammate_name,
                &ctx_type,
                &ctx_id,
            );
        }
        Ok(())
    }

    /// Stop all teammates and emit per-teammate shutdown + team:disbanded events.
    pub async fn stop_team(&self, team_name: &str) -> Result<(), TeamTrackerError> {
        // Capture teammate names and context before mutation
        let (teammate_names, ctx) = if self.app_handle.is_some() {
            let status = self.tracker.get_team_status(team_name).await?;
            let names: Vec<String> = status.teammates.iter().map(|t| t.name.clone()).collect();
            let ctx = (status.context_type.clone(), status.context_id.clone());
            (names, Some(ctx))
        } else {
            (vec![], None)
        };

        self.tracker.stop_team(team_name).await?;

        if let (Some(ref handle), Some((ctx_type, ctx_id))) = (&self.app_handle, ctx) {
            for name in &teammate_names {
                team_events::emit_teammate_shutdown(
                    handle, team_name, name, &ctx_type, &ctx_id,
                );
            }
        }
        Ok(())
    }

    /// Disband a team (stop all + mark disbanded) and emit team:disbanded.
    pub async fn disband_team(&self, team_name: &str) -> Result<(), TeamTrackerError> {
        // Capture context before mutation
        let ctx = if self.app_handle.is_some() {
            Some(self.get_team_context(team_name).await?)
        } else {
            None
        };

        self.tracker.disband_team(team_name).await?;

        // Persist disbanded state
        if let Some(ref repo) = self.session_repo {
            if let Some(sid) = self.cached_session_id(team_name).await {
                if let Err(e) = repo.set_disbanded(&sid).await {
                    warn!("Failed to persist team disbanded: {e}");
                }
            }
        }

        if let (Some(ref handle), Some((ctx_type, ctx_id))) = (&self.app_handle, ctx) {
            team_events::emit_team_disbanded(handle, team_name, &ctx_type, &ctx_id);
        }
        Ok(())
    }

    // ========================================================================
    // Delegate-only methods (no event emission)
    // ========================================================================

    /// Set the teammate handle (process + stream task).
    pub async fn set_teammate_handle(
        &self,
        team_name: &str,
        teammate_name: &str,
        handle: TeammateHandle,
    ) -> Result<(), TeamTrackerError> {
        self.tracker
            .set_teammate_handle(team_name, teammate_name, handle)
            .await
    }

    /// Get team status (serializable snapshot).
    pub async fn get_team_status(
        &self,
        team_name: &str,
    ) -> Result<TeamStatusResponse, TeamTrackerError> {
        self.tracker.get_team_status(team_name).await
    }

    /// Get teammate cost.
    pub async fn get_teammate_cost(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<TeammateCostResponse, TeamTrackerError> {
        self.tracker.get_teammate_cost(team_name, teammate_name).await
    }

    /// Get team messages (serializable).
    pub async fn get_team_messages(
        &self,
        team_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessageResponse>, TeamTrackerError> {
        self.tracker.get_team_messages(team_name, limit).await
    }

    /// List all active team names.
    pub async fn list_teams(&self) -> Vec<String> {
        self.tracker.list_teams().await
    }

    /// Check if a team exists.
    pub async fn team_exists(&self, team_name: &str) -> bool {
        self.tracker.team_exists(team_name).await
    }

    /// Send a message to a teammate's stdin (interactive mode).
    pub async fn send_stdin_message(
        &self,
        team_name: &str,
        teammate_name: &str,
        message: &str,
    ) -> Result<(), TeamTrackerError> {
        self.tracker
            .send_stdin_message(team_name, teammate_name, message)
            .await
    }

    /// Remove a teammate from a team.
    pub async fn remove_teammate(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<(), TeamTrackerError> {
        // Capture context before mutation
        let ctx = if self.app_handle.is_some() {
            Some(self.get_team_context(team_name).await?)
        } else {
            None
        };

        self.tracker.remove_teammate(team_name, teammate_name).await?;

        if let (Some(ref handle), Some((ctx_type, ctx_id))) = (&self.app_handle, ctx) {
            team_events::emit_teammate_shutdown(
                handle,
                team_name,
                teammate_name,
                &ctx_type,
                &ctx_id,
            );
        }
        Ok(())
    }

    /// Get teammate count for a team.
    pub async fn get_teammate_count(&self, team_name: &str) -> Result<usize, TeamTrackerError> {
        self.tracker.get_teammate_count(team_name).await
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Look up context_type and context_id from the team's TeamState.
    async fn get_team_context(
        &self,
        team_name: &str,
    ) -> Result<(String, String), TeamTrackerError> {
        let status = self.tracker.get_team_status(team_name).await?;
        Ok((status.context_type, status.context_id))
    }

    /// Get cached session ID for a team.
    async fn cached_session_id(&self, team_name: &str) -> Option<TeamSessionId> {
        self.session_id_cache.read().await.get(team_name).cloned()
    }

    /// Snapshot current teammates from tracker and persist to DB.
    async fn persist_teammates(&self, team_name: &str) {
        let repo = match self.session_repo {
            Some(ref r) => r,
            None => return,
        };
        let sid = match self.cached_session_id(team_name).await {
            Some(s) => s,
            None => return,
        };
        // Get current teammate list from tracker
        let status = match self.tracker.get_team_status(team_name).await {
            Ok(s) => s,
            Err(_) => return,
        };
        let snapshots: Vec<TeammateSnapshot> = status
            .teammates
            .iter()
            .map(|t| TeammateSnapshot {
                name: t.name.clone(),
                color: t.color.clone(),
                model: t.model.clone(),
                role: t.role.clone(),
                status: t.status.to_string(),
                cost: t.cost.clone(),
                spawned_at: t.spawned_at.clone(),
                last_activity_at: t.last_activity_at.clone(),
            })
            .collect();
        if let Err(e) = repo.update_teammates(&sid, &snapshots).await {
            warn!("Failed to persist teammates: {e}");
        }
    }

    /// Persist a team message to DB.
    async fn persist_message(&self, team_name: &str, msg: &TeamMessage) {
        let repo = match self.message_repo {
            Some(ref r) => r,
            None => return,
        };
        let sid = match self.cached_session_id(team_name).await {
            Some(s) => s,
            None => return,
        };
        let mut record = TeamMessageRecord::new(sid, &msg.sender, &msg.content);
        record.recipient = msg.recipient.clone();
        record.message_type = match msg.message_type {
            TeamMessageType::UserMessage => "user_message".to_string(),
            TeamMessageType::TeammateMessage => "teammate_message".to_string(),
            TeamMessageType::Broadcast => "broadcast".to_string(),
            TeamMessageType::System => "system".to_string(),
        };
        record.created_at = msg.timestamp;
        if let Err(e) = repo.create(record).await {
            warn!("Failed to persist team message: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> TeamService {
        TeamService::new_without_events(Arc::new(TeamStateTracker::new()))
    }

    #[tokio::test]
    async fn test_create_team() {
        let svc = test_service();
        svc.create_team("alpha", "session-1", "ideation")
            .await
            .unwrap();

        assert!(svc.team_exists("alpha").await);
    }

    #[tokio::test]
    async fn test_create_duplicate_team_fails() {
        let svc = test_service();
        svc.create_team("alpha", "s-1", "ideation").await.unwrap();

        let err = svc.create_team("alpha", "s-2", "ideation").await;
        assert!(matches!(err, Err(TeamTrackerError::TeamAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_add_teammate_and_status() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "researcher", "#ff6b35", "opus", "explore")
            .await
            .unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.teammates.len(), 1);
        assert_eq!(status.teammates[0].name, "researcher");
        assert_eq!(status.teammates[0].status, TeammateStatus::Spawning);
        assert_eq!(status.context_id, "ctx-1");
        assert_eq!(status.context_type, "ideation");
    }

    #[tokio::test]
    async fn test_update_teammate_status() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "worker", "#00ff00", "sonnet", "code")
            .await
            .unwrap();

        svc.update_teammate_status("t1", "worker", TeammateStatus::Running)
            .await
            .unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.teammates[0].status, TeammateStatus::Running);
    }

    #[tokio::test]
    async fn test_update_teammate_cost() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "r", "#ff6b35", "opus", "explore")
            .await
            .unwrap();

        let cost = TeammateCost {
            input_tokens: 1000,
            output_tokens: 500,
            cache_creation_tokens: 200,
            cache_read_tokens: 100,
            estimated_usd: 0.05,
        };
        svc.update_teammate_cost("t1", "r", cost).await.unwrap();

        let resp = svc.get_teammate_cost("t1", "r").await.unwrap();
        assert_eq!(resp.input_tokens, 1000);
        assert_eq!(resp.estimated_usd, 0.05);
    }

    #[tokio::test]
    async fn test_send_user_message() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

        let msg = svc.send_user_message("t1", "Hello").await.unwrap();
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[tokio::test]
    async fn test_add_teammate_message() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

        let msg = svc
            .add_teammate_message(
                "t1",
                "researcher",
                Some("planner"),
                "Found results",
                TeamMessageType::TeammateMessage,
            )
            .await
            .unwrap();
        assert_eq!(msg.sender, "researcher");
        assert_eq!(msg.recipient, Some("planner".to_string()));
    }

    #[tokio::test]
    async fn test_stop_teammate() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        svc.stop_teammate("t1", "w").await.unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.teammates[0].status, TeammateStatus::Shutdown);
    }

    #[tokio::test]
    async fn test_stop_team() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "w1", "#ff0000", "sonnet", "code")
            .await
            .unwrap();
        svc.add_teammate("t1", "w2", "#00ff00", "sonnet", "code")
            .await
            .unwrap();

        svc.stop_team("t1").await.unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.phase, super::super::team_state_tracker::TeamPhase::Winding);
        for t in &status.teammates {
            assert_eq!(t.status, TeammateStatus::Shutdown);
        }
    }

    #[tokio::test]
    async fn test_disband_team() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        svc.disband_team("t1").await.unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.phase, super::super::team_state_tracker::TeamPhase::Disbanded);
    }

    #[tokio::test]
    async fn test_get_messages_with_limit() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

        for i in 0..5 {
            svc.send_user_message("t1", &format!("Msg {}", i))
                .await
                .unwrap();
        }

        let all = svc.get_team_messages("t1", None).await.unwrap();
        assert_eq!(all.len(), 5);

        let limited = svc.get_team_messages("t1", Some(2)).await.unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[tokio::test]
    async fn test_list_teams() {
        let svc = test_service();
        svc.create_team("a", "ctx-1", "ideation").await.unwrap();
        svc.create_team("b", "ctx-2", "task").await.unwrap();

        let teams = svc.list_teams().await;
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_teammate() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        svc.remove_teammate("t1", "w").await.unwrap();

        let status = svc.get_team_status("t1").await.unwrap();
        assert_eq!(status.teammates.len(), 0);
    }

    #[tokio::test]
    async fn test_teammate_count() {
        let svc = test_service();
        svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
        svc.add_teammate("t1", "a", "#ff0000", "sonnet", "code")
            .await
            .unwrap();
        svc.add_teammate("t1", "b", "#00ff00", "opus", "explore")
            .await
            .unwrap();

        assert_eq!(svc.get_teammate_count("t1").await.unwrap(), 2);
    }
}
