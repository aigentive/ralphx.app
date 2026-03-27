use super::*;

pub(super) async fn schedule_ready_tasks_for_project(
    app_state: &AppState,
    execution_state: Arc<ExecutionState>,
    project_id: Option<ProjectId>,
) {
    let scheduler = Arc::new(
        TaskSchedulerService::new(
            Arc::clone(&execution_state),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_attachment_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&app_state.memory_event_repo),
            app_state.app_handle.clone(),
        )
        .with_execution_settings_repo(Arc::clone(&app_state.execution_settings_repo))
        .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))
        .with_interactive_process_registry(Arc::clone(&app_state.interactive_process_registry)),
    );
    scheduler.set_self_ref(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);
    scheduler.set_active_project(project_id).await;
    scheduler.try_schedule_ready_tasks().await;
}
