use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;

use crate::commands;
use crate::domain::services::RunningAgentRegistry;
use crate::infrastructure::sqlite::DbConnection;
use crate::infrastructure::ExternalMcpHandle;
use crate::AppState;

pub fn handle_run_event<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    event: &tauri::RunEvent,
) {
    if !matches!(event, tauri::RunEvent::Exit) {
        return;
    }

    let app_state = app_handle.state::<AppState>();

    // Set shutdown flag before killing agents so stream handlers can skip escalation.
    let exec_state = app_handle.state::<Arc<commands::ExecutionState>>();
    exec_state.is_shutting_down.store(true, Ordering::SeqCst);

    let registry = Arc::clone(&app_state.running_agent_registry);
    let interactive = Arc::clone(&app_state.interactive_process_registry);
    let terminal_service = Arc::clone(&app_state.agent_terminal_service);
    let db = app_state.db.clone();

    shutdown_agent_terminals(terminal_service);
    shutdown_agents(registry, interactive);
    shutdown_external_mcp(app_handle);
    checkpoint_wal(db);
}

fn shutdown_agent_terminals(terminal_service: Arc<crate::application::AgentTerminalService>) {
    tauri::async_runtime::block_on(async {
        terminal_service.close_all().await;
    });
}

fn shutdown_agents(
    registry: Arc<dyn RunningAgentRegistry>,
    interactive: Arc<crate::application::InteractiveProcessRegistry>,
) {
    tauri::async_runtime::block_on(async {
        let _ = tokio::time::timeout(Duration::from_millis(2500), async move {
            let ipr_dump = interactive.dump_state().await;
            tracing::info!(
                count = ipr_dump.len(),
                "[IPR_EXIT_DUMP] IPR entries at shutdown: {:?}",
                ipr_dump
            );
            interactive.clear().await;
            let stopped = registry.stop_all().await;
            crate::infrastructure::agents::claude::kill_all_tracked_processes().await;
            if !stopped.is_empty() {
                tracing::info!(count = stopped.len(), "Killed running agents on app exit");
            }
        })
        .await;
    });
}

fn shutdown_external_mcp<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    if let Some(supervisor) = app_handle.state::<ExternalMcpHandle>().get() {
        let supervisor = supervisor.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(supervisor.shutdown());
        })
        .join()
        .ok();
    }
}

fn checkpoint_wal(db: DbConnection) {
    tauri::async_runtime::block_on(async {
        let checkpoint_result = db
            .run(|conn| {
                conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                    .map_err(|e| {
                        crate::error::AppError::Database(format!("WAL checkpoint failed: {e}"))
                    })
            })
            .await;
        if let Err(e) = checkpoint_result {
            tracing::warn!(error = %e, "WAL checkpoint on exit failed");
        }
    });
}
