#[cfg(test)]
mod tests {
    use crate::application::ThrottledEmitter;

    #[test]
    fn is_batchable_returns_true_for_task_status_changed() {
        assert!(ThrottledEmitter::<tauri::Wry>::is_batchable(
            "task:status_changed"
        ));
    }

    #[test]
    fn is_batchable_returns_true_for_task_created() {
        assert!(ThrottledEmitter::<tauri::Wry>::is_batchable("task:created"));
    }

    #[test]
    fn is_batchable_returns_false_for_other_events() {
        assert!(!ThrottledEmitter::<tauri::Wry>::is_batchable(
            "agent:run_completed"
        ));
        assert!(!ThrottledEmitter::<tauri::Wry>::is_batchable(
            "task:updated"
        ));
        assert!(!ThrottledEmitter::<tauri::Wry>::is_batchable(
            "agent:message_created"
        ));
    }
}
