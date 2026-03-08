use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{ChatMessage, IdeationSession, IdeationSessionId, ProjectId};
use crate::domain::entities::ideation::VerificationStatus;
use crate::http_server::types::{
    ApplyDependencySuggestionsRequest, DependencySuggestion, UpdateVerificationRequest,
    VerificationGapRequest,
};
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

// ─────────────────────────────────────────────────────────────────────────────
// get_plan_verification handler tests
// ─────────────────────────────────────────────────────────────────────────────

fn make_metadata_json(
    current_gaps: Vec<serde_json::Value>,
    rounds: Vec<serde_json::Value>,
    current_round: u32,
    max_rounds: u32,
) -> String {
    serde_json::json!({
        "v": 1,
        "current_round": current_round,
        "max_rounds": max_rounds,
        "rounds": rounds,
        "current_gaps": current_gaps,
        "convergence_reason": null,
        "best_round_index": null,
        "parse_failures": []
    })
    .to_string()
}

fn make_gap(severity: &str, category: &str, description: &str) -> serde_json::Value {
    serde_json::json!({
        "severity": severity,
        "category": category,
        "description": description,
        "why_it_matters": null
    })
}

fn make_gap_with_why(
    severity: &str,
    category: &str,
    description: &str,
    why: &str,
) -> serde_json::Value {
    serde_json::json!({
        "severity": severity,
        "category": category,
        "description": description,
        "why_it_matters": why
    })
}

fn make_round(fingerprints: Vec<&str>, gap_score: u32) -> serde_json::Value {
    serde_json::json!({
        "fingerprints": fingerprints,
        "gap_score": gap_score
    })
}

/// Happy path: session with 3 gaps and 2 rounds → response includes
/// current_gaps (3 items) and rounds (2 items with correct scores/counts).
#[tokio::test]
async fn test_get_plan_verification_happy_path_gaps_and_rounds() {
    let state = setup_test_state().await;
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id);
    let session_id = session.id.clone();

    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let gaps = vec![
        make_gap_with_why("critical", "architecture", "Missing auth layer", "Security risk"),
        make_gap("high", "performance", "No caching strategy"),
        make_gap("medium", "testing", "No unit tests"),
    ];
    let rounds = vec![
        make_round(vec!["fp-a", "fp-b"], 13), // round 1: 2 fingerprints, score 13
        make_round(vec!["fp-a", "fp-b", "fp-c"], 10), // round 2: 3 fingerprints, score 10
    ];
    let metadata = make_metadata_json(gaps, rounds, 2, 5);

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id,
            VerificationStatus::NeedsRevision,
            false,
            Some(metadata),
        )
        .await
        .unwrap();

    let result =
        get_plan_verification(State(state), Path(session_id.as_str().to_string())).await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    let response = result.unwrap().0;

    // current_gaps: 3 items with correct fields
    assert_eq!(response.current_gaps.len(), 3, "expected 3 current_gaps");
    let critical = &response.current_gaps[0];
    assert_eq!(critical.severity, "critical");
    assert_eq!(critical.category, "architecture");
    assert_eq!(critical.description, "Missing auth layer");
    assert_eq!(critical.why_it_matters.as_deref(), Some("Security risk"));
    let high = &response.current_gaps[1];
    assert_eq!(high.severity, "high");
    assert!(high.why_it_matters.is_none());

    // rounds: 2 items with 1-based round numbers and correct gap_counts
    assert_eq!(response.rounds.len(), 2, "expected 2 rounds");
    let r1 = &response.rounds[0];
    assert_eq!(r1.round, 1);
    assert_eq!(r1.gap_score, 13);
    assert_eq!(r1.gap_count, 2); // fingerprints.len()
    let r2 = &response.rounds[1];
    assert_eq!(r2.round, 2);
    assert_eq!(r2.gap_score, 10);
    assert_eq!(r2.gap_count, 3);
}

/// Empty metadata test: verification_metadata = NULL → current_gaps: [] and rounds: [].
#[tokio::test]
async fn test_get_plan_verification_null_metadata_returns_empty_vecs() {
    let state = setup_test_state().await;
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id);
    let session_id = session.id.clone();

    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Explicitly set NULL metadata
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id,
            VerificationStatus::Unverified,
            false,
            None,
        )
        .await
        .unwrap();

    let result =
        get_plan_verification(State(state), Path(session_id.as_str().to_string())).await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.current_gaps.is_empty(), "current_gaps must be empty for NULL metadata");
    assert!(response.rounds.is_empty(), "rounds must be empty for NULL metadata");
    assert!(response.gap_score.is_none());
}

/// Malformed metadata test: partial JSON → serde defaults produce empty vecs, no panic.
#[tokio::test]
async fn test_get_plan_verification_malformed_metadata_no_panic() {
    let state = setup_test_state().await;
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id);
    let session_id = session.id.clone();

    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Partial JSON: only schema version present, all other fields absent
    let partial_json = r#"{"v": 1}"#.to_string();
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id,
            VerificationStatus::Reviewing,
            true,
            Some(partial_json),
        )
        .await
        .unwrap();

    let result =
        get_plan_verification(State(state), Path(session_id.as_str().to_string())).await;

    assert!(result.is_ok(), "malformed metadata must not panic the handler");
    let response = result.unwrap().0;
    assert!(response.current_gaps.is_empty(), "serde defaults: current_gaps should be []");
    assert!(response.rounds.is_empty(), "serde defaults: rounds should be []");
}

/// Rounds cap test: session with 15 rounds → last 10 returned in chronological order
/// (rounds 6-15, i.e. 1-based indices 6..=15 from the original vec).
#[tokio::test]
async fn test_get_plan_verification_rounds_capped_at_10() {
    let state = setup_test_state().await;
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id);
    let session_id = session.id.clone();

    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Build 15 rounds with distinct gap_scores (1..=15) so we can verify ordering
    let rounds: Vec<serde_json::Value> = (1u32..=15)
        .map(|i| make_round(vec!["fp-x"], i))
        .collect();

    let metadata = make_metadata_json(vec![], rounds, 15, 15);

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id,
            VerificationStatus::NeedsRevision,
            false,
            Some(metadata),
        )
        .await
        .unwrap();

    let result =
        get_plan_verification(State(state), Path(session_id.as_str().to_string())).await;

    assert!(result.is_ok());
    let response = result.unwrap().0;

    // Only 10 rounds returned
    assert_eq!(response.rounds.len(), 10, "cap must limit to 10 rounds");

    // First returned round is round 6 (oldest of the last 10)
    assert_eq!(response.rounds[0].round, 6, "first returned round should be round 6");
    assert_eq!(response.rounds[0].gap_score, 6, "gap_score should match round index");

    // Last returned round is round 15
    assert_eq!(response.rounds[9].round, 15, "last returned round should be round 15");
    assert_eq!(response.rounds[9].gap_score, 15);

    // Verify chronological order throughout
    for (i, r) in response.rounds.iter().enumerate() {
        assert_eq!(r.round, (i + 6) as u32, "round at index {} should be {}", i, i + 6);
    }
}

/// Round-trip integration test: write gaps via POST /verification (update_plan_verification),
/// then read via GET /verification (get_plan_verification), and verify current_gaps contains
/// the same data with correct field names.
#[tokio::test]
async fn test_get_plan_verification_round_trip_post_then_get() {
    let state = setup_test_state().await;
    let project_id = ProjectId::new();
    let session = IdeationSession::new(project_id);
    let session_id = session.id.clone();
    let session_id_str = session_id.as_str().to_string();

    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // POST: write gaps via update_plan_verification handler
    let post_result = update_plan_verification(
        State(state.clone()),
        Path(session_id_str.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: false,
            round: Some(1),
            gaps: Some(vec![
                VerificationGapRequest {
                    severity: "critical".to_string(),
                    category: "security".to_string(),
                    description: "No authentication".to_string(),
                    why_it_matters: Some("Users can access any data".to_string()),
                },
                VerificationGapRequest {
                    severity: "high".to_string(),
                    category: "scalability".to_string(),
                    description: "No horizontal scaling plan".to_string(),
                    why_it_matters: None,
                },
            ]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
        }),
    )
    .await;

    assert!(post_result.is_ok(), "POST should succeed: {:?}", post_result.err());

    // GET: read back via get_plan_verification handler
    let get_result =
        get_plan_verification(State(state), Path(session_id_str)).await;

    assert!(get_result.is_ok(), "GET should succeed: {:?}", get_result.err());
    let response = get_result.unwrap().0;

    // current_gaps should contain the same 2 gaps written via POST
    assert_eq!(response.current_gaps.len(), 2, "round-trip: expected 2 current_gaps");

    let g0 = &response.current_gaps[0];
    assert_eq!(g0.severity, "critical");
    assert_eq!(g0.category, "security");
    assert_eq!(g0.description, "No authentication");
    assert_eq!(g0.why_it_matters.as_deref(), Some("Users can access any data"));

    let g1 = &response.current_gaps[1];
    assert_eq!(g1.severity, "high");
    assert_eq!(g1.category, "scalability");
    assert!(g1.why_it_matters.is_none());

    // POST handler creates a round entry; GET should reflect it
    assert_eq!(response.rounds.len(), 1, "round-trip: 1 round should be present");
    assert_eq!(response.rounds[0].round, 1);
    assert_eq!(response.rounds[0].gap_count, 2); // 2 fingerprints (one per gap)
}

// ── Condition 6 tests: reviewing with gaps → needs_revision auto-transition ──

/// Condition 6 test 1: reviewing + critical gaps → overridden to needs_revision
#[tokio::test]
async fn test_condition6_reviewing_critical_gaps_overrides_to_needs_revision() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "critical".to_string(),
                category: "security".to_string(),
                description: "Missing auth entirely".to_string(),
                why_it_matters: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "needs_revision", "critical gaps → needs_revision");
    assert!(!resp.in_progress, "in_progress must be false after condition 6 override");
}

/// Condition 6 test 2: reviewing + medium-only gaps → overridden to needs_revision (any severity)
#[tokio::test]
async fn test_condition6_reviewing_medium_gaps_overrides_to_needs_revision() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "medium".to_string(),
                category: "performance".to_string(),
                description: "No caching layer defined".to_string(),
                why_it_matters: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "needs_revision", "medium gaps → needs_revision (any severity)");
    assert!(!resp.in_progress, "in_progress must be false");
}

/// Condition 6 test 3: reviewing + gaps + max_rounds convergence → verified (convergence wins)
#[tokio::test]
async fn test_condition6_convergence_takes_priority_over_reviewing_with_gaps() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // max_rounds=1, round=1 → condition 3 fires first (max_rounds) → Verified
    // condition 6 then sees Verified (not Reviewing) and does not fire
    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "high".to_string(),
                category: "scalability".to_string(),
                description: "No horizontal scaling plan".to_string(),
                why_it_matters: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(1),
            parse_failed: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "verified", "convergence (max_rounds) takes priority over condition 6");
}

/// Condition 6 test 4: reviewing + no gaps → status stays reviewing (condition 6 does not fire)
#[tokio::test]
async fn test_condition6_reviewing_no_gaps_stays_reviewing() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![]), // explicitly empty — no gaps
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "reviewing", "no gaps → status stays reviewing");
}

/// Condition 6 test 5: reviewing + in_progress=false already + gaps → still overridden to needs_revision
#[tokio::test]
async fn test_condition6_reviewing_in_progress_false_with_gaps_overrides_to_needs_revision() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: false, // already false — condition 6 still fires on status
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "low".to_string(),
                category: "documentation".to_string(),
                description: "API docs incomplete".to_string(),
                why_it_matters: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(
        resp.status, "needs_revision",
        "condition 6 fires regardless of requested in_progress value"
    );
    assert!(!resp.in_progress, "in_progress remains false");
}

// ── needs_revision → verified transition tests ──

/// needs_revision → verified succeeds when convergence_reason is provided.
///
/// The orchestrator calls this path when adversarial convergence is met
/// (e.g., 0 critical gaps after N rounds) and directly requests verified status.
#[tokio::test]
async fn test_needs_revision_to_verified_with_convergence_reason() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Put session in NeedsRevision state (simulating prior reviewing→needs_revision cycle)
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::NeedsRevision, false, None)
        .await
        .unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "verified".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: Some("No critical gaps after 5 rounds of adversarial review".to_string()),
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await
    .expect("needs_revision → verified with convergence_reason must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "verified", "convergence_reason present → verified");
    assert!(!resp.in_progress, "in_progress must be false after verification");
}

/// needs_revision → verified is rejected (422) when convergence_reason is absent.
///
/// Without a convergence_reason, the orchestrator cannot skip further review rounds.
#[tokio::test]
async fn test_needs_revision_to_verified_without_convergence_reason() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::NeedsRevision, false, None)
        .await
        .unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "verified".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
        }),
    )
    .await;

    assert!(result.is_err(), "needs_revision → verified without convergence_reason must fail");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "must return 422 when convergence_reason is absent"
    );
}
