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

    #[test]
    fn new_does_not_require_tokio_runtime() {
        // Spawn a dedicated OS thread — guaranteed no Tokio runtime context.
        // Construct EVERYTHING on this thread to avoid Send bound issues.
        let result = std::thread::spawn(|| {
            // create_mock_app() uses mock_builder().build(mock_context(noop_assets()))
            // — all synchronous, no Tokio deps.
            let app = crate::testing::create_mock_app();
            let handle = app.handle().clone();

            // If someone reintroduces tokio::spawn in the constructor,
            // this will panic: "there is no reactor running"
            let _emitter = crate::application::ThrottledEmitter::new(handle);
            // Drop emitter before app to avoid stale handle access from the background flush thread.
            drop(_emitter);
        })
        .join();

        assert!(
            result.is_ok(),
            "ThrottledEmitter::new() panicked — likely uses tokio::spawn instead of std::thread::spawn. See .claude/rules/tokio-runtime-safety.md"
        );
    }
}
