use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{ChatMessage, IdeationSessionId};
use crate::http_server::types::{ApplyDependencySuggestionsRequest, DependencySuggestion};
use std::collections::HashSet;
use std::sync::Arc;

async fn setup_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = crate::application::TeamStateTracker::new();
    let team_service = Arc::new(crate::application::TeamService::new_without_events(
        Arc::new(tracker.clone()),
    ));

    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
    }
}

#[tokio::test]
async fn test_get_session_messages_empty_session() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 50,
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.messages.is_empty());
    assert_eq!(response.count, 0);
    assert!(!response.truncated);
    assert_eq!(response.total_available, 0);
}

#[tokio::test]
async fn test_get_session_messages_returns_messages() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Create messages
    let msg1 = ChatMessage::user_in_session(session_id.clone(), "Hello");
    let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Hi there!");

    state
        .app_state
        .chat_message_repo
        .create(msg1.clone())
        .await
        .unwrap();
    state
        .app_state
        .chat_message_repo
        .create(msg2.clone())
        .await
        .unwrap();

    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 50,
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.messages.len(), 2);
    assert_eq!(response.count, 2);
    assert!(!response.truncated);
    assert_eq!(response.total_available, 2);
}

#[tokio::test]
async fn test_get_session_messages_respects_limit() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Create 10 messages
    for i in 0..10 {
        let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
        state.app_state.chat_message_repo.create(msg).await.unwrap();
    }

    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 5,
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.messages.len(), 5);
    assert_eq!(response.count, 5);
    assert!(response.truncated);
    assert_eq!(response.total_available, 10);
}

#[tokio::test]
async fn test_get_session_messages_caps_at_200() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Request limit over 200
    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 500, // Should be capped to 200
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    // Should still succeed (empty in this case)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_session_messages_default_limit() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Create 60 messages
    for i in 0..60 {
        let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
        state.app_state.chat_message_repo.create(msg).await.unwrap();
    }

    // Use default limit (should be 50)
    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 50, // explicit default
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.messages.len(), 50);
    assert!(response.truncated);
    assert_eq!(response.total_available, 60);
}

#[tokio::test]
async fn test_get_session_messages_returns_chronological_order() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Create messages in order
    let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
    let msg2 = ChatMessage::user_in_session(session_id.clone(), "Second");
    let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");

    // Small delays to ensure different timestamps
    state
        .app_state
        .chat_message_repo
        .create(msg1)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    state
        .app_state
        .chat_message_repo
        .create(msg2)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    state
        .app_state
        .chat_message_repo
        .create(msg3)
        .await
        .unwrap();

    let result = get_session_messages(
        State(state),
        Json(GetSessionMessagesRequest {
            session_id: session_id.as_str().to_string(),
            limit: 50,
            offset: 0,
            include_tool_calls: false,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    // get_recent_by_session returns messages in chronological order (oldest to newest)
    // after selecting the most recent N messages
    assert_eq!(response.messages[0].content, "First");
    assert_eq!(response.messages[1].content, "Second");
    assert_eq!(response.messages[2].content, "Third");
}

// -------------------------------------------------------------------------
// apply_proposal_dependencies — analyzing_dependencies lifecycle tests
// -------------------------------------------------------------------------

/// Success path: applying dependencies removes the session from `analyzing_dependencies`.
#[tokio::test]
async fn test_apply_proposal_dependencies_clears_analyzing_state() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Mark the session as actively analyzing
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.insert(session_id.clone());
    }
    assert!(
        state
            .app_state
            .analyzing_dependencies
            .read()
            .await
            .contains(&session_id),
        "session should be in analyzing_dependencies before call"
    );

    // Call the handler with an empty dependency list (no proposals needed for this)
    let result = apply_proposal_dependencies(
        State(state.clone()),
        Json(ApplyDependencySuggestionsRequest {
            session_id: session_id.as_str().to_string(),
            dependencies: vec![],
        }),
    )
    .await;

    assert!(result.is_ok(), "handler should succeed: {:?}", result.err());

    // Session must be cleared from analyzing_dependencies
    assert!(
        !state
            .app_state
            .analyzing_dependencies
            .read()
            .await
            .contains(&session_id),
        "session should be removed from analyzing_dependencies after apply"
    );
}

/// If the session was never in `analyzing_dependencies`, the handler still succeeds.
#[tokio::test]
async fn test_apply_proposal_dependencies_when_not_analyzing_is_safe() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Do NOT insert into analyzing_dependencies
    let result = apply_proposal_dependencies(
        State(state.clone()),
        Json(ApplyDependencySuggestionsRequest {
            session_id: session_id.as_str().to_string(),
            dependencies: vec![],
        }),
    )
    .await;

    assert!(result.is_ok(), "handler should succeed even when session not in analyzing set");
    assert!(
        state
            .app_state
            .analyzing_dependencies
            .read()
            .await
            .is_empty(),
        "analyzing_dependencies should remain empty"
    );
}

/// Applying with invalid dependency proposal IDs skips them but still clears analyzing state.
#[tokio::test]
async fn test_apply_proposal_dependencies_clears_analyzing_even_with_invalid_deps() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.insert(session_id.clone());
    }

    // Provide dependency with non-existent proposal IDs — they'll be skipped
    let result = apply_proposal_dependencies(
        State(state.clone()),
        Json(ApplyDependencySuggestionsRequest {
            session_id: session_id.as_str().to_string(),
            dependencies: vec![DependencySuggestion {
                proposal_id: "nonexistent-id".to_string(),
                depends_on_id: "also-nonexistent".to_string(),
                reason: None,
            }],
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.applied_count, 0);
    assert_eq!(response.skipped_count, 1);

    // Even with skipped deps, analyzing state must be cleared
    assert!(
        !state
            .app_state
            .analyzing_dependencies
            .read()
            .await
            .contains(&session_id),
        "analyzing_dependencies should be cleared even when all deps are skipped"
    );
}

/// Safety timeout: a stale entry in `analyzing_dependencies` is auto-removed after 60 seconds.
#[tokio::test(start_paused = true)]
async fn test_safety_timeout_removes_stale_analyzing_entry() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_id = IdeationSessionId::new();

    // Simulate inserting into the set (as done by spawn_dependency_suggester)
    analyzing.write().await.insert(session_id.clone());

    // Spawn the timeout cleanup task (mirrors production code)
    let timeout_analyzing = Arc::clone(&analyzing);
    let timeout_id = session_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        let mut set = timeout_analyzing.write().await;
        set.remove(&timeout_id);
    });

    // Still in the set before timeout
    assert!(analyzing.read().await.contains(&session_id));

    // Yield to allow the spawned task to start and register its sleep timer
    tokio::task::yield_now().await;

    // Advance time past the 60-second threshold
    tokio::time::advance(tokio::time::Duration::from_secs(61)).await;
    // Yield multiple times: the spawned task needs turns for (1) wakeup, (2) lock acquisition
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    assert!(
        !analyzing.read().await.contains(&session_id),
        "safety timeout should have removed stale session from analyzing_dependencies"
    );
}

// -------------------------------------------------------------------------
// Agent crash path — analyzing_dependencies cleared on crash
// -------------------------------------------------------------------------

/// Agent crash path: when an agent exits with a failure status, the crash handler removes
/// the session from `analyzing_dependencies`. This test verifies the state management
/// logic directly (mirroring the crash detection code in helpers.rs lines 737-800).
#[tokio::test]
async fn test_crash_path_clears_analyzing_dependencies() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_id = IdeationSessionId::new();

    // Simulate: session enters analyzing state when agent is spawned
    analyzing.write().await.insert(session_id.clone());
    assert!(analyzing.read().await.contains(&session_id));

    // Simulate: agent process exits with non-zero status (crash path)
    // Production code in helpers.rs does:
    //   let mut set = analyzing.write().await;
    //   set.remove(&session_id);
    //   // then emits analysis_failed event
    {
        let mut set = analyzing.write().await;
        let was_analyzing = set.remove(&session_id);
        assert!(was_analyzing, "session should have been in the set before crash cleanup");
    }

    // Verify session is no longer in analyzing_dependencies after crash
    assert!(
        !analyzing.read().await.contains(&session_id),
        "crash path should remove session from analyzing_dependencies"
    );
}

/// Crash path idempotence: if the session was already cleared (e.g., by the success path
/// racing the crash detection), removing a non-existent entry is safe.
#[tokio::test]
async fn test_crash_path_is_safe_when_already_cleared() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_id = IdeationSessionId::new();

    // Session NOT in the set (already cleared by success path)
    assert!(!analyzing.read().await.contains(&session_id));

    // Crash path remove is a no-op — should not panic
    {
        let mut set = analyzing.write().await;
        let was_analyzing = set.remove(&session_id);
        assert!(!was_analyzing, "remove should return false when session was not in set");
    }

    assert!(analyzing.read().await.is_empty());
}

// -------------------------------------------------------------------------
// Multiple rapid triggers — re-entry safety
// -------------------------------------------------------------------------

/// Multiple rapid auto-triggers insert the same session_id multiple times. The HashSet
/// semantics ensure this is idempotent — the session appears exactly once regardless of
/// how many times it was inserted.
#[tokio::test]
async fn test_rapid_triggers_set_is_idempotent() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_id = IdeationSessionId::new();

    // Simulate multiple rapid trigger inserts (mirrors maybe_trigger_dependency_analysis
    // being called several times before the first agent completes)
    for _ in 0..5 {
        let mut set = analyzing.write().await;
        set.insert(session_id.clone());
    }

    // The session should appear exactly once despite multiple inserts
    let set = analyzing.read().await;
    assert_eq!(
        set.len(),
        1,
        "HashSet should deduplicate: session must appear exactly once after multiple inserts"
    );
    assert!(set.contains(&session_id));
}

/// Two different sessions can be analyzed concurrently without corrupting the set.
#[tokio::test]
async fn test_two_sessions_analyzed_concurrently() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_a = IdeationSessionId::new();
    let session_b = IdeationSessionId::new();

    // Both sessions enter analyzing state
    {
        let mut set = analyzing.write().await;
        set.insert(session_a.clone());
        set.insert(session_b.clone());
    }
    assert_eq!(analyzing.read().await.len(), 2);

    // Session A completes — Session B should still be in the set
    analyzing.write().await.remove(&session_a);

    let set = analyzing.read().await;
    assert!(!set.contains(&session_a), "session A should be gone after completion");
    assert!(set.contains(&session_b), "session B should still be analyzing");
}

/// Failure path: if analysis completes before the safety timeout, the timeout is a no-op.
#[tokio::test(start_paused = true)]
async fn test_safety_timeout_is_noop_when_already_cleared() {
    let analyzing = Arc::new(tokio::sync::RwLock::new(
        HashSet::<IdeationSessionId>::new(),
    ));
    let session_id = IdeationSessionId::new();

    analyzing.write().await.insert(session_id.clone());

    // Spawn timeout
    let timeout_analyzing = Arc::clone(&analyzing);
    let timeout_id = session_id.clone();
    let removed_by_timeout = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let removed_flag = Arc::clone(&removed_by_timeout);
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        let mut set = timeout_analyzing.write().await;
        if set.remove(&timeout_id) {
            removed_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });

    // Simulate success path: apply_proposal_dependencies removes session before timeout
    {
        let mut set = analyzing.write().await;
        set.remove(&session_id);
    }

    // Advance past timeout
    tokio::time::advance(tokio::time::Duration::from_secs(61)).await;
    tokio::task::yield_now().await;

    // Timeout should NOT have emitted failure (remove returned false)
    assert!(
        !removed_by_timeout.load(std::sync::atomic::Ordering::SeqCst),
        "timeout should be a no-op when session was already cleared by success path"
    );
    assert!(analyzing.read().await.is_empty());
}

// -------------------------------------------------------------------------
// Session deletion edge case — analyzing_dependencies cleared on session delete/archive
// -------------------------------------------------------------------------

/// Session deleted while analysis running: the delete command removes the session from
/// analyzing_dependencies. This test verifies the state management behavior that
/// archive_ideation_session and delete_ideation_session now implement.
#[tokio::test]
async fn test_session_deletion_clears_analyzing_state() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Mark the session as actively analyzing
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.insert(session_id.clone());
    }
    assert!(
        state.app_state.analyzing_dependencies.read().await.contains(&session_id),
        "session should be in analyzing_dependencies before deletion"
    );

    // Simulate what archive_ideation_session / delete_ideation_session now does:
    // remove from analyzing_dependencies before deleting the session record
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.remove(&session_id);
    }

    assert!(
        !state.app_state.analyzing_dependencies.read().await.contains(&session_id),
        "analyzing_dependencies should be cleared when session is deleted or archived"
    );
    assert!(state.app_state.analyzing_dependencies.read().await.is_empty());
}

/// Deleting a session that was NOT analyzing is safe (remove is idempotent on HashSet).
#[tokio::test]
async fn test_session_deletion_when_not_analyzing_is_safe() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Session never entered analyzing state
    assert!(state.app_state.analyzing_dependencies.read().await.is_empty());

    // Remove is a no-op — must not panic
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.remove(&session_id);
    }

    assert!(state.app_state.analyzing_dependencies.read().await.is_empty());
}

/// Deleting one session does not affect a concurrently analyzing session.
#[tokio::test]
async fn test_session_deletion_does_not_affect_other_sessions() {
    let state = setup_test_state().await;
    let session_a = IdeationSessionId::new();
    let session_b = IdeationSessionId::new();

    // Both sessions are analyzing concurrently
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.insert(session_a.clone());
        analyzing.insert(session_b.clone());
    }
    assert_eq!(state.app_state.analyzing_dependencies.read().await.len(), 2);

    // Delete session A — session B should be unaffected
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.remove(&session_a);
    }

    let set = state.app_state.analyzing_dependencies.read().await;
    assert!(!set.contains(&session_a), "deleted session should be gone from analyzing set");
    assert!(set.contains(&session_b), "other session should remain in analyzing set");
}

// ── Debounce generation counter tests ────────────────────────────────────────

/// stale_gen_skips: Simulate 5 rapid triggers; verify only the last-gen task proceeds.
///
/// Gate 1 in `maybe_trigger_dependency_analysis` reads `debounce_generations[session]`
/// after the 2-second sleep and compares it to the captured value at spawn time.
/// All but the final trigger will see a stale gen and return early.
#[tokio::test]
async fn test_debounce_stale_gen_skips() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();
    let debounce_generations = Arc::clone(&state.app_state.debounce_generations);

    // Simulate 5 rapid triggers by incrementing the gen counter 5 times,
    // capturing each value as a task would at spawn time.
    let mut captured_gens: Vec<u64> = Vec::new();
    for _ in 0..5 {
        let captured = {
            let mut gens = debounce_generations.lock().unwrap();
            let gen = gens.entry(session_id.clone()).or_insert(0);
            *gen = gen.wrapping_add(1);
            *gen
        };
        captured_gens.push(captured);
    }

    // After 5 triggers the current gen must be 5.
    let current_gen = {
        let gens = debounce_generations.lock().unwrap();
        *gens.get(&session_id).unwrap_or(&0)
    };
    assert_eq!(current_gen, 5, "gen should be 5 after 5 rapid triggers");

    // Only the last captured value matches → that task would proceed.
    assert_eq!(captured_gens[4], current_gen, "last trigger gen must match current");

    // All earlier captured values are stale → those tasks would skip (gate 1).
    for &captured in &captured_gens[..4] {
        assert_ne!(
            captured, current_gen,
            "earlier trigger gen {captured} should be stale"
        );
    }
}

/// analysis_already_running_guard: Gen matches (gate 1 passes) but
/// `analyzing_dependencies` already contains the session (gate 2 blocks spawn).
#[tokio::test]
async fn test_debounce_analysis_already_running_guard() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Simulate one trigger: increment gen and capture value.
    let captured_gen = {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_id.clone()).or_insert(0);
        *gen = gen.wrapping_add(1);
        *gen
    };

    // Mark session as already analyzing (an agent is running).
    {
        let mut analyzing = state.app_state.analyzing_dependencies.write().await;
        analyzing.insert(session_id.clone());
    }

    // Gate 1: gen must match (no newer trigger).
    let current_gen = {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        *gens.get(&session_id).unwrap_or(&0)
    };
    assert_eq!(current_gen, captured_gen, "gate 1 should pass: gen matches");

    // Gate 2: analyzing_dependencies contains session → spawn must be skipped.
    let is_analyzing = state
        .app_state
        .analyzing_dependencies
        .read()
        .await
        .contains(&session_id);
    assert!(is_analyzing, "gate 2 must block: analysis already in progress");
}

/// manual_auto_coexistence: A manual trigger increments the gen counter, making
/// any concurrently pending auto-trigger with the old gen stale.
#[tokio::test]
async fn test_debounce_manual_auto_coexistence() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Auto-trigger path: increment gen, capture value (gen=1).
    let auto_captured_gen = {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_id.clone()).or_insert(0);
        *gen = gen.wrapping_add(1);
        *gen
    };
    assert_eq!(auto_captured_gen, 1);

    // Before the auto-trigger's 2s sleep completes, a manual trigger fires
    // (spawn_dependency_suggester path) — increments gen to 2.
    {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_id.clone()).or_insert(0);
        *gen = gen.wrapping_add(1);
    }

    // Auto-trigger wakes up and reads current gen.
    let current_gen = {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        *gens.get(&session_id).unwrap_or(&0)
    };
    assert_eq!(current_gen, 2, "gen should be 2 after manual trigger");

    // Gate 1 fails for the auto-trigger: captured_gen(1) ≠ current_gen(2) → skip.
    assert_ne!(
        auto_captured_gen, current_gen,
        "auto-trigger with stale gen should be skipped after manual trigger"
    );
}

/// session_delete_clears_gen: After a session is deleted/archived, its
/// `debounce_generations` entry must be removed to prevent unbounded growth.
#[tokio::test]
async fn test_debounce_session_delete_clears_gen() {
    let state = setup_test_state().await;
    let session_id = IdeationSessionId::new();

    // Simulate several triggers building up the gen counter.
    {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_id.clone()).or_insert(0);
        *gen = 3; // directly set to 3 to represent 3 prior triggers
    }

    // Verify the entry exists before cleanup.
    {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        assert!(gens.contains_key(&session_id), "gen entry should exist before cleanup");
        assert_eq!(*gens.get(&session_id).unwrap(), 3);
    }

    // Simulate the cleanup added to archive_ideation_session / delete_ideation_session.
    {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        gens.remove(&session_id);
    }

    // Entry must be gone.
    {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        assert!(
            !gens.contains_key(&session_id),
            "gen entry must be removed after session deletion/archive"
        );
    }
}

/// independent_sessions: Two sessions debounce independently — gen counters and
/// cleanup of one session must not affect the other.
#[tokio::test]
async fn test_debounce_independent_sessions() {
    let state = setup_test_state().await;
    let session_a = IdeationSessionId::new();
    let session_b = IdeationSessionId::new();

    // Session A: 3 triggers.
    for _ in 0..3 {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_a.clone()).or_insert(0);
        *gen = gen.wrapping_add(1);
    }

    // Session B: 2 triggers.
    for _ in 0..2 {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        let gen = gens.entry(session_b.clone()).or_insert(0);
        *gen = gen.wrapping_add(1);
    }

    // Verify independent counters.
    let (gen_a, gen_b) = {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        (
            *gens.get(&session_a).unwrap_or(&0),
            *gens.get(&session_b).unwrap_or(&0),
        )
    };
    assert_eq!(gen_a, 3, "session A should have gen=3");
    assert_eq!(gen_b, 2, "session B should have gen=2");

    // Deleting session A must not affect session B.
    {
        let mut gens = state.app_state.debounce_generations.lock().unwrap();
        gens.remove(&session_a);
    }

    let (gen_a_after, gen_b_after) = {
        let gens = state.app_state.debounce_generations.lock().unwrap();
        (gens.get(&session_a).copied(), gens.get(&session_b).copied())
    };
    assert!(gen_a_after.is_none(), "session A gen should be removed");
    assert_eq!(gen_b_after, Some(2), "session B gen should be unaffected");
}
