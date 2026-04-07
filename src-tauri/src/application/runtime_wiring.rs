use std::sync::Arc;

use tauri::Manager;

use crate::AppState;
use crate::infrastructure::ExternalMcpHandle;

pub fn create_main_window<R: tauri::Runtime, M: tauri::Manager<R>>(app: &M) -> tauri::Result<()> {
    use tauri::{
        LogicalPosition, Position, TitleBarStyle, WebviewUrl, WebviewWindowBuilder,
    };

    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("")
        .inner_size(1200.0, 800.0)
        .decorations(true)
        .hidden_title(true)
        .visible(false);

    #[cfg(target_os = "macos")]
    {
        builder = builder.title_bar_style(TitleBarStyle::Overlay).traffic_light_position(
            Position::Logical(LogicalPosition { x: 20.0, y: 30.0 }),
        );
    }

    let webview_window = builder.build()?;
    let _ = webview_window.show();
    Ok(())
}

pub fn build_http_app_state(
    app_state: &AppState,
    app_handle: tauri::AppHandle,
) -> crate::AppResult<Arc<AppState>> {
    let shared_db_conn = Arc::clone(app_state.db.inner());
    let shared_question_state = Arc::clone(&app_state.question_state);
    let shared_permission_state = Arc::clone(&app_state.permission_state);
    let shared_message_queue = Arc::clone(&app_state.message_queue);
    let shared_interactive_process_registry = Arc::clone(&app_state.interactive_process_registry);
    let shared_github_service = app_state.github_service.clone();
    let shared_pr_poller_registry = Arc::clone(&app_state.pr_poller_registry);
    let mut http_app_state_inner = AppState::new_production_shared(app_handle, shared_db_conn)?;
    http_app_state_inner.question_state = shared_question_state;
    http_app_state_inner.permission_state = shared_permission_state;
    http_app_state_inner.message_queue = shared_message_queue;
    http_app_state_inner.interactive_process_registry = shared_interactive_process_registry;
    http_app_state_inner.github_service = shared_github_service;
    http_app_state_inner.pr_poller_registry = shared_pr_poller_registry;
    // INVARIANT: streaming_state_cache uses Arc internally; clone shares the same cache.
    http_app_state_inner.streaming_state_cache = app_state.streaming_state_cache.clone();
    http_app_state_inner.webhook_publisher = app_state.webhook_publisher.clone();
    http_app_state_inner.session_merge_locks = Arc::clone(&app_state.session_merge_locks);
    Ok(Arc::new(http_app_state_inner))
}

pub fn register_managed_state(
    app: &mut tauri::App<tauri::Wry>,
    app_state: AppState,
    service_team_tracker: crate::application::TeamStateTracker,
) {
    let team_session_repo = Arc::clone(&app_state.team_session_repo);
    let team_message_repo = Arc::clone(&app_state.team_message_repo);

    let throttled_emitter = crate::application::ThrottledEmitter::new(app.handle().clone());
    app.manage(throttled_emitter);
    app.manage(app_state);

    let team_service = Arc::new(crate::application::TeamService::new_with_repos(
        Arc::new(service_team_tracker),
        app.handle().clone(),
        team_session_repo,
        team_message_repo,
    ));
    app.manage(team_service);
    app.manage(ExternalMcpHandle::new());
}
