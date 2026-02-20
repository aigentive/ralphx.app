use super::*;

#[test]
fn merge_recovery_metadata_new_creates_empty() {
    let meta = MergeRecoveryMetadata::new();
    assert_eq!(meta.version, 1);
    assert!(meta.events.is_empty());
    assert_eq!(meta.last_state, MergeRecoveryState::Succeeded);
}

#[test]
fn merge_recovery_metadata_default_works() {
    let meta = MergeRecoveryMetadata::default();
    assert_eq!(meta.version, 1);
    assert!(meta.events.is_empty());
}

#[test]
fn merge_recovery_metadata_max_events_constant() {
    assert_eq!(MergeRecoveryMetadata::MAX_EVENTS, 50);
}

#[test]
fn merge_recovery_event_new_sets_defaults() {
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Merge deferred",
    );

    assert_eq!(event.kind, MergeRecoveryEventKind::Deferred);
    assert_eq!(event.source, MergeRecoverySource::System);
    assert_eq!(event.reason_code, MergeRecoveryReasonCode::TargetBranchBusy);
    assert_eq!(event.message, "Merge deferred");
    assert!(event.target_branch.is_none());
    assert!(event.source_branch.is_none());
    assert!(event.blocking_task_id.is_none());
    assert!(event.attempt.is_none());
}

#[test]
fn merge_recovery_event_builder_methods() {
    let task_id = TaskId::new();
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::AutoRetryTriggered,
        MergeRecoverySource::Auto,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Auto retry",
    )
    .with_target_branch("main")
    .with_source_branch("task-branch")
    .with_blocking_task(task_id.clone())
    .with_attempt(2);

    assert_eq!(event.target_branch, Some("main".to_string()));
    assert_eq!(event.source_branch, Some("task-branch".to_string()));
    assert_eq!(event.blocking_task_id, Some(task_id));
    assert_eq!(event.attempt, Some(2));
}

#[test]
fn merge_recovery_event_serializes_to_json() {
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Merge deferred",
    )
    .with_target_branch("main")
    .with_attempt(1);

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"kind\":\"deferred\""));
    assert!(json.contains("\"source\":\"system\""));
    assert!(json.contains("\"reason_code\":\"target_branch_busy\""));
    assert!(json.contains("\"target_branch\":\"main\""));
    assert!(json.contains("\"attempt\":1"));
}

#[test]
fn merge_recovery_event_deserializes_from_json() {
    let json = r#"{
        "at": "2026-02-11T10:00:00Z",
        "kind": "deferred",
        "source": "system",
        "reason_code": "target_branch_busy",
        "message": "Merge deferred",
        "target_branch": "main",
        "attempt": 1
    }"#;

    let event: MergeRecoveryEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event.kind, MergeRecoveryEventKind::Deferred);
    assert_eq!(event.source, MergeRecoverySource::System);
    assert_eq!(event.reason_code, MergeRecoveryReasonCode::TargetBranchBusy);
    assert_eq!(event.message, "Merge deferred");
    assert_eq!(event.target_branch, Some("main".to_string()));
    assert_eq!(event.attempt, Some(1));

}

#[test]
fn merge_recovery_event_skips_serializing_none_fields() {
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Merge deferred",
    );

    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.contains("\"target_branch\""));
    assert!(!json.contains("\"source_branch\""));
    assert!(!json.contains("\"blocking_task_id\""));
    assert!(!json.contains("\"attempt\""));
}

#[test]
fn merge_recovery_metadata_serializes_to_json() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.events.push(MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Deferred",
    ));
    meta.last_state = MergeRecoveryState::Deferred;

    let json = serde_json::to_string(&meta).unwrap();
    assert!(json.contains("\"version\":1"));
    assert!(json.contains("\"events\":["));
    assert!(json.contains("\"last_state\":\"deferred\""));
}

#[test]
fn merge_recovery_metadata_deserializes_from_json() {
    let json = r#"{
        "version": 1,
        "events": [
            {
                "at": "2026-02-11T10:00:00Z",
                "kind": "deferred",
                "source": "system",
                "reason_code": "target_branch_busy",
                "message": "Deferred"
            }
        ],
        "last_state": "deferred"
    }"#;

    let meta: MergeRecoveryMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.version, 1);
    assert_eq!(meta.events.len(), 1);
    assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
}

#[test]
fn merge_recovery_metadata_roundtrip() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.events.push(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        )
        .with_target_branch("main")
        .with_attempt(1),
    );
    meta.last_state = MergeRecoveryState::Deferred;

    let json = serde_json::to_string(&meta).unwrap();
    let restored: MergeRecoveryMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(meta, restored);
}

#[test]
fn merge_recovery_event_kind_serialization() {
    let kinds = [
        (MergeRecoveryEventKind::Deferred, "deferred"),
        (
            MergeRecoveryEventKind::AutoRetryTriggered,
            "auto_retry_triggered",
        ),
        (MergeRecoveryEventKind::AttemptStarted, "attempt_started"),
        (MergeRecoveryEventKind::AttemptFailed, "attempt_failed"),
        (
            MergeRecoveryEventKind::AttemptSucceeded,
            "attempt_succeeded",
        ),
        (MergeRecoveryEventKind::ManualRetry, "manual_retry"),
        (MergeRecoveryEventKind::MainMergeDeferred, "main_merge_deferred"),
        (MergeRecoveryEventKind::MainMergeRetry, "main_merge_retry"),
    ];

    for (kind, expected) in &kinds {
        let json = serde_json::to_string(kind).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn merge_recovery_source_serialization() {
    let sources = [
        (MergeRecoverySource::System, "system"),
        (MergeRecoverySource::Auto, "auto"),
        (MergeRecoverySource::User, "user"),
    ];

    for (source, expected) in &sources {
        let json = serde_json::to_string(source).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn merge_recovery_reason_code_serialization() {
    let codes = [
        (
            MergeRecoveryReasonCode::TargetBranchBusy,
            "target_branch_busy",
        ),
        (MergeRecoveryReasonCode::GitError, "git_error"),
        (
            MergeRecoveryReasonCode::ValidationFailed,
            "validation_failed",
        ),
        (MergeRecoveryReasonCode::BranchNotFound, "branch_not_found"),
        (MergeRecoveryReasonCode::AgentsRunning, "agents_running"),
        (MergeRecoveryReasonCode::DeferredTimeout, "deferred_timeout"),
        (
            MergeRecoveryReasonCode::ProviderRateLimited,
            "provider_rate_limited",
        ),
        (MergeRecoveryReasonCode::Unknown, "unknown"),
    ];

    for (code, expected) in &codes {
        let json = serde_json::to_string(code).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn merge_recovery_state_serialization() {
    let states = [
        (MergeRecoveryState::Deferred, "deferred"),
        (MergeRecoveryState::Retrying, "retrying"),
        (MergeRecoveryState::Failed, "failed"),
        (MergeRecoveryState::Succeeded, "succeeded"),
        (MergeRecoveryState::RateLimited, "rate_limited"),
    ];

    for (state, expected) in &states {
        let json = serde_json::to_string(state).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

// ===== Helper Function Tests =====

#[test]
fn append_event_adds_to_log() {
    let mut meta = MergeRecoveryMetadata::new();
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Deferred",
    );

    meta.append_event(event.clone());

    assert_eq!(meta.events.len(), 1);
    assert_eq!(meta.events[0].message, "Deferred");
}

#[test]
fn append_event_with_state_updates_both() {
    let mut meta = MergeRecoveryMetadata::new();
    let event = MergeRecoveryEvent::new(
        MergeRecoveryEventKind::Deferred,
        MergeRecoverySource::System,
        MergeRecoveryReasonCode::TargetBranchBusy,
        "Deferred",
    );

    meta.append_event_with_state(event, MergeRecoveryState::Deferred);

    assert_eq!(meta.events.len(), 1);
    assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
}

#[test]
fn append_event_trims_when_exceeds_max() {
    let mut meta = MergeRecoveryMetadata::new();

    // Add MAX_EVENTS + 5 events
    for i in 0..(MergeRecoveryMetadata::MAX_EVENTS + 5) {
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            format!("Event {}", i),
        );
        meta.append_event(event);
    }

    // Should only keep MAX_EVENTS
    assert_eq!(meta.events.len(), MergeRecoveryMetadata::MAX_EVENTS);

    // Oldest events should be trimmed, newest kept
    // First event should now be event #5 (0-4 trimmed)
    assert_eq!(meta.events[0].message, "Event 5");
    assert_eq!(
        meta.events[MergeRecoveryMetadata::MAX_EVENTS - 1].message,
        format!("Event {}", MergeRecoveryMetadata::MAX_EVENTS + 4)
    );
}

#[test]
fn append_event_preserves_chronological_order() {
    let mut meta = MergeRecoveryMetadata::new();

    for i in 0..10 {
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            format!("Event {}", i),
        );
        meta.append_event(event);
    }

    // Events should be in order
    for i in 0..10 {
        assert_eq!(meta.events[i].message, format!("Event {}", i));
    }
}

#[test]
fn from_task_metadata_with_no_metadata() {
    let result = MergeRecoveryMetadata::from_task_metadata(None).unwrap();
    assert!(result.is_none());
}

#[test]
fn from_task_metadata_with_empty_json() {
    let result = MergeRecoveryMetadata::from_task_metadata(Some("{}")).unwrap();
    assert!(result.is_none());
}

#[test]
fn from_task_metadata_with_other_keys_only() {
    let json = r#"{"error": "some error", "source_branch": "task-branch"}"#;
    let result = MergeRecoveryMetadata::from_task_metadata(Some(json)).unwrap();
    assert!(result.is_none());
}

#[test]
fn from_task_metadata_with_valid_recovery_data() {
    let json = r#"{
        "error": "some error",
        "merge_recovery": {
            "version": 1,
            "events": [
                {
                    "at": "2026-02-11T10:00:00Z",
                    "kind": "deferred",
                    "source": "system",
                    "reason_code": "target_branch_busy",
                    "message": "Deferred"
                }
            ],
            "last_state": "deferred"
        }
    }"#;

    let result = MergeRecoveryMetadata::from_task_metadata(Some(json)).unwrap();
    assert!(result.is_some());

    let meta = result.unwrap();
    assert_eq!(meta.version, 1);
    assert_eq!(meta.events.len(), 1);
    assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
    assert_eq!(meta.events[0].message, "Deferred");
}

#[test]
fn from_task_metadata_with_invalid_json() {
    let result = MergeRecoveryMetadata::from_task_metadata(Some("not json"));
    assert!(result.is_err());
}

#[test]
fn from_task_metadata_with_invalid_recovery_structure() {
    let json = r#"{"merge_recovery": "not an object"}"#;
    let result = MergeRecoveryMetadata::from_task_metadata(Some(json));
    assert!(result.is_err());
}

#[test]
fn update_task_metadata_creates_new_object() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        ),
        MergeRecoveryState::Deferred,
    );

    let result = meta.update_task_metadata(None).unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(value.get("merge_recovery").is_some());
    assert_eq!(value["merge_recovery"]["version"], 1);
    assert_eq!(
        value["merge_recovery"]["events"].as_array().unwrap().len(),
        1
    );
    assert_eq!(value["merge_recovery"]["last_state"], "deferred");
}

#[test]
fn update_task_metadata_preserves_existing_keys() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        ),
        MergeRecoveryState::Deferred,
    );

    let existing = r#"{"error": "some error", "source_branch": "task-branch"}"#;
    let result = meta.update_task_metadata(Some(existing)).unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["error"], "some error");
    assert_eq!(value["source_branch"], "task-branch");
    assert!(value.get("merge_recovery").is_some());
}

#[test]
fn update_task_metadata_overwrites_existing_recovery() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Retry",
        ),
        MergeRecoveryState::Retrying,
    );

    let existing = r#"{
        "merge_recovery": {
            "version": 1,
            "events": [],
            "last_state": "succeeded"
        }
    }"#;

    let result = meta.update_task_metadata(Some(existing)).unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["merge_recovery"]["events"].as_array().unwrap().len(),
        1
    );
    assert_eq!(value["merge_recovery"]["last_state"], "retrying");
}

#[test]
fn update_task_metadata_handles_malformed_existing() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        ),
        MergeRecoveryState::Deferred,
    );

    // Malformed JSON should be replaced with empty object
    let result = meta.update_task_metadata(Some("not json")).unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(value.get("merge_recovery").is_some());
    // Only merge_recovery key should exist (malformed JSON was discarded)
    assert_eq!(value.as_object().unwrap().len(), 1);
}

#[test]
fn roundtrip_from_and_update_task_metadata() {
    // Create metadata with events
    let mut original = MergeRecoveryMetadata::new();
    original.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        )
        .with_target_branch("main")
        .with_attempt(1),
        MergeRecoveryState::Deferred,
    );

    // Serialize to task metadata JSON
    let json_str = original.update_task_metadata(None).unwrap();

    // Parse back from task metadata JSON
    let parsed = MergeRecoveryMetadata::from_task_metadata(Some(&json_str))
        .unwrap()
        .unwrap();

    // Should match original
    assert_eq!(parsed.version, original.version);
    assert_eq!(parsed.events.len(), original.events.len());
    assert_eq!(parsed.last_state, original.last_state);
    assert_eq!(parsed.events[0].message, "Deferred");
    assert_eq!(parsed.events[0].target_branch, Some("main".to_string()));
    assert_eq!(parsed.events[0].attempt, Some(1));
}

#[test]
fn merge_recovery_reason_code_branch_not_found_serializes() {
    let code = MergeRecoveryReasonCode::BranchNotFound;
    let json = serde_json::to_string(&code).unwrap();
    assert_eq!(json, "\"branch_not_found\"");

    let parsed: MergeRecoveryReasonCode = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, MergeRecoveryReasonCode::BranchNotFound);
}

// ===== Rate limit metadata tests =====

#[test]
fn provider_rate_limited_reason_code_serializes() {
    let code = MergeRecoveryReasonCode::ProviderRateLimited;
    let json = serde_json::to_string(&code).unwrap();
    assert_eq!(json, "\"provider_rate_limited\"");

    let parsed: MergeRecoveryReasonCode = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, MergeRecoveryReasonCode::ProviderRateLimited);
}

#[test]
fn rate_limited_state_serializes() {
    let state = MergeRecoveryState::RateLimited;
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(json, "\"rate_limited\"");

    let parsed: MergeRecoveryState = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, MergeRecoveryState::RateLimited);
}

#[test]
fn rate_limit_retry_after_stored_in_metadata() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.rate_limit_retry_after = Some("2026-02-20T15:00:00+00:00".to_string());
    meta.append_event_with_state(
        MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AttemptFailed,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::ProviderRateLimited,
            "Rate limit hit during merge",
        ),
        MergeRecoveryState::RateLimited,
    );

    let json = meta.update_task_metadata(None).unwrap();
    let restored = MergeRecoveryMetadata::from_task_metadata(Some(&json))
        .unwrap()
        .unwrap();

    assert_eq!(
        restored.rate_limit_retry_after,
        Some("2026-02-20T15:00:00+00:00".to_string())
    );
    assert_eq!(restored.last_state, MergeRecoveryState::RateLimited);
    assert_eq!(
        restored.events[0].reason_code,
        MergeRecoveryReasonCode::ProviderRateLimited
    );
}

#[test]
fn rate_limit_retry_after_none_not_serialized() {
    let meta = MergeRecoveryMetadata::new();
    let json = serde_json::to_string(&meta).unwrap();
    assert!(
        !json.contains("rate_limit_retry_after"),
        "None rate_limit_retry_after should be skipped in serialization"
    );
}

#[test]
fn rate_limit_retry_after_backward_compat_deserialize() {
    // Old metadata without rate_limit_retry_after field should deserialize fine
    let json = r#"{
        "version": 1,
        "events": [],
        "last_state": "succeeded"
    }"#;
    let meta: MergeRecoveryMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.rate_limit_retry_after, None);
}

#[test]
fn rate_limit_retry_after_roundtrip_through_task_metadata() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.rate_limit_retry_after = Some("2026-02-20T15:00:00+00:00".to_string());

    // Write to task metadata with existing keys
    let existing = r#"{"error": "some error"}"#;
    let json = meta.update_task_metadata(Some(existing)).unwrap();

    // Read back
    let restored = MergeRecoveryMetadata::from_task_metadata(Some(&json))
        .unwrap()
        .unwrap();
    assert_eq!(
        restored.rate_limit_retry_after,
        Some("2026-02-20T15:00:00+00:00".to_string())
    );

    // Verify other keys preserved
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["error"], "some error");
}

#[test]
fn rate_limit_cleared_after_expiry() {
    let mut meta = MergeRecoveryMetadata::new();
    meta.rate_limit_retry_after = Some("2026-02-20T15:00:00+00:00".to_string());
    meta.last_state = MergeRecoveryState::RateLimited;

    // Simulate clearing after expiry
    meta.rate_limit_retry_after = None;
    meta.last_state = MergeRecoveryState::Retrying;

    let json = meta.update_task_metadata(None).unwrap();
    let restored = MergeRecoveryMetadata::from_task_metadata(Some(&json))
        .unwrap()
        .unwrap();
    assert_eq!(restored.rate_limit_retry_after, None);
    assert_eq!(restored.last_state, MergeRecoveryState::Retrying);
}

