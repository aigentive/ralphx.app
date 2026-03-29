use super::*;
use crate::domain::execution::context_matches_running_status;

/// A single running process with enriched data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    /// Task ID
    pub task_id: String,
    /// Task title
    pub title: String,
    /// Current internal status
    pub internal_status: String,
    /// Step progress summary (if steps exist)
    pub step_progress: Option<StepProgressSummary>,
    /// Elapsed time in seconds since entering current status
    pub elapsed_seconds: Option<i64>,
    /// Trigger origin (scheduler, revision, recovery, retry, qa)
    pub trigger_origin: Option<String>,
    /// Task branch name
    pub task_branch: Option<String>,
}

/// A running ideation session with enriched data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningIdeationSession {
    /// Session ID
    pub session_id: String,
    /// Session title
    pub title: String,
    /// Elapsed time in seconds since session was created
    pub elapsed_seconds: Option<i64>,
    /// Team mode (solo, research, debate)
    pub team_mode: Option<String>,
    /// Whether the agent is actively generating (false = idle between turns)
    pub is_generating: bool,
}

/// Response for get_running_processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcessesResponse {
    /// List of running processes
    pub processes: Vec<RunningProcess>,
    /// List of running ideation sessions
    pub ideation_sessions: Vec<RunningIdeationSession>,
}


#[doc(hidden)]
pub fn context_matches_running_status_for_gc(
    context_type: ChatContextType,
    status: InternalStatus,
) -> bool {
    context_matches_running_status(context_type, status)
}

pub(super) async fn prune_stale_execution_registry_entries(
    app_state: &AppState,
    execution_state: &ExecutionState,
) {
    let entries = app_state.running_agent_registry.list_all().await;
    if entries.is_empty() {
        return;
    }

    let engine = crate::application::PruneEngine::new(
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.task_repo),
        Some(Arc::clone(&app_state.interactive_process_registry)),
    );

    let mut pruned_any = false;

    for (key, info) in &entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(ct) => ct,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        // Age guard: pid=0 entries younger than 30s are in the try_register →
        // update_agent_process window. The pruner must not race against the spawn.
        if info.pid == 0 {
            let age = chrono::Utc::now() - info.started_at;
            if age < chrono::Duration::seconds(30) {
                continue;
            }
        }

        // Compute pid liveness once; both the IPR check and staleness evaluation use it.
        let pid_alive = crate::domain::services::is_process_alive(info.pid);

        // PID-verified IPR check: skip if interactive process is alive; remove stale
        // IPR entries (PID dead) so reconciliation is not blocked forever.
        if engine.check_ipr_skip(key, pid_alive).await {
            continue;
        }

        if engine.evaluate_and_prune(key, info, pid_alive).await {
            // Clean up any interactive idle slot tracking for this pruned entry
            // so ghost entries don't persist in interactive_idle_slots.
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            execution_state.remove_interactive_slot(&slot_key);
            pruned_any = true;
        }
    }

    // Correct the running count if entries were pruned.  The GC runs every ~5s so
    // this keeps the slot counter accurate between 30s reconciliation cycles (Bug 3).
    if pruned_any {
        let remaining = app_state.running_agent_registry.list_all().await;
        let idle_count = remaining
            .iter()
            .filter(|(k, _)| {
                let slot_key = format!("{}/{}", k.context_type, k.context_id);
                execution_state.is_interactive_idle(&slot_key)
            })
            .count() as u32;
        execution_state.set_running_count((remaining.len() as u32).saturating_sub(idle_count));
    }
}
