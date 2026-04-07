use std::sync::Arc;

use tracing::{info, warn};

use crate::AppState;

pub(crate) fn run_startup_cleanup(app_state: &AppState) {
    // Expire stale pending questions/permissions from previous runs.
    // Must happen before the HTTP server starts accepting agent requests.
    {
        let qs = Arc::clone(&app_state.question_state);
        let ps = Arc::clone(&app_state.permission_state);
        tauri::async_runtime::block_on(async move {
            qs.expire_stale_on_startup().await;
            ps.expire_stale_on_startup().await;
        });
    }

    // Periodic sweep for orphaned in-memory pending questions.
    // Cleans up questions from agents that died without resolving them
    // (complement to expire_stale_on_startup which only runs once at boot).
    {
        let qs = Arc::clone(&app_state.question_state);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                qs.sweep_stale(std::time::Duration::from_secs(900)).await;
            }
        });
    }

    // Cleanup stale process state from previous session.
    // All spawned agent team processes are children of the Tauri app, so any
    // restart (including crash) leaves their DB rows in an active state.
    {
        let team_repo = Arc::clone(&app_state.team_session_repo);
        tauri::async_runtime::block_on(async move {
            match team_repo.disband_all_active("app_restart").await {
                Ok(n) => info!(count = n, "Disbanded stale team sessions on startup"),
                Err(e) => warn!(error = %e, "Failed to disband stale team sessions"),
            }
        });
    }

    // All spawned processes are Tauri children — app restart means they are dead.
    {
        let process_repo = Arc::clone(&app_state.process_repo);
        tauri::async_runtime::block_on(async move {
            match process_repo.fail_all_active("app_restart").await {
                Ok(n) => info!(
                    count = n,
                    "Marked stale research processes failed on startup"
                ),
                Err(e) => {
                    warn!(error = %e, "Failed to mark stale research processes failed on startup")
                }
            }
        });
    }
}
