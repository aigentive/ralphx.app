// SupervisorService
// Application service for supervisor monitoring and intervention

use crate::domain::supervisor::{
    action_for_detection, DetectionResult, Pattern, ProgressInfo, Severity, SupervisorAction,
    SupervisorEvent, ToolCallInfo, ToolCallWindow,
};
use crate::infrastructure::supervisor::EventBus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for supervisor thresholds
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Token usage threshold (default: 50,000)
    pub token_threshold: u32,
    /// Maximum tokens before forcing stop (default: 100,000)
    pub max_tokens: u32,
    /// Time threshold in seconds before warning (default: 600 = 10 min)
    pub time_threshold_seconds: u64,
    /// Progress check interval in seconds (default: 30)
    pub progress_interval_seconds: u64,
    /// Minimum loop occurrences before action (default: 3)
    pub loop_threshold: usize,
    /// Minimum stuck checks before action (default: 5)
    pub stuck_threshold: usize,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            token_threshold: 50_000,
            max_tokens: 100_000,
            time_threshold_seconds: 600,
            progress_interval_seconds: 30,
            loop_threshold: 3,
            stuck_threshold: 5,
        }
    }
}

/// State for a single task being monitored
#[derive(Debug, Clone)]
pub struct TaskMonitorState {
    /// Task ID
    pub task_id: String,
    /// Task description
    pub description: String,
    /// Rolling window of recent tool calls
    pub tool_window: ToolCallWindow,
    /// Count of stuck checks (no progress)
    pub stuck_count: usize,
    /// Error count by error message
    pub error_counts: HashMap<String, usize>,
    /// Last progress info
    pub last_progress: Option<ProgressInfo>,
    /// Actions taken on this task
    pub actions_taken: Vec<SupervisorAction>,
    /// Whether the task is paused
    pub is_paused: bool,
    /// Whether the task is killed
    pub is_killed: bool,
}

impl TaskMonitorState {
    pub fn new(task_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            description: description.into(),
            tool_window: ToolCallWindow::default(),
            stuck_count: 0,
            error_counts: HashMap::new(),
            last_progress: None,
            actions_taken: Vec::new(),
            is_paused: false,
            is_killed: false,
        }
    }

    pub fn record_tool_call(&mut self, info: ToolCallInfo) {
        self.tool_window.push(info);
    }

    pub fn record_error(&mut self, message: &str) {
        *self.error_counts.entry(message.to_string()).or_insert(0) += 1;
    }

    pub fn record_progress(&mut self, info: ProgressInfo, had_progress: bool) {
        if !had_progress {
            self.stuck_count += 1;
        } else {
            self.stuck_count = 0;
        }
        self.last_progress = Some(info);
    }

    pub fn record_action(&mut self, action: SupervisorAction) {
        if matches!(action, SupervisorAction::Pause { .. }) {
            self.is_paused = true;
        }
        if matches!(action, SupervisorAction::Kill { .. }) {
            self.is_killed = true;
        }
        self.actions_taken.push(action);
    }
}

/// Supervisor service for monitoring agent execution
pub struct SupervisorService {
    /// Event bus for receiving/publishing events
    event_bus: EventBus,
    /// Configuration
    config: SupervisorConfig,
    /// State for each monitored task
    task_states: Arc<RwLock<HashMap<String, TaskMonitorState>>>,
    /// Callback for executing actions (optional)
    action_handler: Option<ActionHandler>,
}

/// Type alias for supervisor action handler callback
type ActionHandler = Arc<dyn Fn(SupervisorAction, &str) + Send + Sync>;

impl SupervisorService {
    /// Create a new supervisor service with default config
    pub fn new(event_bus: EventBus) -> Self {
        Self::with_config(event_bus, SupervisorConfig::default())
    }

    /// Create a new supervisor service with custom config
    pub fn with_config(event_bus: EventBus, config: SupervisorConfig) -> Self {
        Self {
            event_bus,
            config,
            task_states: Arc::new(RwLock::new(HashMap::new())),
            action_handler: None,
        }
    }

    /// Set an action handler callback
    pub fn set_action_handler<F>(&mut self, handler: F)
    where
        F: Fn(SupervisorAction, &str) + Send + Sync + 'static,
    {
        self.action_handler = Some(Arc::new(handler));
    }

    /// Get the event bus for this service
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Process a supervisor event and determine if action is needed
    pub async fn process_event(&self, event: SupervisorEvent) -> Option<SupervisorAction> {
        match event {
            SupervisorEvent::TaskStart {
                task_id,
                agent_role,
                ..
            } => {
                self.start_monitoring(task_id.clone(), agent_role).await;
                None
            }
            SupervisorEvent::ToolCall { task_id, info, .. } => {
                self.handle_tool_call(&task_id, info).await
            }
            SupervisorEvent::Error { task_id, info, .. } => {
                self.handle_error(&task_id, &info.message).await
            }
            SupervisorEvent::ProgressTick { task_id, info, .. } => {
                self.handle_progress(&task_id, info).await
            }
            SupervisorEvent::TokenThreshold {
                task_id,
                tokens_used,
                threshold,
                ..
            } => self.handle_token_threshold(&task_id, tokens_used, threshold).await,
            SupervisorEvent::TimeThreshold {
                task_id,
                elapsed_minutes,
                threshold_minutes,
                ..
            } => {
                // Convert minutes to seconds for internal handling
                self.handle_time_threshold(&task_id, elapsed_minutes as u64 * 60, threshold_minutes as u64 * 60)
                    .await
            }
        }
    }

    /// Start monitoring a task
    pub async fn start_monitoring(&self, task_id: String, description: String) {
        let mut states = self.task_states.write().await;
        states.insert(task_id.clone(), TaskMonitorState::new(task_id, description));
    }

    /// Stop monitoring a task
    pub async fn stop_monitoring(&self, task_id: &str) {
        let mut states = self.task_states.write().await;
        states.remove(task_id);
    }

    /// Get the current state for a task
    pub async fn get_task_state(&self, task_id: &str) -> Option<TaskMonitorState> {
        let states = self.task_states.read().await;
        states.get(task_id).cloned()
    }

    /// Check if a task is paused
    pub async fn is_task_paused(&self, task_id: &str) -> bool {
        let states = self.task_states.read().await;
        states.get(task_id).map(|s| s.is_paused).unwrap_or(false)
    }

    /// Check if a task is killed
    pub async fn is_task_killed(&self, task_id: &str) -> bool {
        let states = self.task_states.read().await;
        states.get(task_id).map(|s| s.is_killed).unwrap_or(false)
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: &str) -> bool {
        let mut states = self.task_states.write().await;
        if let Some(state) = states.get_mut(task_id) {
            if state.is_paused && !state.is_killed {
                state.is_paused = false;
                return true;
            }
        }
        false
    }

    async fn handle_tool_call(&self, task_id: &str, info: ToolCallInfo) -> Option<SupervisorAction> {
        let mut states = self.task_states.write().await;
        let state = states.get_mut(task_id)?;

        if state.is_killed {
            return None;
        }

        state.record_tool_call(info);

        // Check for loop patterns
        if let Some(detection) = state.tool_window.detect_loop() {
            let action = action_for_detection(&detection);
            if action.is_intervention() {
                state.record_action(action.clone());
                self.execute_action(&action, task_id);
                return Some(action);
            }
        }

        None
    }

    async fn handle_error(&self, task_id: &str, message: &str) -> Option<SupervisorAction> {
        let mut states = self.task_states.write().await;
        let state = states.get_mut(task_id)?;

        if state.is_killed {
            return None;
        }

        state.record_error(message);

        // Check for repeating errors
        let error_count = *state.error_counts.get(message).unwrap_or(&0);

        if error_count >= 4 {
            let detection = DetectionResult::new(
                Pattern::RepeatingError,
                90,
                format!("Error '{}' occurred {} times", message, error_count),
                error_count,
            );
            let action = action_for_detection(&detection);
            state.record_action(action.clone());
            self.execute_action(&action, task_id);
            return Some(action);
        } else if error_count >= 3 {
            let detection = DetectionResult::new(
                Pattern::RepeatingError,
                75,
                format!("Error '{}' occurred {} times", message, error_count),
                error_count,
            );
            let action = action_for_detection(&detection);
            if action.is_intervention() {
                state.record_action(action.clone());
                self.execute_action(&action, task_id);
                return Some(action);
            }
        }

        None
    }

    async fn handle_progress(&self, task_id: &str, info: ProgressInfo) -> Option<SupervisorAction> {
        let mut states = self.task_states.write().await;
        let state = states.get_mut(task_id)?;

        if state.is_killed {
            return None;
        }

        // Determine if there was progress
        let had_progress = info.has_file_changes || info.has_new_commits;
        state.record_progress(info, had_progress);

        // Check for stuck pattern
        if state.stuck_count >= self.config.stuck_threshold {
            let detection = DetectionResult::new(
                Pattern::Stuck,
                80 + state.stuck_count.min(10) as u8,
                format!("No progress for {} checks", state.stuck_count),
                state.stuck_count,
            );
            let action = action_for_detection(&detection);
            if action.is_intervention() {
                state.record_action(action.clone());
                self.execute_action(&action, task_id);
                return Some(action);
            }
        }

        None
    }

    async fn handle_token_threshold(
        &self,
        task_id: &str,
        current: u32,
        threshold: u32,
    ) -> Option<SupervisorAction> {
        let mut states = self.task_states.write().await;
        let state = states.get_mut(task_id)?;

        if state.is_killed {
            return None;
        }

        // High token usage could indicate a runaway or very complex task
        let severity = if current >= self.config.max_tokens {
            Severity::Critical
        } else if current >= threshold {
            Severity::High
        } else {
            Severity::Medium
        };

        let action = match severity {
            Severity::Critical => SupervisorAction::kill(
                "Token limit exceeded",
                format!(
                    "Task used {} tokens (max: {}). Stopping to prevent runaway.",
                    current, self.config.max_tokens
                ),
            ),
            Severity::High => SupervisorAction::pause(format!(
                "Token usage ({}) exceeds threshold ({}). Review before continuing.",
                current, threshold
            )),
            _ => SupervisorAction::log(
                Severity::Medium,
                format!("Token usage: {} / {}", current, threshold),
            ),
        };

        if action.is_intervention() {
            state.record_action(action.clone());
            self.execute_action(&action, task_id);
        }

        Some(action)
    }

    async fn handle_time_threshold(
        &self,
        task_id: &str,
        elapsed_seconds: u64,
        threshold_seconds: u64,
    ) -> Option<SupervisorAction> {
        let mut states = self.task_states.write().await;
        let state = states.get_mut(task_id)?;

        if state.is_killed {
            return None;
        }

        // Check if task is taking too long
        let severity = if elapsed_seconds >= threshold_seconds * 3 {
            Severity::Critical
        } else if elapsed_seconds >= threshold_seconds * 2 {
            Severity::High
        } else {
            Severity::Medium
        };

        let action = match severity {
            Severity::Critical => SupervisorAction::kill(
                "Time limit exceeded",
                format!(
                    "Task running for {} seconds (limit: {}). May be stuck or too complex.",
                    elapsed_seconds,
                    threshold_seconds * 3
                ),
            ),
            Severity::High => SupervisorAction::pause(format!(
                "Task running for {} seconds. Please verify progress before continuing.",
                elapsed_seconds
            )),
            _ => SupervisorAction::inject_guidance(
                "Task is taking longer than expected. Consider breaking into smaller subtasks.",
            ),
        };

        if action.is_intervention() {
            state.record_action(action.clone());
            self.execute_action(&action, task_id);
        }

        Some(action)
    }

    fn execute_action(&self, action: &SupervisorAction, task_id: &str) {
        if let Some(handler) = &self.action_handler {
            handler(action.clone(), task_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::supervisor::ErrorInfo;

    fn create_test_service() -> SupervisorService {
        let event_bus = EventBus::new();
        SupervisorService::with_config(
            event_bus,
            SupervisorConfig {
                loop_threshold: 3,
                stuck_threshold: 3,
                ..Default::default()
            },
        )
    }

    #[tokio::test]
    async fn test_supervisor_service_new() {
        let event_bus = EventBus::new();
        let service = SupervisorService::new(event_bus);
        assert_eq!(service.config.token_threshold, 50_000);
    }

    #[tokio::test]
    async fn test_start_monitoring() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        let state = service.get_task_state("task-1").await;
        assert!(state.is_some());
        assert_eq!(state.unwrap().task_id, "task-1");
    }

    #[tokio::test]
    async fn test_stop_monitoring() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;
        service.stop_monitoring("task-1").await;

        let state = service.get_task_state("task-1").await;
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn test_process_task_start_event() {
        let service = create_test_service();
        let event = SupervisorEvent::task_start("task-1", "Test task");

        let action = service.process_event(event).await;
        assert!(action.is_none());

        let state = service.get_task_state("task-1").await;
        assert!(state.is_some());
    }

    #[tokio::test]
    async fn test_process_tool_call_no_loop() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Different tool calls shouldn't trigger loop detection
        let event1 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file1.rs"));
        let event2 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Write", "file2.rs"));
        let event3 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Edit", "file3.rs"));

        assert!(service.process_event(event1).await.is_none());
        assert!(service.process_event(event2).await.is_none());
        assert!(service.process_event(event3).await.is_none());
    }

    #[tokio::test]
    async fn test_process_tool_call_loop_detected() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Same tool call repeatedly should trigger loop detection
        for i in 0..5 {
            let event = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file.rs"));
            let action = service.process_event(event).await;

            if i >= 3 {
                // After 3+ identical calls, should detect loop
                assert!(action.is_some(), "Expected action after {} calls", i + 1);
            }
        }

        let state = service.get_task_state("task-1").await.unwrap();
        assert!(!state.actions_taken.is_empty());
    }

    #[tokio::test]
    async fn test_process_error_repeating() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        let error_info = ErrorInfo::new("Type error in foo.rs", "compile");

        for i in 0..4 {
            let event = SupervisorEvent::error("task-1", error_info.clone());
            let action = service.process_event(event).await;

            if i >= 2 {
                // After 3+ errors, should take action
                assert!(action.is_some(), "Expected action after {} errors", i + 1);
            }
        }
    }

    #[tokio::test]
    async fn test_process_progress_stuck() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // No progress for multiple checks
        for i in 0..5 {
            let info = ProgressInfo::new(); // has_file_changes and has_new_commits are false
            let event = SupervisorEvent::progress_tick("task-1", info);
            let action = service.process_event(event).await;

            if i >= 4 {
                // After 5+ stuck checks, should take action
                assert!(action.is_some(), "Expected action after {} stuck checks", i + 1);
            }
        }
    }

    #[tokio::test]
    async fn test_process_progress_not_stuck() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Progress with file changes
        for _ in 0..5 {
            let mut info = ProgressInfo::new();
            info.has_file_changes = true;
            let event = SupervisorEvent::progress_tick("task-1", info);
            let action = service.process_event(event).await;
            assert!(action.is_none());
        }

        let state = service.get_task_state("task-1").await.unwrap();
        assert_eq!(state.stuck_count, 0);
    }

    #[tokio::test]
    async fn test_process_token_threshold() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        let event = SupervisorEvent::token_threshold("task-1", 60_000, 50_000);
        let action = service.process_event(event).await;

        assert!(action.is_some());
        assert!(matches!(action.unwrap(), SupervisorAction::Pause { .. }));
    }

    #[tokio::test]
    async fn test_process_token_threshold_critical() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
        let action = service.process_event(event).await;

        assert!(action.is_some());
        assert!(matches!(action.unwrap(), SupervisorAction::Kill { .. }));

        assert!(service.is_task_killed("task-1").await);
    }

    #[tokio::test]
    async fn test_process_time_threshold() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // 12 minutes elapsed, 10 minute threshold (just over, should inject guidance)
        let event = SupervisorEvent::time_threshold("task-1", 12, 10);
        let action = service.process_event(event).await;

        assert!(action.is_some());
        assert!(matches!(
            action.unwrap(),
            SupervisorAction::InjectGuidance { .. }
        ));
    }

    #[tokio::test]
    async fn test_process_time_threshold_pause() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // 20 minutes elapsed, 10 minute threshold (2x = pause)
        let event = SupervisorEvent::time_threshold("task-1", 20, 10);
        let action = service.process_event(event).await;

        assert!(action.is_some());
        assert!(matches!(action.unwrap(), SupervisorAction::Pause { .. }));
        assert!(service.is_task_paused("task-1").await);
    }

    #[tokio::test]
    async fn test_resume_paused_task() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Trigger pause (20 min elapsed, 10 min threshold)
        let event = SupervisorEvent::time_threshold("task-1", 20, 10);
        service.process_event(event).await;

        assert!(service.is_task_paused("task-1").await);

        // Resume
        let resumed = service.resume_task("task-1").await;
        assert!(resumed);
        assert!(!service.is_task_paused("task-1").await);
    }

    #[tokio::test]
    async fn test_cannot_resume_killed_task() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Kill the task
        let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
        service.process_event(event).await;

        assert!(service.is_task_killed("task-1").await);

        // Try to resume
        let resumed = service.resume_task("task-1").await;
        assert!(!resumed);
    }

    #[tokio::test]
    async fn test_killed_task_ignores_events() {
        let service = create_test_service();
        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Kill the task
        let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
        service.process_event(event).await;

        // Subsequent events should be ignored
        let event = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file.rs"));
        let action = service.process_event(event).await;
        assert!(action.is_none());
    }

    #[tokio::test]
    async fn test_action_handler_called() {
        let event_bus = EventBus::new();
        let mut service = SupervisorService::with_config(
            event_bus,
            SupervisorConfig {
                loop_threshold: 3,
                ..Default::default()
            },
        );

        let action_taken = Arc::new(RwLock::new(false));
        let action_taken_clone = action_taken.clone();

        service.set_action_handler(move |_action, _task_id| {
            let action_taken = action_taken_clone.clone();
            tokio::spawn(async move {
                *action_taken.write().await = true;
            });
        });

        service
            .start_monitoring("task-1".to_string(), "Test task".to_string())
            .await;

        // Trigger an action (20 min elapsed, 10 min threshold)
        let event = SupervisorEvent::time_threshold("task-1", 20, 10);
        service.process_event(event).await;

        // Give the handler time to run
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(*action_taken.read().await);
    }

    #[tokio::test]
    async fn test_task_monitor_state() {
        let mut state = TaskMonitorState::new("task-1", "Test task");

        assert_eq!(state.task_id, "task-1");
        assert!(!state.is_paused);
        assert!(!state.is_killed);
        assert_eq!(state.stuck_count, 0);

        state.record_tool_call(ToolCallInfo::new("Read", "file.rs"));
        assert_eq!(state.tool_window.len(), 1);

        state.record_error("Test error");
        assert_eq!(*state.error_counts.get("Test error").unwrap(), 1);

        state.record_progress(ProgressInfo::new(), false);
        assert_eq!(state.stuck_count, 1);

        state.record_progress(ProgressInfo::new(), true);
        assert_eq!(state.stuck_count, 0);
    }

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();
        assert_eq!(config.token_threshold, 50_000);
        assert_eq!(config.max_tokens, 100_000);
        assert_eq!(config.time_threshold_seconds, 600);
        assert_eq!(config.progress_interval_seconds, 30);
        assert_eq!(config.loop_threshold, 3);
        assert_eq!(config.stuck_threshold, 5);
    }
}
