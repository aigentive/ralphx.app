// TeamStateTracker — tracks active teams, teammates, and their state
//
// Thread-safe service for managing agent team lifecycle:
// - Create/disband teams
// - Add/remove teammates
// - Track teammate status (Spawning → Running → Idle → Shutdown)
// - Store team messages for frontend consumption
// - Track cost per teammate

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

// ============================================================================
// Types
// ============================================================================

/// Status of a teammate agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeammateStatus {
    Spawning,
    Running,
    Idle,
    Completed,
    Failed,
    Shutdown,
}

impl std::fmt::Display for TeammateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeammateStatus::Spawning => write!(f, "spawning"),
            TeammateStatus::Running => write!(f, "running"),
            TeammateStatus::Idle => write!(f, "idle"),
            TeammateStatus::Completed => write!(f, "completed"),
            TeammateStatus::Failed => write!(f, "failed"),
            TeammateStatus::Shutdown => write!(f, "shutdown"),
        }
    }
}

/// Cost tracking for a teammate
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeammateCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub estimated_usd: f64,
}

/// Handle to a teammate's child process (not serializable)
///
/// Supports interactive mode via an explicit `stdin` pipe. When a teammate
/// is spawned in interactive mode (no `-p` flag), messages are sent by writing
/// to `stdin` instead of spawning a new process.
pub struct TeammateHandle {
    pub child: Child,
    pub stream_task: Option<JoinHandle<()>>,
    /// Explicit stdin pipe for interactive mode messaging.
    /// When set, messages can be written directly to the teammate's stdin.
    pub stdin: Option<ChildStdin>,
}

impl TeammateHandle {
    /// Create a new handle from a child process, optionally capturing stdin.
    pub fn new(mut child: Child, interactive: bool) -> Self {
        let stdin = if interactive {
            child.stdin.take()
        } else {
            None
        };
        Self {
            child,
            stream_task: None,
            stdin,
        }
    }

    /// Write a message to the teammate's stdin pipe (interactive mode).
    ///
    /// Returns an error if the teammate has no stdin pipe or the write fails.
    pub async fn write_message(&mut self, message: &str) -> Result<(), std::io::Error> {
        if let Some(ref mut stdin) = self.stdin {
            stdin.write_all(message.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Teammate stdin not available (non-interactive mode)",
            ))
        }
    }

    /// Check if this handle supports interactive messaging
    pub fn is_interactive(&self) -> bool {
        self.stdin.is_some()
    }
}

impl std::fmt::Debug for TeammateHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeammateHandle")
            .field("child_pid", &self.child.id())
            .field("interactive", &self.stdin.is_some())
            .finish()
    }
}

/// State of a single teammate
pub struct TeammateState {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role: String,
    pub status: TeammateStatus,
    pub cost: TeammateCost,
    pub handle: Option<TeammateHandle>,
    pub spawned_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

impl std::fmt::Debug for TeammateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeammateState")
            .field("name", &self.name)
            .field("color", &self.color)
            .field("model", &self.model)
            .field("role", &self.role)
            .field("status", &self.status)
            .field("cost", &self.cost)
            .field("has_handle", &self.handle.is_some())
            .finish()
    }
}

/// A message in the team conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub id: String,
    pub team_name: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub message_type: TeamMessageType,
    pub timestamp: DateTime<Utc>,
}

/// Type of team message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMessageType {
    UserMessage,
    TeammateMessage,
    Broadcast,
    System,
}

/// State of an active team
pub struct TeamState {
    pub name: String,
    pub context_id: String,
    pub context_type: String,
    pub lead_name: Option<String>,
    pub teammates: HashMap<String, TeammateState>,
    pub messages: Vec<TeamMessage>,
    pub created_at: DateTime<Utc>,
    pub phase: TeamPhase,
}

impl std::fmt::Debug for TeamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamState")
            .field("name", &self.name)
            .field("context_id", &self.context_id)
            .field("lead_name", &self.lead_name)
            .field("teammate_count", &self.teammates.len())
            .field("message_count", &self.messages.len())
            .field("phase", &self.phase)
            .finish()
    }
}

/// Phase of the team lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamPhase {
    Forming,
    Active,
    Winding,
    Disbanded,
}

impl std::fmt::Display for TeamPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamPhase::Forming => write!(f, "forming"),
            TeamPhase::Active => write!(f, "active"),
            TeamPhase::Winding => write!(f, "winding"),
            TeamPhase::Disbanded => write!(f, "disbanded"),
        }
    }
}

// ============================================================================
// Serializable response types (for IPC)
// ============================================================================

/// Serializable snapshot of teammate state
#[derive(Debug, Clone, Serialize)]
pub struct TeammateStatusResponse {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role: String,
    pub status: TeammateStatus,
    pub cost: TeammateCost,
    pub spawned_at: String,
    pub last_activity_at: String,
}

/// Serializable snapshot of team state
#[derive(Debug, Clone, Serialize)]
pub struct TeamStatusResponse {
    pub name: String,
    pub context_id: String,
    pub context_type: String,
    pub lead_name: Option<String>,
    pub teammates: Vec<TeammateStatusResponse>,
    pub phase: TeamPhase,
    pub created_at: String,
    pub message_count: usize,
}

/// Serializable team message for IPC
#[derive(Debug, Clone, Serialize)]
pub struct TeamMessageResponse {
    pub id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub message_type: TeamMessageType,
    pub timestamp: String,
}

/// Serializable cost response
#[derive(Debug, Clone, Serialize)]
pub struct TeammateCostResponse {
    pub teammate_name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub estimated_usd: f64,
}

// ============================================================================
// TeamStateTracker
// ============================================================================

/// Thread-safe tracker for active agent teams
#[derive(Debug, Clone)]
pub struct TeamStateTracker {
    teams: Arc<RwLock<HashMap<String, TeamState>>>,
}

impl Default for TeamStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamStateTracker {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new team
    pub async fn create_team(
        &self,
        name: &str,
        context_id: &str,
        context_type: &str,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        if teams.contains_key(name) {
            return Err(TeamTrackerError::TeamAlreadyExists(name.to_string()));
        }
        teams.insert(
            name.to_string(),
            TeamState {
                name: name.to_string(),
                context_id: context_id.to_string(),
                context_type: context_type.to_string(),
                lead_name: None,
                teammates: HashMap::new(),
                messages: Vec::new(),
                created_at: Utc::now(),
                phase: TeamPhase::Forming,
            },
        );
        Ok(())
    }

    /// Add a teammate to a team
    pub async fn add_teammate(
        &self,
        team_name: &str,
        name: &str,
        color: &str,
        model: &str,
        role: &str,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        if team.teammates.contains_key(name) {
            return Err(TeamTrackerError::TeammateAlreadyExists(name.to_string()));
        }

        let now = Utc::now();
        team.teammates.insert(
            name.to_string(),
            TeammateState {
                name: name.to_string(),
                color: color.to_string(),
                model: model.to_string(),
                role: role.to_string(),
                status: TeammateStatus::Spawning,
                cost: TeammateCost::default(),
                handle: None,
                spawned_at: now,
                last_activity_at: now,
            },
        );

        // First teammate added becomes the lead
        if team.lead_name.is_none() {
            team.lead_name = Some(name.to_string());
        }

        // Move to Active phase when first teammate is added
        if team.phase == TeamPhase::Forming {
            team.phase = TeamPhase::Active;
        }

        Ok(())
    }

    /// Set the teammate handle (process + stream task)
    pub async fn set_teammate_handle(
        &self,
        team_name: &str,
        teammate_name: &str,
        handle: TeammateHandle,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get_mut(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;
        teammate.handle = Some(handle);
        Ok(())
    }

    /// Update teammate status
    pub async fn update_teammate_status(
        &self,
        team_name: &str,
        teammate_name: &str,
        status: TeammateStatus,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get_mut(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;
        teammate.status = status;
        teammate.last_activity_at = Utc::now();
        Ok(())
    }

    /// Update teammate cost
    pub async fn update_teammate_cost(
        &self,
        team_name: &str,
        teammate_name: &str,
        cost: TeammateCost,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get_mut(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;
        teammate.cost = cost;
        Ok(())
    }

    /// Get team status (serializable snapshot)
    pub async fn get_team_status(
        &self,
        team_name: &str,
    ) -> Result<TeamStatusResponse, TeamTrackerError> {
        let teams = self.teams.read().await;
        let team = teams
            .get(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        Ok(TeamStatusResponse {
            name: team.name.clone(),
            context_id: team.context_id.clone(),
            context_type: team.context_type.clone(),
            lead_name: team.lead_name.clone(),
            teammates: team
                .teammates
                .values()
                .map(|t| TeammateStatusResponse {
                    name: t.name.clone(),
                    color: t.color.clone(),
                    model: t.model.clone(),
                    role: t.role.clone(),
                    status: t.status,
                    cost: t.cost.clone(),
                    spawned_at: t.spawned_at.to_rfc3339(),
                    last_activity_at: t.last_activity_at.to_rfc3339(),
                })
                .collect(),
            phase: team.phase,
            created_at: team.created_at.to_rfc3339(),
            message_count: team.messages.len(),
        })
    }

    /// Get teammate cost
    pub async fn get_teammate_cost(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<TeammateCostResponse, TeamTrackerError> {
        let teams = self.teams.read().await;
        let team = teams
            .get(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;

        Ok(TeammateCostResponse {
            teammate_name: teammate.name.clone(),
            input_tokens: teammate.cost.input_tokens,
            output_tokens: teammate.cost.output_tokens,
            cache_creation_tokens: teammate.cost.cache_creation_tokens,
            cache_read_tokens: teammate.cost.cache_read_tokens,
            estimated_usd: teammate.cost.estimated_usd,
        })
    }

    /// Send a user message to the team
    pub async fn send_user_message(
        &self,
        team_name: &str,
        content: &str,
    ) -> Result<TeamMessage, TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        let message = TeamMessage {
            id: uuid::Uuid::new_v4().to_string(),
            team_name: team_name.to_string(),
            sender: "user".to_string(),
            recipient: None,
            content: content.to_string(),
            message_type: TeamMessageType::UserMessage,
            timestamp: Utc::now(),
        };
        team.messages.push(message.clone());
        Ok(message)
    }

    /// Add a teammate message
    pub async fn add_teammate_message(
        &self,
        team_name: &str,
        sender: &str,
        recipient: Option<&str>,
        content: &str,
        message_type: TeamMessageType,
    ) -> Result<TeamMessage, TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        let message = TeamMessage {
            id: uuid::Uuid::new_v4().to_string(),
            team_name: team_name.to_string(),
            sender: sender.to_string(),
            recipient: recipient.map(|r| r.to_string()),
            content: content.to_string(),
            message_type,
            timestamp: Utc::now(),
        };
        team.messages.push(message.clone());
        Ok(message)
    }

    /// Get team messages (serializable)
    pub async fn get_team_messages(
        &self,
        team_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessageResponse>, TeamTrackerError> {
        let teams = self.teams.read().await;
        let team = teams
            .get(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        let messages: Vec<TeamMessageResponse> = team
            .messages
            .iter()
            .rev()
            .take(limit.unwrap_or(usize::MAX))
            .map(|m| TeamMessageResponse {
                id: m.id.clone(),
                sender: m.sender.clone(),
                recipient: m.recipient.clone(),
                content: m.content.clone(),
                message_type: m.message_type.clone(),
                timestamp: m.timestamp.to_rfc3339(),
            })
            .collect();

        Ok(messages)
    }

    /// Stop a specific teammate
    pub async fn stop_teammate(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get_mut(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;

        // Kill the child process if running
        if let Some(ref mut handle) = teammate.handle {
            let _ = handle.child.kill().await;
            if let Some(task) = handle.stream_task.take() {
                task.abort();
            }
        }
        teammate.status = TeammateStatus::Shutdown;
        teammate.last_activity_at = Utc::now();
        teammate.handle = None;
        Ok(())
    }

    /// Remove a teammate from a team
    pub async fn remove_teammate(
        &self,
        team_name: &str,
        teammate_name: &str,
    ) -> Result<(), TeamTrackerError> {
        // Stop first
        self.stop_teammate(team_name, teammate_name).await?;

        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        team.teammates.remove(teammate_name);
        Ok(())
    }

    /// Stop all teammates in a team
    pub async fn stop_team(&self, team_name: &str) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;

        for teammate in team.teammates.values_mut() {
            if let Some(ref mut handle) = teammate.handle {
                let _ = handle.child.kill().await;
                if let Some(task) = handle.stream_task.take() {
                    task.abort();
                }
            }
            teammate.status = TeammateStatus::Shutdown;
            teammate.last_activity_at = Utc::now();
            teammate.handle = None;
        }
        team.phase = TeamPhase::Winding;
        Ok(())
    }

    /// Disband a team (stop all + remove)
    pub async fn disband_team(&self, team_name: &str) -> Result<(), TeamTrackerError> {
        // Stop all teammates first (needs separate write lock acquisition)
        self.stop_team(team_name).await?;

        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        team.phase = TeamPhase::Disbanded;
        Ok(())
    }

    /// List all active team names
    pub async fn list_teams(&self) -> Vec<String> {
        let teams = self.teams.read().await;
        teams.keys().cloned().collect()
    }

    /// Check if a team exists
    pub async fn team_exists(&self, team_name: &str) -> bool {
        let teams = self.teams.read().await;
        teams.contains_key(team_name)
    }

    /// Send a message to a teammate's stdin (interactive mode).
    ///
    /// Returns an error if the teammate doesn't exist, has no handle,
    /// or is not in interactive mode.
    pub async fn send_stdin_message(
        &self,
        team_name: &str,
        teammate_name: &str,
        message: &str,
    ) -> Result<(), TeamTrackerError> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        let teammate = team
            .teammates
            .get_mut(teammate_name)
            .ok_or_else(|| TeamTrackerError::TeammateNotFound(teammate_name.to_string()))?;

        if let Some(ref mut handle) = teammate.handle {
            handle
                .write_message(message)
                .await
                .map_err(|e| TeamTrackerError::StdinWriteFailed(e.to_string()))?;
            teammate.last_activity_at = Utc::now();
            Ok(())
        } else {
            Err(TeamTrackerError::TeammateNotFound(format!(
                "{} (no process handle)",
                teammate_name
            )))
        }
    }

    /// Get the teammate count for a team
    pub async fn get_teammate_count(&self, team_name: &str) -> Result<usize, TeamTrackerError> {
        let teams = self.teams.read().await;
        let team = teams
            .get(team_name)
            .ok_or_else(|| TeamTrackerError::TeamNotFound(team_name.to_string()))?;
        Ok(team.teammates.len())
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone)]
pub enum TeamTrackerError {
    TeamNotFound(String),
    TeamAlreadyExists(String),
    TeammateNotFound(String),
    TeammateAlreadyExists(String),
    StdinWriteFailed(String),
    MaxTeammatesExceeded { max: u8, current: usize },
}

impl std::fmt::Display for TeamTrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TeamNotFound(name) => write!(f, "Team not found: {}", name),
            Self::TeamAlreadyExists(name) => write!(f, "Team already exists: {}", name),
            Self::TeammateNotFound(name) => write!(f, "Teammate not found: {}", name),
            Self::TeammateAlreadyExists(name) => write!(f, "Teammate already exists: {}", name),
            Self::StdinWriteFailed(msg) => write!(f, "Stdin write failed: {}", msg),
            Self::MaxTeammatesExceeded { max, current } => {
                write!(f, "Max teammates exceeded: {current} >= {max}")
            }
        }
    }
}

impl std::error::Error for TeamTrackerError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_team() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("ideation-team", "session-123", "ideation")
            .await
            .unwrap();

        assert!(tracker.team_exists("ideation-team").await);
        assert!(!tracker.team_exists("nonexistent").await);
    }

    #[tokio::test]
    async fn test_create_duplicate_team_fails() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        let result = tracker.create_team("team1", "ctx-2", "ideation").await;
        assert!(matches!(result, Err(TeamTrackerError::TeamAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_add_teammate() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        tracker
            .add_teammate("team1", "researcher", "#ff6b35", "opus", "explore")
            .await
            .unwrap();

        let status = tracker.get_team_status("team1").await.unwrap();
        assert_eq!(status.teammates.len(), 1);
        assert_eq!(status.teammates[0].name, "researcher");
        assert_eq!(status.teammates[0].status, TeammateStatus::Spawning);
        assert_eq!(status.lead_name, Some("researcher".to_string()));
        assert_eq!(status.phase, TeamPhase::Active);
    }

    #[tokio::test]
    async fn test_add_duplicate_teammate_fails() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "researcher", "#ff6b35", "opus", "explore")
            .await
            .unwrap();

        let result = tracker
            .add_teammate("team1", "researcher", "#00ff00", "sonnet", "plan")
            .await;
        assert!(matches!(
            result,
            Err(TeamTrackerError::TeammateAlreadyExists(_))
        ));
    }

    #[tokio::test]
    async fn test_update_teammate_status() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "worker", "#00ff00", "sonnet", "code")
            .await
            .unwrap();

        tracker
            .update_teammate_status("team1", "worker", TeammateStatus::Running)
            .await
            .unwrap();

        let status = tracker.get_team_status("team1").await.unwrap();
        assert_eq!(status.teammates[0].status, TeammateStatus::Running);
    }

    #[tokio::test]
    async fn test_update_status_nonexistent_team_fails() {
        let tracker = TeamStateTracker::new();
        let result = tracker
            .update_teammate_status("nonexistent", "worker", TeammateStatus::Running)
            .await;
        assert!(matches!(result, Err(TeamTrackerError::TeamNotFound(_))));
    }

    #[tokio::test]
    async fn test_update_status_nonexistent_teammate_fails() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        let result = tracker
            .update_teammate_status("team1", "ghost", TeammateStatus::Running)
            .await;
        assert!(matches!(result, Err(TeamTrackerError::TeammateNotFound(_))));
    }

    #[tokio::test]
    async fn test_update_teammate_cost() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "researcher", "#ff6b35", "opus", "explore")
            .await
            .unwrap();

        let cost = TeammateCost {
            input_tokens: 1000,
            output_tokens: 500,
            cache_creation_tokens: 200,
            cache_read_tokens: 100,
            estimated_usd: 0.05,
        };
        tracker
            .update_teammate_cost("team1", "researcher", cost)
            .await
            .unwrap();

        let cost_response = tracker
            .get_teammate_cost("team1", "researcher")
            .await
            .unwrap();
        assert_eq!(cost_response.input_tokens, 1000);
        assert_eq!(cost_response.output_tokens, 500);
        assert_eq!(cost_response.estimated_usd, 0.05);
    }

    #[tokio::test]
    async fn test_send_user_message() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        let msg = tracker
            .send_user_message("team1", "Hello team!")
            .await
            .unwrap();
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.content, "Hello team!");
        assert_eq!(msg.message_type, TeamMessageType::UserMessage);
    }

    #[tokio::test]
    async fn test_add_teammate_message() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        let msg = tracker
            .add_teammate_message(
                "team1",
                "researcher",
                Some("planner"),
                "Found some results",
                TeamMessageType::TeammateMessage,
            )
            .await
            .unwrap();
        assert_eq!(msg.sender, "researcher");
        assert_eq!(msg.recipient, Some("planner".to_string()));
    }

    #[tokio::test]
    async fn test_get_team_messages_with_limit() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();

        for i in 0..5 {
            tracker
                .send_user_message("team1", &format!("Message {}", i))
                .await
                .unwrap();
        }

        // Get all messages
        let all = tracker.get_team_messages("team1", None).await.unwrap();
        assert_eq!(all.len(), 5);

        // Get limited messages (most recent first)
        let limited = tracker
            .get_team_messages("team1", Some(2))
            .await
            .unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[tokio::test]
    async fn test_stop_team() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "worker1", "#ff0000", "sonnet", "code")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "worker2", "#00ff00", "sonnet", "code")
            .await
            .unwrap();

        tracker.stop_team("team1").await.unwrap();

        let status = tracker.get_team_status("team1").await.unwrap();
        assert_eq!(status.phase, TeamPhase::Winding);
        for t in &status.teammates {
            assert_eq!(t.status, TeammateStatus::Shutdown);
        }
    }

    #[tokio::test]
    async fn test_disband_team() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "worker", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        tracker.disband_team("team1").await.unwrap();

        let status = tracker.get_team_status("team1").await.unwrap();
        assert_eq!(status.phase, TeamPhase::Disbanded);
    }

    #[tokio::test]
    async fn test_list_teams() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("alpha", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .create_team("beta", "ctx-2", "ideation")
            .await
            .unwrap();

        let teams = tracker.list_teams().await;
        assert_eq!(teams.len(), 2);
        assert!(teams.contains(&"alpha".to_string()));
        assert!(teams.contains(&"beta".to_string()));
    }

    #[tokio::test]
    async fn test_remove_teammate() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("team1", "ctx-1", "ideation")
            .await
            .unwrap();
        tracker
            .add_teammate("team1", "worker", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        tracker.remove_teammate("team1", "worker").await.unwrap();

        let status = tracker.get_team_status("team1").await.unwrap();
        assert_eq!(status.teammates.len(), 0);
    }

    #[tokio::test]
    async fn test_thread_safety() {
        let tracker = TeamStateTracker::new();
        tracker
            .create_team("shared", "ctx-1", "ideation")
            .await
            .unwrap();

        let mut handles = vec![];
        for i in 0..10 {
            let t = tracker.clone();
            handles.push(tokio::spawn(async move {
                t.add_teammate(
                    "shared",
                    &format!("worker-{}", i),
                    "#ffffff",
                    "sonnet",
                    "code",
                )
                .await
                .unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let status = tracker.get_team_status("shared").await.unwrap();
        assert_eq!(status.teammates.len(), 10);
    }

    #[tokio::test]
    async fn test_default_creates_new_tracker() {
        let tracker = TeamStateTracker::default();
        let teams = tracker.list_teams().await;
        assert!(teams.is_empty());
    }
}
