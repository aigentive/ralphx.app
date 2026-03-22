use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use ralphx_lib::application::{AppState, InteractiveProcessKey, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::ideation::{SessionOrigin, VerificationStatus};
use ralphx_lib::domain::entities::{
    ChatContextType, ChatMessage, IdeationSession, IdeationSessionBuilder, IdeationSessionId,
    ProjectId,
};
use ralphx_lib::domain::services::RunningAgentKey;
use ralphx_lib::http_server::handlers::*;
use ralphx_lib::http_server::project_scope::ProjectScope;
use ralphx_lib::http_server::types::{
    ChildSessionStatusParams, SendSessionMessageRequest, UpdateVerificationRequest,
    VerificationGapRequest,
};
use ralphx_lib::http_server::types::HttpServerState;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

fn unrestricted_scope() -> ProjectScope {
    ProjectScope(None)
}

async fn setup_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));

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
        get_plan_verification(State(state), unrestricted_scope(), Path(session_id.as_str().to_string())).await;

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
        get_plan_verification(State(state), unrestricted_scope(), Path(session_id.as_str().to_string())).await;

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
        get_plan_verification(State(state), unrestricted_scope(), Path(session_id.as_str().to_string())).await;

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
        get_plan_verification(State(state), unrestricted_scope(), Path(session_id.as_str().to_string())).await;

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
                    source: None,
                },
                VerificationGapRequest {
                    severity: "high".to_string(),
                    category: "scalability".to_string(),
                    description: "No horizontal scaling plan".to_string(),
                    why_it_matters: None,
                    source: None,
                },
            ]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(post_result.is_ok(), "POST should succeed: {:?}", post_result.err());

    // GET: read back via get_plan_verification handler
    let get_result =
        get_plan_verification(State(state), unrestricted_scope(), Path(session_id_str)).await;

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
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
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
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
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
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(1),
            parse_failed: None,
            generation: None,
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
            generation: None,
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
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
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
            generation: None,
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
            generation: None,
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

// ── Auto-verifier integration tests ──
// These tests verify the server-side behavior that the auto-verifier agent relies on.

/// Zombie protection: generation mismatch → 409 CONFLICT
///
/// When a stale auto-verifier agent sends `in_progress=true` with an outdated
/// generation counter (e.g., because the verification was reset and a new run started),
/// the server must reject it with 409 CONFLICT to prevent two agents from running
/// simultaneously and corrupting state.
#[tokio::test]
async fn test_zombie_generation_mismatch() {
    let state = setup_test_state().await;

    // Create a session with generation=5 (simulates a reset that incremented the counter)
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(5)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Stale agent sends generation=999 → must be rejected with 409
    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: Some(999),
        }),
    )
    .await;

    assert!(result.is_err(), "stale generation must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, StatusCode::CONFLICT, "must return 409 CONFLICT for generation mismatch");

    // Correct generation (5) → must succeed
    let result_ok = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: Some(5),
        }),
    )
    .await;

    assert!(result_ok.is_ok(), "correct generation must succeed: {:?}", result_ok.err());
}

/// Zombie protection: no generation provided → guard does not fire
///
/// When `generation` is None (not provided by the agent), the guard is skipped
/// and the call proceeds normally regardless of the stored generation value.
#[tokio::test]
async fn test_zombie_guard_skipped_when_no_generation_provided() {
    let state = setup_test_state().await;

    // Session with generation=7
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(7)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // No generation → guard not triggered → must succeed even though stored generation is 7
    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None, // no generation = no guard check
        }),
    )
    .await;

    assert!(result.is_ok(), "missing generation must not trigger the guard: {:?}", result.err());
}

/// Zombie protection: terminal call (in_progress=false) with stale generation → 409 CONFLICT
///
/// Terminal calls (verified, needs_revision, skipped) must also be guarded.
/// A zombie agent that finished after a reset must not overwrite the new agent's terminal status.
#[tokio::test]
async fn test_zombie_terminal_call_stale_generation_rejected() {
    let state = setup_test_state().await;

    // Session at Reviewing with generation=3 (simulates a reset mid-verification)
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(3)
        .verification_status(VerificationStatus::Reviewing)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Zombie agent sends terminal status (in_progress=false) with stale generation=1 → must be rejected
    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: false,
            round: Some(2),
            gaps: None,
            convergence_reason: Some("max_rounds".to_string()),
            max_rounds: None,
            parse_failed: None,
            generation: Some(1), // Stale — reset incremented to 3
        }),
    )
    .await;

    assert!(result.is_err(), "terminal call with stale generation must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "must return 409 CONFLICT for stale generation on terminal call"
    );
}

/// Zombie protection: terminal call (in_progress=false) with correct generation → success
///
/// A legitimate agent finishing its round loop must be able to write terminal status
/// when it provides the correct current generation.
#[tokio::test]
async fn test_zombie_terminal_call_correct_generation_succeeds() {
    let state = setup_test_state().await;

    // Session at Reviewing with generation=3
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(3)
        .verification_status(VerificationStatus::Reviewing)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Correct generation → terminal call must succeed
    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: false,
            round: Some(2),
            gaps: None,
            convergence_reason: Some("max_rounds".to_string()),
            max_rounds: None,
            parse_failed: None,
            generation: Some(3), // Correct generation
        }),
    )
    .await;

    assert!(
        result.is_ok(),
        "terminal call with correct generation must succeed: {:?}",
        result.err()
    );
}

/// Zombie protection: terminal call without generation parameter → guard does not fire
///
/// Backward compatibility: callers that omit generation entirely are not affected
/// by the guard, regardless of whether the call is in_progress=true or false.
#[tokio::test]
async fn test_zombie_terminal_call_no_generation_no_guard() {
    let state = setup_test_state().await;

    // Session at Reviewing with generation=5
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(5)
        .verification_status(VerificationStatus::Reviewing)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // No generation parameter → guard skipped, terminal call proceeds normally
    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: false,
            round: Some(1),
            gaps: None,
            convergence_reason: Some("max_rounds".to_string()),
            max_rounds: None,
            parse_failed: None,
            generation: None, // No generation = no guard check
        }),
    )
    .await;

    assert!(
        result.is_ok(),
        "terminal call without generation must not trigger the guard: {:?}",
        result.err()
    );
}

/// Empty round guard: round 1 with 0 gaps does NOT trigger convergence.
///
/// A critic that finds 0 gaps in round 1 may simply be broken or confused.
/// The server requires at least round 2 before accepting zero_blocking convergence.
/// After round 1 with 0 gaps, the status should remain reviewing (condition 6 doesn't
/// fire because there are no gaps), not verified.
#[tokio::test]
async fn test_single_round_zero_gaps_does_not_converge() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Round 1 with 0 gaps — zero_blocking_converged would be true,
    // but the round guard (round >= 2) prevents convergence.
    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![]), // explicitly empty — no gaps found
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    // Round 1 + 0 gaps: condition 6 doesn't fire (no gaps), auto-converge blocked (round < 2)
    // So status stays "reviewing" with in_progress=true
    assert_eq!(
        resp.status, "reviewing",
        "round 1 + 0 gaps must NOT trigger convergence — round guard requires round >= 2"
    );
    assert!(resp.convergence_reason.is_none(), "no convergence_reason expected for round 1");
}

/// Iterative convergence: gaps clear across rounds → server auto-detects zero_blocking.
///
/// Simulates the real verification loop where the critic finds no blocking gaps
/// after the plan is revised. The server auto-detects convergence when 0 critical AND
/// 0 high AND 0 medium (zero_blocking, AD3) AND round >= 2.
///
/// Flow:
/// - Pre-state: session in Reviewing with metadata showing 1 critical + 2 high from round 1
/// - Round 2: agent sends needs_revision (Reviewing → NeedsRevision), gaps=[] (all cleared)
///   → zero_blocking_converged = (0==0 && 0==0 && 0==0) = true, round=2 >= 2 → Verified
///
/// The server detects convergence automatically without the agent providing convergence_reason.
#[tokio::test]
async fn test_iterative_convergence_decreasing_gaps() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Set up round 1 state: session is Reviewing, metadata.current_gaps has 1 critical + 2 high
    // (simulates what the agent stored after the first critic pass)
    let prior_gaps = vec![
        make_gap("critical", "security", "No authentication layer"),
        make_gap("high", "scalability", "No caching strategy"),
        make_gap("high", "reliability", "No retry mechanism"),
    ];
    let prior_rounds = vec![
        make_round(vec!["no-authentication-layer", "no-caching-strategy", "no-retry-mechanism"], 50),
    ];
    let round1_metadata = make_metadata_json(prior_gaps, prior_rounds, 1, 5);

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Reviewing,
            true,
            Some(round1_metadata),
        )
        .await
        .unwrap();

    // Round 2: agent sends needs_revision with 0 critical + 0 high + 0 medium (all cleared)
    // Server computes: zero_blocking_converged = true, round=2 >= 2 → Verified
    let round2 = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(2),
            gaps: Some(vec![]), // all blocking gaps resolved
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 2 must succeed");

    let resp2 = round2.0;
    assert_eq!(
        resp2.status, "verified",
        "0 critical + 0 high + 0 medium at round 2 → server auto-converges to verified"
    );
    assert_eq!(
        resp2.convergence_reason.as_deref(),
        Some("zero_blocking"),
        "convergence_reason must be 'zero_blocking'"
    );
}

/// Jaccard convergence: 2-pair requirement — only 1 matching pair does not converge.
///
/// The server requires jaccard >= 0.8 for BOTH of:
///   - (new_round, prev_round) pair
///   - (prev_round, prev_prev_round) pair
///
/// This test verifies that the 2-pair requirement is enforced: if only the most recent
/// consecutive pair matches, convergence is not triggered.
///
/// Flow:
/// - Round 1: [gap_a, gap_b] → needs_revision, rounds=[fp1]
/// - Round 2: [gap_c, gap_d] (different) → needs_revision, rounds=[fp1, fp2]
/// - Round 3: [gap_c, gap_d] (same as round 2) → jaccard(fp3,fp2)=1.0 BUT jaccard(fp2,fp1)<1.0
///   → 2-pair requirement not met → needs_revision (no convergence)
#[tokio::test]
async fn test_jaccard_convergence_same_fingerprints() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Shared gap set — same gaps submitted in each round to produce identical fingerprints
    let make_gaps = || {
        vec![
            VerificationGapRequest {
                severity: "high".to_string(),
                category: "scalability".to_string(),
                description: "No horizontal scaling plan".to_string(),
                why_it_matters: None,
                source: None,
            },
            VerificationGapRequest {
                severity: "medium".to_string(),
                category: "documentation".to_string(),
                description: "API docs are incomplete".to_string(),
                why_it_matters: None,
                source: None,
            },
        ]
    };

    // Round 1: Unverified → reviewing + gaps → condition 6 → needs_revision, rounds=[fp1]
    let round1 = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(make_gaps()),
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 1 must succeed");

    assert_eq!(round1.0.status, "needs_revision", "round 1 → needs_revision (condition 6)");

    // Read current metadata (persisted by round 1) so we can reset status to Reviewing
    // while keeping the rounds/fingerprints intact.
    let after_r1 = state.app_state.ideation_session_repo
        .get_by_id(&session_id_obj).await.unwrap().unwrap();
    let r1_metadata = after_r1.verification_metadata.clone();

    // Reset status to Reviewing (keeps round 1 fingerprints in metadata)
    state.app_state.ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Reviewing, true, r1_metadata)
        .await.unwrap();

    // Round 2: Reviewing → needs_revision + different gaps (with critical), round=2
    // Before push: metadata.rounds=[fp1] (len==1) → "need 2 consecutive" → not yet converged.
    // Using critical gaps to prevent zero_blocking from firing (critical_count > 0).
    // Using different gaps from round 1 so jaccard(fp2, fp1) < 1.0.
    let make_gaps_round2 = || {
        vec![
            VerificationGapRequest {
                severity: "critical".to_string(),
                category: "security".to_string(),
                description: "No authentication mechanism specified".to_string(),
                why_it_matters: None,
                source: None,
            },
            VerificationGapRequest {
                severity: "high".to_string(),
                category: "reliability".to_string(),
                description: "No retry mechanism defined".to_string(),
                why_it_matters: None,
                source: None,
            },
        ]
    };

    let round2 = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(2),
            gaps: Some(make_gaps_round2()),
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 2 must succeed");

    // critical_count=1 → zero_blocking=false. Jaccard: len==1 before push → not yet. → needs_revision
    assert_eq!(round2.0.status, "needs_revision", "round 2 → still needs_revision (jaccard needs 2 pairs)");

    // Reset status to Reviewing again (keeps 2-round metadata)
    let after_r2 = state.app_state.ideation_session_repo
        .get_by_id(&session_id_obj).await.unwrap().unwrap();
    let r2_metadata = after_r2.verification_metadata.clone();

    state.app_state.ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Reviewing, true, r2_metadata)
        .await.unwrap();

    // Round 3: same gaps as round 2 → jaccard(fp3=fp2, fp2)=1.0 BUT jaccard(fp2, fp1)<1.0
    // fp1 = fingerprints of [high_scale, medium_docs]
    // fp2 = fingerprints of [crit_auth, high_retry]
    // fp3 = fp2 (same gaps)
    // jaccard(fp3, fp2) = 1.0 ≥ 0.8 ✓ (same gaps)
    // jaccard(fp2, fp1) = 0.0 (completely different descriptions) < 0.8 ✗
    // → 2-pair requirement NOT met → no convergence
    let round3 = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(3),
            gaps: Some(make_gaps_round2()), // same as round 2 → fp3 == fp2
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 3 must succeed");

    // critical_count=1 → zero_blocking=false. Jaccard fires but only 1 of 2 pairs qualifies.
    // → needs_revision (no convergence)
    assert_eq!(
        round3.0.status, "needs_revision",
        "round 3: jaccard(fp3,fp2)=1.0 but jaccard(fp2,fp1)=0.0 → 2-pair requirement not met"
    );
    assert!(
        round3.0.convergence_reason.is_none(),
        "no convergence when only 1 of 2 consecutive pairs is above the Jaccard threshold"
    );
}

/// Jaccard convergence triggered: all 3 consecutive rounds have identical critical gaps.
///
/// When a critic keeps finding the same gaps unchanged for 3 rounds,
/// the server detects that the plan has converged (can't be improved further).
///
/// Uses critical gaps to prevent zero_blocking from triggering first.
/// Status is reset to Reviewing between rounds using direct repo calls
/// (simulating the agent's needs_revision → reviewing → needs_revision cycle).
#[tokio::test]
async fn test_jaccard_convergence_triggered_three_identical_rounds() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Gaps with 1 critical → zero_blocking can never fire (critical_count > 0)
    // Same gaps each round → fingerprints identical → Jaccard = 1.0 for all pairs
    let stable_gaps = || {
        vec![
            VerificationGapRequest {
                severity: "critical".to_string(),
                category: "architecture".to_string(),
                description: "Plan has no rollback strategy".to_string(),
                why_it_matters: None,
                source: None,
            },
            VerificationGapRequest {
                severity: "high".to_string(),
                category: "operations".to_string(),
                description: "No deployment runbook provided".to_string(),
                why_it_matters: None,
                source: None,
            },
        ]
    };

    // Round 1: Unverified → reviewing + critical gaps → condition 6 → needs_revision, rounds=[fp1]
    let round1 = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(stable_gaps()),
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 1 must succeed");

    assert_eq!(round1.0.status, "needs_revision", "round 1 → needs_revision (condition 6)");

    // Reset to Reviewing, preserving round 1's metadata (fingerprints)
    let after_r1 = state.app_state.ideation_session_repo
        .get_by_id(&session_id_obj).await.unwrap().unwrap();
    state.app_state.ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Reviewing, true, after_r1.verification_metadata)
        .await.unwrap();

    // Round 2: Reviewing → needs_revision + same gaps, round=2
    // Before push: rounds=[fp1] (len==1) → "need 2 consecutive" → not yet. After: rounds=[fp1, fp2=fp1]
    // zero_blocking: critical_count=1 > 0 → false. → needs_revision stays.
    let round2 = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(2),
            gaps: Some(stable_gaps()),
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 2 must succeed");

    assert_eq!(round2.0.status, "needs_revision", "round 2 → needs_revision (1 pair is not enough for jaccard)");

    // Reset to Reviewing again, preserving round 1+2 metadata
    let after_r2 = state.app_state.ideation_session_repo
        .get_by_id(&session_id_obj).await.unwrap().unwrap();
    state.app_state.ideation_session_repo
        .update_verification_state(&session_id_obj, VerificationStatus::Reviewing, true, after_r2.verification_metadata)
        .await.unwrap();

    // Round 3: Reviewing → needs_revision + same gaps, round=3
    // Before push: rounds=[fp1, fp2=fp1] (len==2) → jaccard check fires
    // jaccard(new_fp, fp2) = jaccard(fp1, fp1) = 1.0 ≥ 0.8 ✓
    // jaccard(fp2, fp1) = jaccard(fp1, fp1) = 1.0 ≥ 0.8 ✓
    // → jaccard_converged = true → Verified
    let round3 = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(3),
            gaps: Some(stable_gaps()),
            convergence_reason: None,
            max_rounds: Some(10),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 3 must succeed");

    let resp3 = round3.0;
    assert_eq!(
        resp3.status, "verified",
        "3 identical rounds → jaccard_converged (all pairs Jaccard=1.0 ≥ 0.8)"
    );
    assert_eq!(
        resp3.convergence_reason.as_deref(),
        Some("jaccard_converged"),
        "convergence_reason must be 'jaccard_converged'"
    );
}

/// Max rounds exit: reaching max_rounds forces convergence to verified.
///
/// The server auto-terminates the verification loop when `current_round >= max_rounds`.
/// This prevents infinite loops when the plan has stubborn unresolved gaps.
#[tokio::test]
async fn test_max_rounds_exit_behavior() {
    let state = setup_test_state().await;
    let session = IdeationSession::new(ProjectId::new());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Round 3 = max_rounds=3: gaps present but max_rounds fires → verified
    // (condition 3 takes priority over condition 6)
    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(3),
            gaps: Some(vec![VerificationGapRequest {
                severity: "critical".to_string(),
                category: "security".to_string(),
                description: "Unresolved authentication gap".to_string(),
                why_it_matters: Some("Users remain vulnerable".to_string()),
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(3),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(
        resp.status, "verified",
        "current_round >= max_rounds → server forces convergence to verified"
    );
    assert_eq!(
        resp.convergence_reason.as_deref(),
        Some("max_rounds"),
        "convergence_reason must be 'max_rounds'"
    );
}

// ── Re-verification transition tests ──
// Tests covering Verified → Reviewing, Skipped → Reviewing, metadata reset,
// zombie protection after re-verify, and regression for existing transitions.

/// Re-verify: Verified → Reviewing returns 200.
///
/// A plan-verifier agent must be able to restart verification on a session
/// that was previously verified. The Verified → Reviewing transition must
/// be allowed and return 200 with status="reviewing".
#[tokio::test]
async fn test_reverify_verified_to_reviewing_returns_200() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Verified)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "Verified → Reviewing must return 200: {:?}", result.err());
    let resp = result.unwrap().0;
    assert_eq!(resp.status, "reviewing", "status must be reviewing after transition");
    assert!(resp.in_progress, "in_progress must be true");
}

/// Re-verify: Skipped → Reviewing returns 200.
///
/// A user who previously skipped verification must be able to start it.
/// Skipped → Reviewing must be allowed.
#[tokio::test]
async fn test_reverify_skipped_to_reviewing_returns_200() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Skipped)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "Skipped → Reviewing must return 200: {:?}", result.err());
    let resp = result.unwrap().0;
    assert_eq!(resp.status, "reviewing");
    assert!(resp.in_progress);
}

/// Re-verify clears ALL stale metadata and passes condition 6.
///
/// When transitioning from Verified → Reviewing, the handler must:
/// 1. Clear all stale fields: current_gaps, rounds, convergence_reason,
///    best_round_index, current_round, parse_failures.
/// 2. Increment verification_generation (N → N+1).
/// 3. Return response with new generation N+1 (not stale N).
/// 4. Allow a subsequent needs_revision call with generation=N+1.
///
/// The initial reviewing call sends NO gaps, which is safe from condition 6
/// (condition 6 only fires when reviewing + gaps present).
#[tokio::test]
async fn test_reverify_clears_all_stale_metadata() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(3)
        .verification_status(VerificationStatus::Verified)
        .build();
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Set stale metadata from a prior verification run
    let stale_metadata = serde_json::json!({
        "v": 1,
        "current_round": 3,
        "max_rounds": 5,
        "rounds": [
            {"fingerprints": ["fp-auth", "fp-scale"], "gap_score": 12},
            {"fingerprints": ["fp-auth"], "gap_score": 8},
            {"fingerprints": ["fp-auth"], "gap_score": 8}
        ],
        "current_gaps": [
            {"severity": "high", "category": "security", "description": "No auth", "why_it_matters": null}
        ],
        "convergence_reason": "max_rounds",
        "best_round_index": 2,
        "parse_failures": [1]
    })
    .to_string();

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Verified,
            false,
            Some(stale_metadata),
        )
        .await
        .unwrap();

    // Trigger re-verify with no gaps — safe from condition 6
    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(3),
        }),
    )
    .await
    .expect("re-verify from Verified must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "reviewing", "status must be reviewing after re-verify");
    assert!(resp.in_progress, "in_progress must be true");
    assert_eq!(
        resp.verification_generation, 4,
        "response must use new generation N+1=4, not stale N=3"
    );
    assert!(resp.current_gaps.is_empty(), "stale current_gaps must be cleared in response");
    assert!(resp.rounds.is_empty(), "stale rounds must be cleared in response");
    assert!(resp.convergence_reason.is_none(), "stale convergence_reason must be cleared");

    // Verify DB state directly
    let updated = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.verification_status, VerificationStatus::Reviewing);
    assert!(updated.verification_in_progress);
    assert_eq!(updated.verification_generation, 4, "generation must be 4 in DB");

    // Parse metadata JSON to confirm all stale fields were cleared
    let meta: serde_json::Value = serde_json::from_str(
        updated.verification_metadata.as_deref().unwrap_or("{}"),
    )
    .unwrap();
    assert_eq!(meta["current_gaps"], serde_json::json!([]), "current_gaps must be empty");
    assert_eq!(meta["rounds"], serde_json::json!([]), "rounds must be empty");
    assert!(meta["convergence_reason"].is_null(), "convergence_reason must be null");
    assert!(meta["best_round_index"].is_null(), "best_round_index must be null");
    assert_eq!(meta["current_round"], 0, "current_round must be reset to 0");
    assert_eq!(meta["parse_failures"], serde_json::json!([]), "parse_failures must be cleared");

    // Confirm next valid call succeeds with new generation — do NOT call reviewing→reviewing (422)
    let next = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "high".to_string(),
                category: "testing".to_string(),
                description: "New gap found in fresh review".to_string(),
                why_it_matters: None,
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(4), // New generation after reset
        }),
    )
    .await;

    assert!(
        next.is_ok(),
        "needs_revision with new generation=4 must succeed after metadata reset: {:?}",
        next.err()
    );
    let next_resp = next.unwrap().0;
    assert_eq!(next_resp.status, "needs_revision");
    assert_eq!(next_resp.current_gaps.len(), 1, "new gap must be present");
}

/// Skipped → NeedsRevision is rejected (422).
///
/// The only allowed transition from Skipped is Skipped → Reviewing.
#[tokio::test]
async fn test_skipped_to_needs_revision_blocked() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Skipped)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_err(), "Skipped → NeedsRevision must be rejected");
    let (status, _) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "must return 422 for invalid Skipped → NeedsRevision"
    );
}

/// Skipped → Verified is rejected (422).
///
/// The only allowed transition from Skipped is Skipped → Reviewing.
#[tokio::test]
async fn test_skipped_to_verified_blocked() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Skipped)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "verified".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: Some("zero_blocking".to_string()),
            max_rounds: None,
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_err(), "Skipped → Verified must be rejected");
    let (status, _) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "must return 422 for invalid Skipped → Verified"
    );
}

/// Zombie protection after re-verify: stale-generation agent is rejected with 409.
///
/// When reset_and_begin_reverify increments the generation from N to N+1,
/// a stale agent still sending generation=N must receive 409 CONFLICT.
/// The new agent with generation=N+1 must succeed.
#[tokio::test]
async fn test_reverify_zombie_agent_rejected_after_generation_increment() {
    let state = setup_test_state().await;

    // Session at Verified with generation=5
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(5)
        .verification_status(VerificationStatus::Verified)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Re-verify: generation increments from 5 → 6
    let reverify = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(5),
        }),
    )
    .await
    .expect("re-verify must succeed and increment generation to 6");

    assert_eq!(reverify.0.verification_generation, 6, "generation must be 6 after reset");

    // Zombie agent sends needs_revision with old generation=5 → must be rejected with 409
    let zombie = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(5), // Stale — old generation
        }),
    )
    .await;

    assert!(zombie.is_err(), "zombie agent with stale generation=5 must be rejected");
    let (status, _) = zombie.unwrap_err();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "must return 409 CONFLICT for zombie agent after re-verify"
    );

    // Fresh agent with correct generation=6 → must succeed
    let fresh = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(6),
        }),
    )
    .await;

    assert!(fresh.is_ok(), "fresh agent with generation=6 must succeed: {:?}", fresh.err());
}

/// ImportedVerified → Reviewing triggers metadata reset.
///
/// ImportedVerified sessions may carry stale gaps/rounds from the imported state.
/// Transitioning to Reviewing must clear all stale metadata (same as Verified → Reviewing).
#[tokio::test]
async fn test_imported_verified_to_reviewing_triggers_metadata_reset() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(2)
        .verification_status(VerificationStatus::ImportedVerified)
        .build();
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let stale_metadata = serde_json::json!({
        "v": 1,
        "current_round": 2,
        "max_rounds": 3,
        "rounds": [{"fingerprints": ["fp-imported"], "gap_score": 5}],
        "current_gaps": [
            {"severity": "medium", "category": "docs", "description": "Missing docs", "why_it_matters": null}
        ],
        "convergence_reason": "zero_blocking",
        "best_round_index": 0,
        "parse_failures": []
    })
    .to_string();

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::ImportedVerified,
            false,
            Some(stale_metadata),
        )
        .await
        .unwrap();

    let result = update_plan_verification(
        State(state.clone()),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(2),
        }),
    )
    .await
    .expect("ImportedVerified → Reviewing must succeed");

    let resp = result.0;
    assert_eq!(resp.status, "reviewing");
    assert!(resp.in_progress);
    assert_eq!(resp.verification_generation, 3, "generation must be incremented to 2+1=3");
    assert!(resp.current_gaps.is_empty(), "imported stale gaps must be cleared");
    assert!(resp.rounds.is_empty(), "imported stale rounds must be cleared");
    assert!(resp.convergence_reason.is_none(), "imported convergence_reason must be cleared");

    let updated = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.verification_generation, 3);

    let meta: serde_json::Value = serde_json::from_str(
        updated.verification_metadata.as_deref().unwrap_or("{}"),
    )
    .unwrap();
    assert_eq!(meta["current_gaps"], serde_json::json!([]));
    assert_eq!(meta["rounds"], serde_json::json!([]));
    assert!(meta["convergence_reason"].is_null());
    assert_eq!(meta["current_round"], 0);
}

/// Regression: Verified → Skipped still allowed after new re-verify arms.
///
/// The new Verified → Reviewing match arm must not shadow the existing
/// catch-all that allows any status → Skipped.
#[tokio::test]
async fn test_verified_to_skipped_still_allowed() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Verified)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "skipped".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "Verified → Skipped must still be allowed: {:?}", result.err());
    let resp = result.unwrap().0;
    assert_eq!(resp.status, "skipped");
}

/// Full re-verify flow: reach Verified, re-verify, new gaps replace old gaps.
///
/// End-to-end test simulating the plan-verifier agent's second run:
/// 1. Session is at Verified with stale gaps from prior run.
/// 2. Re-verify transition clears metadata and increments generation.
/// 3. First round of new verification (needs_revision with new gaps) succeeds.
/// 4. New gaps are present in the response; old gaps are gone.
#[tokio::test]
async fn test_full_reverify_flow_new_gaps_replace_old() {
    let state = setup_test_state().await;

    // Session at Verified with generation=1 and stale gaps
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_generation(1)
        .verification_status(VerificationStatus::Verified)
        .build();
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let old_metadata = serde_json::json!({
        "v": 1,
        "current_round": 1,
        "max_rounds": 1,
        "rounds": [{"fingerprints": ["old-gap-fp"], "gap_score": 5}],
        "current_gaps": [
            {"severity": "high", "category": "old", "description": "Old outdated gap", "why_it_matters": null}
        ],
        "convergence_reason": "max_rounds",
        "best_round_index": 0,
        "parse_failures": []
    })
    .to_string();

    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Verified,
            false,
            Some(old_metadata),
        )
        .await
        .unwrap();

    // Re-verify: Verified → Reviewing (clears metadata, increments gen 1 → 2)
    let reverify = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(1),
        }),
    )
    .await
    .expect("re-verify must succeed");

    assert_eq!(reverify.0.verification_generation, 2);
    assert!(reverify.0.current_gaps.is_empty(), "old gaps must be cleared after re-verify");

    // Round 1 with fresh gaps using new generation=2
    let round1 = update_plan_verification(
        State(state.clone()),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(1),
            gaps: Some(vec![VerificationGapRequest {
                severity: "critical".to_string(),
                category: "security".to_string(),
                description: "Completely new security gap found in fresh review".to_string(),
                why_it_matters: Some("Fresh analysis found new vulnerabilities".to_string()),
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: Some(2),
        }),
    )
    .await
    .expect("round 1 with new generation must succeed");

    let r1 = round1.0;
    assert_eq!(r1.status, "needs_revision", "critical gap → needs_revision");
    assert_eq!(r1.current_gaps.len(), 1, "exactly 1 new gap");
    assert_eq!(
        r1.current_gaps[0].description,
        "Completely new security gap found in fresh review",
        "new gap description must be present"
    );
    assert_eq!(r1.current_gaps[0].severity, "critical");

    // DB: new gaps present, old gaps gone
    let final_session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .unwrap()
        .unwrap();
    let meta: serde_json::Value = serde_json::from_str(
        final_session.verification_metadata.as_deref().unwrap_or("{}"),
    )
    .unwrap();
    assert_eq!(
        meta["current_gaps"][0]["description"],
        "Completely new security gap found in fresh review",
        "DB must contain new gap"
    );
    assert_ne!(
        meta["current_gaps"][0]["description"].as_str().unwrap_or(""),
        "Old outdated gap",
        "old gap must not be present in DB"
    );
}

// ============================================================================
// get_child_session_status_handler tests
// ============================================================================

/// Helper: spawn a `cat` process to get a live ChildStdin for IPR registration.
/// Caller is responsible for killing the child after the test.
async fn spawn_test_stdin_ideation() -> (
    tokio::process::Child,
    tokio::process::ChildStdin,
    tokio::process::ChildStdout,
) {
    let mut child = tokio::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn cat for ideation IPR test");
    let stdin = child.stdin.take().expect("cat stdin handle");
    let stdout = child.stdout.take().expect("cat stdout handle");
    (child, stdin, stdout)
}

/// Helper: default no-op params for get_child_session_status_handler.
fn no_messages_params() -> ChildSessionStatusParams {
    ChildSessionStatusParams {
        include_messages: None,
        message_limit: None,
    }
}

/// Helper: create and persist an Active ideation session.
async fn create_active_session(state: &HttpServerState) -> IdeationSessionId {
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .build();
    let id = session.id.clone();
    state.app_state.ideation_session_repo.create(session).await.unwrap();
    id
}

/// Test 1: agent in registry with heartbeat < threshold → estimated_status = "likely_generating"
#[tokio::test]
async fn test_get_child_session_status_likely_generating() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Register agent under "session" key with a very recent heartbeat (well within 10s threshold)
    let key = RunningAgentKey::new("session", &sid_str);
    state
        .app_state
        .running_agent_registry
        .register(key.clone(), 99999, "test-conv".to_string(), "test-run".to_string(), None, None)
        .await;
    state
        .app_state
        .running_agent_registry
        .update_heartbeat(&key, chrono::Utc::now())
        .await;

    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    assert!(resp.agent_state.is_running, "agent must be running");
    assert_eq!(
        resp.agent_state.estimated_status, "likely_generating",
        "recent heartbeat must yield likely_generating"
    );
}

/// Test 2: agent in registry with heartbeat > threshold → estimated_status = "likely_waiting"
#[tokio::test]
async fn test_get_child_session_status_likely_waiting() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Register agent under "ideation" key (tests dual-key lookup) with stale heartbeat
    let key = RunningAgentKey::new("ideation", &sid_str);
    state
        .app_state
        .running_agent_registry
        .register(key.clone(), 99998, "test-conv-2".to_string(), "test-run-2".to_string(), None, None)
        .await;
    // Heartbeat 1000s ago — well beyond the 10s default threshold
    let stale = chrono::Utc::now() - chrono::Duration::seconds(1000);
    state
        .app_state
        .running_agent_registry
        .update_heartbeat(&key, stale)
        .await;

    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    assert!(resp.agent_state.is_running, "agent must be running");
    assert_eq!(
        resp.agent_state.estimated_status, "likely_waiting",
        "stale heartbeat (1000s) must yield likely_waiting"
    );
}

/// Test 3: agent not in registry → estimated_status = "idle"
#[tokio::test]
async fn test_get_child_session_status_idle() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // No agent registered — registry is empty
    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    assert!(!resp.agent_state.is_running, "agent must not be running");
    assert_eq!(resp.agent_state.estimated_status, "idle");
    assert!(resp.agent_state.pid.is_none());
    assert!(resp.agent_state.last_active_at.is_none());
}

/// Test 4: include_messages=true returns messages (truncated to 500 chars)
#[tokio::test]
async fn test_get_child_session_status_include_messages_truncated() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Create a message with content longer than 500 chars — must be truncated
    let long_content = "A".repeat(700);
    let msg = ChatMessage::user_in_session(session_id.clone(), long_content.clone());
    state.app_state.chat_message_repo.create(msg).await.unwrap();

    let params = ChildSessionStatusParams {
        include_messages: Some(true),
        message_limit: Some(5),
    };

    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(params),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    let messages = resp.recent_messages.expect("messages must be returned");
    assert_eq!(messages.len(), 1, "one message created");
    assert_eq!(
        messages[0].content.chars().count(),
        500,
        "content must be truncated to 500 chars"
    );
    assert_eq!(messages[0].role, "user");
}

/// Test 5: session not found → 404
#[tokio::test]
async fn test_get_child_session_status_not_found_returns_404() {
    let state = setup_test_state().await;

    let result = get_child_session_status_handler(
        State(state),
        Path("non-existent-session-id".to_string()),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_err(), "expected Err for missing session");
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, StatusCode::NOT_FOUND, "must return 404 for missing session");
}

/// Test 6: message_limit=10000 clamped to 50 (max enforcement)
///
/// We create 60 messages and request 10000 — only 50 should be returned.
#[tokio::test]
async fn test_get_child_session_status_message_limit_clamped_to_50() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Create 60 messages
    for i in 0..60 {
        let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
        state.app_state.chat_message_repo.create(msg).await.unwrap();
    }

    let params = ChildSessionStatusParams {
        include_messages: Some(true),
        message_limit: Some(10000),
    };

    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(params),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let messages = result.unwrap().0.recent_messages.expect("messages must be returned");
    assert!(
        messages.len() <= 50,
        "message_limit=10000 must be clamped to 50, got {}",
        messages.len()
    );
}

/// Test 7 (boundary): heartbeat exactly at threshold → "likely_waiting"
///
/// The handler uses `elapsed < threshold_secs` for "likely_generating".
/// At elapsed == threshold_secs, the condition is false → "likely_waiting".
/// Default threshold is 10 seconds.
#[tokio::test]
async fn test_get_child_session_status_heartbeat_at_exact_threshold_is_likely_waiting() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    let key = RunningAgentKey::new("session", &sid_str);
    state
        .app_state
        .running_agent_registry
        .register(key.clone(), 99997, "test-conv-3".to_string(), "test-run-3".to_string(), None, None)
        .await;

    // Default threshold = 10 seconds. Set heartbeat to exactly 10s ago.
    // elapsed == threshold → condition (elapsed < threshold) is false → "likely_waiting"
    let default_threshold_secs: i64 = 10;
    let at_boundary = chrono::Utc::now() - chrono::Duration::seconds(default_threshold_secs);
    state
        .app_state
        .running_agent_registry
        .update_heartbeat(&key, at_boundary)
        .await;

    let result = get_child_session_status_handler(
        State(state),
        Path(sid_str),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    assert_eq!(
        resp.agent_state.estimated_status, "likely_waiting",
        "heartbeat at exact threshold boundary must yield likely_waiting (elapsed >= threshold)"
    );
}

/// Test 8: valid verification_metadata → populated VerificationInfo with gap_score and current_round
#[tokio::test]
async fn test_get_child_session_status_valid_verification_metadata_populated() {
    let state = setup_test_state().await;

    let metadata_json = serde_json::json!({
        "v": 1,
        "current_round": 2,
        "max_rounds": 5,
        "rounds": [
            {"fingerprints": ["fp-1"], "gap_score": 7},
            {"fingerprints": ["fp-2"], "gap_score": 3}
        ],
        "current_gaps": [],
        "convergence_reason": null,
        "best_round_index": null,
        "parse_failures": []
    })
    .to_string();

    let mut session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Reviewing)
        .verification_generation(2)
        .build();
    session.verification_metadata = Some(metadata_json);
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = get_child_session_status_handler(
        State(state),
        Path(session_id),
        Query(no_messages_params()),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    let resp = result.unwrap().0;
    let verification = resp.verification.expect("verification must be populated for non-Unverified status");
    assert_eq!(verification.status, "reviewing");
    assert_eq!(verification.generation, 2);
    assert_eq!(
        verification.current_round,
        Some(2),
        "current_round=2 from metadata"
    );
    assert_eq!(
        verification.gap_score,
        Some(3),
        "gap_score must come from last round (index 1, score=3)"
    );
}

/// Test 9: malformed verification_metadata JSON → verification: None (no panic)
#[tokio::test]
async fn test_get_child_session_status_malformed_metadata_returns_none() {
    let state = setup_test_state().await;

    let mut session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .verification_status(VerificationStatus::Reviewing)
        .build();
    // Inject invalid JSON — deserialization must fail gracefully
    session.verification_metadata = Some("not-valid-json{{{".to_string());
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = get_child_session_status_handler(
        State(state),
        Path(session_id),
        Query(no_messages_params()),
    )
    .await;

    // Handler must return Ok (no 500), with verification.gap_score = None and current_round = None
    assert!(result.is_ok(), "malformed metadata must not cause 500: {:?}", result.err());
    let resp = result.unwrap().0;
    // VerificationInfo is present (status is Reviewing, not Unverified), but gap_score/current_round are None
    let verification = resp.verification.expect("VerificationInfo present for non-Unverified status");
    assert_eq!(verification.status, "reviewing");
    assert!(
        verification.gap_score.is_none(),
        "malformed metadata → gap_score must be None"
    );
    assert!(
        verification.current_round.is_none(),
        "malformed metadata → current_round must be None"
    );
}

// ============================================================================
// send_ideation_session_message_handler tests
// ============================================================================

/// Test 10: interactive process under "session" key → delivery_status = "sent"
///
/// Validates the dual-key IPR check: agents spawned via HTTP use "session" key.
#[tokio::test]
async fn test_send_ideation_session_message_interactive_session_key_sent() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();
    let message = "Hello agent";

    // Register a live stdin under "session" key
    let (mut child, stdin, stdout) = spawn_test_stdin_ideation().await;
    let ipr_key = InteractiveProcessKey::new("session", &sid_str);
    state
        .app_state
        .interactive_process_registry
        .register(ipr_key, stdin)
        .await;

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str.clone()),
        Json(SendSessionMessageRequest {
            message: message.to_string(),
        }),
    )
    .await;

    let mut written = String::new();
    let mut reader = BufReader::new(stdout);
    reader.read_line(&mut written).await.expect("read cat stdout");
    let _ = child.kill().await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    assert_eq!(result.unwrap().0.delivery_status, "sent");
    let payload: serde_json::Value = serde_json::from_str(written.trim_end()).expect("valid JSON");
    assert_eq!(payload["type"], "user");
    assert_eq!(payload["message"]["role"], "user");
    let content = payload["message"]["content"].as_str().expect("content string");
    assert!(
        content.contains(&format!("<context_id>{sid_str}</context_id>")),
        "content must include ideation context wrapper: {content}"
    );
    assert!(
        content.contains(&format!("<user_message>{message}</user_message>")),
        "content must include wrapped user message: {content}"
    );
}

/// Test 11: interactive process under "ideation" key → delivery_status = "sent"
///
/// Validates the dual-key IPR check: agents spawned via Tauri IPC use "ideation" key.
#[tokio::test]
async fn test_send_ideation_session_message_interactive_ideation_key_sent() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();
    let message = "Nudge from orchestrator";

    // Register a live stdin under "ideation" key (Tauri IPC spawn path)
    let (mut child, stdin, stdout) = spawn_test_stdin_ideation().await;
    let ipr_key = InteractiveProcessKey::new("ideation", &sid_str);
    state
        .app_state
        .interactive_process_registry
        .register(ipr_key, stdin)
        .await;

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str.clone()),
        Json(SendSessionMessageRequest {
            message: message.to_string(),
        }),
    )
    .await;

    let mut written = String::new();
    let mut reader = BufReader::new(stdout);
    reader.read_line(&mut written).await.expect("read cat stdout");
    let _ = child.kill().await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    assert_eq!(result.unwrap().0.delivery_status, "sent");
    let payload: serde_json::Value = serde_json::from_str(written.trim_end()).expect("valid JSON");
    assert_eq!(payload["type"], "user");
    assert_eq!(payload["message"]["role"], "user");
    let content = payload["message"]["content"].as_str().expect("content string");
    assert!(
        content.contains(&format!("<context_id>{sid_str}</context_id>")),
        "content must include ideation context wrapper: {content}"
    );
    assert!(
        content.contains(&format!("<user_message>{message}</user_message>")),
        "content must include wrapped user message: {content}"
    );
}

/// Test 12: running agent under "session" RunningAgentKey (no interactive process) → "queued"
///
/// Agent is in the running registry but has no stdin → message is queued.
#[tokio::test]
async fn test_send_ideation_session_message_running_session_key_queued() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Register agent as running (no IPR entry — stdin not registered)
    let agent_key = RunningAgentKey::new("session", &sid_str);
    state
        .app_state
        .running_agent_registry
        .register(agent_key, 88888, "test-conv-q".to_string(), "test-run-q".to_string(), None, None)
        .await;

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str),
        Json(SendSessionMessageRequest {
            message: "Queue this message".to_string(),
        }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    assert_eq!(
        result.unwrap().0.delivery_status,
        "queued",
        "running agent without IPR → message must be queued"
    );
}

/// Test 13: running agent under "ideation" RunningAgentKey (no interactive process) → "queued"
///
/// Validates dual-key check on the running_agent_registry: Tauri IPC agents use "ideation" key.
#[tokio::test]
async fn test_send_ideation_session_message_running_ideation_key_queued() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Register agent as running under "ideation" key (no "session" key, no IPR)
    let agent_key = RunningAgentKey::new("ideation", &sid_str);
    state
        .app_state
        .running_agent_registry
        .register(agent_key, 77777, "test-conv-iq".to_string(), "test-run-iq".to_string(), None, None)
        .await;

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str),
        Json(SendSessionMessageRequest {
            message: "Queue via ideation key".to_string(),
        }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok: {:?}", result.err());
    assert_eq!(
        result.unwrap().0.delivery_status,
        "queued",
        "running agent under ideation key without IPR → message must be queued"
    );
}

/// Test 14: agent idle → spawn path entered; in test env Claude CLI absent → 500
///
/// Verifies: (a) not "sent" (no IPR), (b) not "queued" (not in registry),
/// (c) handler reaches spawn path, which fails with 500 when Claude CLI unavailable.
/// `with_team_mode()` is called before `send_message()` in this path.
/// In production (with Claude CLI present) this would return delivery_status = "spawned".
#[tokio::test]
async fn test_send_ideation_session_message_agent_idle_spawn_path_entered() {
    let state = setup_test_state().await;

    // Team-mode session: verifies that session_is_team_mode() is evaluated for spawn path
    let mut session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .team_mode("research")
        .build();
    session.status = ralphx_lib::domain::entities::ideation::IdeationSessionStatus::Active;
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // No IPR, no running agent → spawn path
    let result = send_ideation_session_message_handler(
        State(state),
        Path(session_id),
        Json(SendSessionMessageRequest {
            message: "Spawn me an agent".to_string(),
        }),
    )
    .await;

    // This proves the handler entered the spawn path (not "sent"/"queued").
    // In environments without Claude CLI: spawn fails → Err(500)
    // In environments with Claude CLI in PATH: spawn succeeds → Ok("spawned")
    // Both outcomes prove the spawn path was entered.
    match result {
        Ok(Json(resp)) => assert_eq!(
            resp.delivery_status, "spawned",
            "agent idle → spawn path entered → delivery_status must be 'spawned'"
        ),
        Err((status, _)) => assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "agent idle → spawn failure must return 500"
        ),
    }
}

/// Test 15: session with Archived status → 422
#[tokio::test]
async fn test_send_ideation_session_message_archived_session_returns_422() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .status(ralphx_lib::domain::entities::ideation::IdeationSessionStatus::Archived)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = send_ideation_session_message_handler(
        State(state),
        Path(session_id),
        Json(SendSessionMessageRequest { message: "Hello".to_string() }),
    )
    .await;

    assert!(result.is_err(), "Archived session must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "Archived session → 422"
    );
}

/// Test 16: session with Accepted status → 422
#[tokio::test]
async fn test_send_ideation_session_message_accepted_session_returns_422() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .status(ralphx_lib::domain::entities::ideation::IdeationSessionStatus::Accepted)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    let result = send_ideation_session_message_handler(
        State(state),
        Path(session_id),
        Json(SendSessionMessageRequest { message: "Hello".to_string() }),
    )
    .await;

    assert!(result.is_err(), "Accepted session must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "Accepted session → 422"
    );
}

/// Test 17: empty message → 422
#[tokio::test]
async fn test_send_ideation_session_message_empty_message_returns_422() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str),
        Json(SendSessionMessageRequest { message: String::new() }),
    )
    .await;

    assert!(result.is_err(), "empty message must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "empty message → 422");
}

/// Test 18: message longer than 10_000 chars → 422
#[tokio::test]
async fn test_send_ideation_session_message_too_long_returns_422() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    let huge_message = "X".repeat(10_001);

    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str),
        Json(SendSessionMessageRequest { message: huge_message }),
    )
    .await;

    assert!(result.is_err(), "message >10000 chars must be rejected");
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "too-long message → 422");
}

/// Test 19: send_message() returns Err → 500 (not false "spawned" positive)
///
/// This is the same condition as the idle spawn test in test env (Claude CLI absent).
/// Validates the error arm of the send_message match: `Err(e) → log + return 500`.
/// Separate from test 14 to document the explicit error-handling contract.
#[tokio::test]
async fn test_send_ideation_session_message_send_error_returns_500() {
    let state = setup_test_state().await;
    let session_id = create_active_session(&state).await;
    let sid_str = session_id.as_str().to_string();

    // Agent is idle, no IPR → reaches send_message() → spawn path entered.
    // In environments without Claude CLI: SpawnFailed → Err(500)
    // In environments with Claude CLI in PATH: spawn succeeds → Ok("spawned")
    // Both outcomes prove errors are propagated correctly (no silent swallowing).
    let result = send_ideation_session_message_handler(
        State(state),
        Path(sid_str),
        Json(SendSessionMessageRequest {
            message: "Trigger spawn failure".to_string(),
        }),
    )
    .await;

    match result {
        Ok(Json(resp)) => assert_eq!(
            resp.delivery_status, "spawned",
            "send_message Ok → must be 'spawned' (Claude CLI found)"
        ),
        Err((status, _)) => assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "send_message Err → 500 (not 'spawned' false positive)"
        ),
    }
}

// ============================================================================
// External origin guard tests
// ============================================================================

/// External session + status=skipped → update_plan_verification must return 403.
///
/// Proof Obligation 1: external agent attempts to skip verification via the internal HTTP handler.
/// The guard at the top of update_plan_verification checks session.origin == External before
/// processing any transition logic.
#[tokio::test]
async fn test_update_verification_rejects_skip_for_external_origin() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .origin(SessionOrigin::External)
        .build();
    let session_id = session.id.as_str().to_string();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let result = update_plan_verification(
        State(state),
        Path(session_id),
        Json(UpdateVerificationRequest {
            status: "skipped".to_string(),
            in_progress: false,
            round: None,
            gaps: None,
            convergence_reason: None,
            max_rounds: None,
            parse_failed: None,
            generation: None,
        }),
    )
    .await;

    assert!(result.is_err(), "external session must reject skip status");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "external skip must return 403 FORBIDDEN"
    );
}

/// External session + revert_and_skip → must return 403.
///
/// Proof Obligation 2: external agent attempts to use the revert_and_skip endpoint.
/// The origin guard fires before any artifact lookup, so no artifact needs to exist.
#[tokio::test]
async fn test_revert_and_skip_blocks_external_origin() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .origin(SessionOrigin::External)
        .build();
    let session_id = session.id.as_str().to_string();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let result = revert_and_skip(
        State(state),
        Path(session_id),
        Json(RevertAndSkipRequest {
            plan_version_to_restore: "non-existent-artifact-id".to_string(),
        }),
    )
    .await;

    assert!(result.is_err(), "external session must reject revert_and_skip");
    let (status, _body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "external revert_and_skip must return 403 FORBIDDEN"
    );
}

// ============================================================================
// Auto-propose integration tests (PDM-24)
// ============================================================================

/// Auto-propose fires for external sessions that reach zero_blocking convergence.
///
/// Proof Obligation 3 (partial): session.origin == External && convergence_reason == zero_blocking
/// → auto_propose_for_external() sends message to the orchestrator agent.
///
/// Flow:
/// - Create external session, pre-register running agent (Gate 2 → queues message)
/// - Round 1: Reviewing + gaps stored in metadata
/// - Round 2: needs_revision + 0 gaps → server auto-converges (zero_blocking)
/// - Assert: message_queue contains <auto-propose> for the session's ideation context
#[tokio::test]
async fn test_auto_propose_fires_for_external_zero_blocking() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .origin(SessionOrigin::External)
        .build();
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Pre-register a running agent under "ideation" key so send_message queues instead of spawning.
    // auto_propose_for_external calls send_message(ChatContextType::Ideation, session_id, ...)
    // which uses RunningAgentKey::new("ideation", session_id) for Gate 2.
    let agent_key = RunningAgentKey::new("ideation", &session_id);
    state
        .app_state
        .running_agent_registry
        .register(
            agent_key,
            12345,
            "test-conv-ap".to_string(),
            "test-run-ap".to_string(),
            None,
            None,
        )
        .await;

    // Clone the message_queue Arc before state is moved into the handler
    let message_queue = Arc::clone(&state.app_state.message_queue);

    // Set up round 1 state: session in Reviewing with 2 blocking gaps
    let prior_gaps = vec![
        make_gap("high", "security", "No authentication layer"),
        make_gap("medium", "testing", "No unit tests"),
    ];
    let prior_rounds = vec![make_round(
        vec!["no-authentication-layer", "no-unit-tests"],
        30,
    )];
    let round1_metadata = make_metadata_json(prior_gaps, prior_rounds, 1, 5);
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Reviewing,
            true,
            Some(round1_metadata),
        )
        .await
        .unwrap();

    // Round 2: 0 gaps → server auto-detects zero_blocking, overrides to Verified,
    // then calls auto_propose_for_external (external + zero_blocking guard passes).
    let result = update_plan_verification(
        State(state),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(2),
            gaps: Some(vec![]), // all blocking gaps cleared
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 2 must succeed");

    let resp = result.0;
    assert_eq!(
        resp.status, "verified",
        "0 gaps at round 2 → server auto-converges to verified"
    );
    assert_eq!(
        resp.convergence_reason.as_deref(),
        Some("zero_blocking"),
        "convergence_reason must be 'zero_blocking'"
    );

    // Assert auto-propose was queued for this session's ideation context
    let queued = message_queue.get_queued(ChatContextType::Ideation, &session_id);
    assert!(
        !queued.is_empty(),
        "auto-propose message must be queued for external zero_blocking session"
    );
    assert!(
        queued.iter().any(|m| m.content.contains("<auto-propose>")),
        "queued message must contain <auto-propose> tag; got: {:?}",
        queued.iter().map(|m| &m.content).collect::<Vec<_>>()
    );
}

/// Auto-propose is skipped for internal (non-external) sessions even if zero_blocking fires.
///
/// Proof Obligation 3: internal sessions are NOT affected by auto-propose.
/// The guard inside auto_propose_for_external checks session.origin == External and returns
/// early for Internal sessions. The call site also has the origin check, so auto_propose
/// is never invoked for internal sessions.
///
/// Flow: same verification round sequence as above, but session origin = Internal (default).
/// Assert: message_queue is empty after convergence.
#[tokio::test]
async fn test_auto_propose_skipped_for_internal_session() {
    let state = setup_test_state().await;

    // Default origin = Internal (no .origin() call needed)
    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .build();
    let session_id_obj = session.id.clone();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Clone message_queue before state is moved
    let message_queue = Arc::clone(&state.app_state.message_queue);

    // Round 1: Reviewing + gaps
    let prior_gaps = vec![
        make_gap("high", "security", "No authentication layer"),
        make_gap("medium", "testing", "No unit tests"),
    ];
    let prior_rounds = vec![make_round(
        vec!["no-authentication-layer", "no-unit-tests"],
        30,
    )];
    let round1_metadata = make_metadata_json(prior_gaps, prior_rounds, 1, 5);
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Reviewing,
            true,
            Some(round1_metadata),
        )
        .await
        .unwrap();

    // Round 2: zero_blocking convergence on internal session
    let result = update_plan_verification(
        State(state),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "needs_revision".to_string(),
            in_progress: true,
            round: Some(2),
            gaps: Some(vec![]),
            convergence_reason: None,
            max_rounds: Some(5),
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("round 2 must succeed");

    let resp = result.0;
    assert_eq!(
        resp.status, "verified",
        "internal session must still converge to verified on zero_blocking"
    );
    assert_eq!(
        resp.convergence_reason.as_deref(),
        Some("zero_blocking"),
        "convergence_reason must be 'zero_blocking'"
    );

    // No auto-propose message should be queued for internal sessions
    let queued = message_queue.get_queued(ChatContextType::Ideation, &session_id);
    assert!(
        queued.is_empty(),
        "no auto-propose must be queued for internal sessions; got: {:?}",
        queued.iter().map(|m| &m.content).collect::<Vec<_>>()
    );
}

/// Auto-propose is skipped when convergence reason is max_rounds (not zero_blocking).
///
/// Proof Obligation 4: only zero_blocking triggers auto-propose.
/// max_rounds and jaccard_converged may have unresolved gaps, so they are excluded.
/// The call site guard checks convergence_reason == Some("zero_blocking") explicitly.
///
/// Flow: external session reaches Verified via max_rounds (gaps still present).
/// Assert: message_queue is empty after convergence.
#[tokio::test]
async fn test_auto_propose_skipped_for_non_zero_blocking() {
    let state = setup_test_state().await;

    let session = IdeationSessionBuilder::new()
        .project_id(ProjectId::new())
        .origin(SessionOrigin::External)
        .build();
    let session_id = session.id.as_str().to_string();
    state.app_state.ideation_session_repo.create(session).await.unwrap();

    // Clone message_queue before state is moved
    let message_queue = Arc::clone(&state.app_state.message_queue);

    // Round 3 = max_rounds=3: critical gap still present → server forces convergence via max_rounds
    // (not zero_blocking). auto_propose call site guard fails on convergence_reason check.
    let result = update_plan_verification(
        State(state),
        Path(session_id.clone()),
        Json(UpdateVerificationRequest {
            status: "reviewing".to_string(),
            in_progress: true,
            round: Some(3),
            gaps: Some(vec![VerificationGapRequest {
                severity: "critical".to_string(),
                category: "security".to_string(),
                description: "Unresolved authentication gap".to_string(),
                why_it_matters: Some("Users remain vulnerable".to_string()),
                source: None,
            }]),
            convergence_reason: None,
            max_rounds: Some(3), // round == max_rounds → max_rounds convergence
            parse_failed: None,
            generation: None,
        }),
    )
    .await
    .expect("handler must succeed");

    let resp = result.0;
    assert_eq!(
        resp.status, "verified",
        "current_round >= max_rounds → verified via max_rounds"
    );
    assert_eq!(
        resp.convergence_reason.as_deref(),
        Some("max_rounds"),
        "convergence_reason must be 'max_rounds'"
    );

    // No auto-propose for max_rounds convergence even on external sessions
    let queued = message_queue.get_queued(ChatContextType::Ideation, &session_id);
    assert!(
        queued.is_empty(),
        "no auto-propose must be queued for max_rounds convergence; got: {:?}",
        queued.iter().map(|m| &m.content).collect::<Vec<_>>()
    );
}
