use super::*;
use crate::infrastructure::memory::MemoryTeamMessageRepository;

fn test_service() -> TeamService {
    TeamService::new_without_events(Arc::new(TeamStateTracker::new()))
}

#[tokio::test]
async fn test_create_team() {
    let svc = test_service();
    svc.create_team("alpha", "session-1", "ideation")
        .await
        .unwrap();

    assert!(svc.team_exists("alpha").await);
}

#[tokio::test]
async fn test_create_duplicate_team_fails() {
    let svc = test_service();
    svc.create_team("alpha", "s-1", "ideation").await.unwrap();

    let err = svc.create_team("alpha", "s-2", "ideation").await;
    assert!(matches!(err, Err(TeamTrackerError::TeamAlreadyExists(_))));
}

#[tokio::test]
async fn test_add_teammate_and_status() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "researcher", "#ff6b35", "opus", "explore")
        .await
        .unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(status.teammates.len(), 1);
    assert_eq!(status.teammates[0].name, "researcher");
    assert_eq!(status.teammates[0].status, TeammateStatus::Spawning);
    assert_eq!(status.context_id, "ctx-1");
    assert_eq!(status.context_type, "ideation");
}

#[tokio::test]
async fn test_update_teammate_status() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "worker", "#00ff00", "sonnet", "code")
        .await
        .unwrap();

    svc.update_teammate_status("t1", "worker", TeammateStatus::Running)
        .await
        .unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(status.teammates[0].status, TeammateStatus::Running);
}

#[tokio::test]
async fn test_update_teammate_cost() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "r", "#ff6b35", "opus", "explore")
        .await
        .unwrap();

    let cost = TeammateCost {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_tokens: 200,
        cache_read_tokens: 100,
        estimated_usd: 0.05,
    };
    svc.update_teammate_cost("t1", "r", cost).await.unwrap();

    let resp = svc.get_teammate_cost("t1", "r").await.unwrap();
    assert_eq!(resp.input_tokens, 1000);
    assert_eq!(resp.estimated_usd, 0.05);
}

#[tokio::test]
async fn test_send_user_message() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

    let msg = svc.send_user_message("t1", "Hello").await.unwrap();
    assert_eq!(msg.sender, "user");
    assert_eq!(msg.content, "Hello");
}

#[tokio::test]
async fn test_add_teammate_message() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

    let msg = svc
        .add_teammate_message(
            "t1",
            "researcher",
            Some("planner"),
            "Found results",
            TeamMessageType::TeammateMessage,
        )
        .await
        .unwrap();
    assert_eq!(msg.sender, "researcher");
    assert_eq!(msg.recipient, Some("planner".to_string()));
}

#[tokio::test]
async fn test_stop_teammate() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
        .await
        .unwrap();

    svc.stop_teammate("t1", "w").await.unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(status.teammates[0].status, TeammateStatus::Shutdown);
}

#[tokio::test]
async fn test_stop_team() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "w1", "#ff0000", "sonnet", "code")
        .await
        .unwrap();
    svc.add_teammate("t1", "w2", "#00ff00", "sonnet", "code")
        .await
        .unwrap();

    svc.stop_team("t1").await.unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(
        status.phase,
        super::super::team_state_tracker::TeamPhase::Winding
    );
    for t in &status.teammates {
        assert_eq!(t.status, TeammateStatus::Shutdown);
    }
}

#[tokio::test]
async fn test_disband_team() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
        .await
        .unwrap();

    svc.disband_team("t1").await.unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(
        status.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded
    );
}

#[tokio::test]
async fn test_get_messages_with_limit() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();

    for i in 0..5 {
        svc.send_user_message("t1", &format!("Msg {}", i))
            .await
            .unwrap();
    }

    let all = svc.get_team_messages("t1", None).await.unwrap();
    assert_eq!(all.len(), 5);

    let limited = svc.get_team_messages("t1", Some(2)).await.unwrap();
    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn test_list_teams() {
    let svc = test_service();
    svc.create_team("a", "ctx-1", "ideation").await.unwrap();
    svc.create_team("b", "ctx-2", "task").await.unwrap();

    let teams = svc.list_teams().await;
    assert_eq!(teams.len(), 2);
}

#[tokio::test]
async fn test_remove_teammate() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "w", "#ff0000", "sonnet", "code")
        .await
        .unwrap();

    svc.remove_teammate("t1", "w").await.unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(status.teammates.len(), 0);
}

#[tokio::test]
async fn test_teammate_count() {
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "a", "#ff0000", "sonnet", "code")
        .await
        .unwrap();
    svc.add_teammate("t1", "b", "#00ff00", "opus", "explore")
        .await
        .unwrap();

    assert_eq!(svc.get_teammate_count("t1").await.unwrap(), 2);
}

#[tokio::test]
async fn test_cleanup_stale_teams_for_context() {
    let svc = test_service();
    // Create two teams: one for the target context, one for a different context
    svc.create_team("team-a", "ctx-target", "task_execution")
        .await
        .unwrap();
    svc.create_team("team-b", "ctx-other", "ideation")
        .await
        .unwrap();

    // Cleanup should only disband the team belonging to ctx-target
    svc.cleanup_stale_teams_for_context("ctx-target").await;

    // team-a should now be disbanded
    let status_a = svc.get_team_status("team-a").await.unwrap();
    assert_eq!(
        status_a.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded
    );

    // team-b should still be active (different context)
    let status_b = svc.get_team_status("team-b").await.unwrap();
    assert_ne!(
        status_b.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded
    );
}

#[tokio::test]
async fn test_cleanup_stale_teams_no_match() {
    let svc = test_service();
    svc.create_team("team-x", "ctx-1", "ideation")
        .await
        .unwrap();

    // Cleanup for a non-existent context should not panic or affect other teams
    svc.cleanup_stale_teams_for_context("ctx-nonexistent").await;

    // team-x should remain unaffected
    let status = svc.get_team_status("team-x").await.unwrap();
    assert_ne!(
        status.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded
    );
}

#[tokio::test]
async fn test_disband_sets_all_teammates_shutdown() {
    // Verifies disband_team transitions every teammate to Shutdown before marking Disbanded.
    // This mirrors the per-teammate emit_teammate_shutdown loop in team_service.rs.
    let svc = test_service();
    svc.create_team("t1", "ctx-1", "ideation").await.unwrap();
    svc.add_teammate("t1", "worker1", "#ff0000", "sonnet", "code")
        .await
        .unwrap();
    svc.add_teammate("t1", "worker2", "#00ff00", "sonnet", "code")
        .await
        .unwrap();
    svc.add_teammate("t1", "worker3", "#0000ff", "opus", "explore")
        .await
        .unwrap();

    svc.disband_team("t1").await.unwrap();

    let status = svc.get_team_status("t1").await.unwrap();
    assert_eq!(
        status.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded,
        "team phase must be Disbanded"
    );
    // Every teammate must be Shutdown (per-teammate events precede team:disbanded)
    for t in &status.teammates {
        assert_eq!(
            t.status,
            TeammateStatus::Shutdown,
            "teammate {} must be Shutdown after disband",
            t.name
        );
    }
}

/// Regression: persist_message must not silently drop messages when the calling
/// TeamService instance has an empty session_id_cache because create_team was
/// called on a DIFFERENT TeamService sharing the same tracker + repos.
///
/// Production scenario:
///  - Tauri service calls create_team via chat_service_streaming → populates its cache
///  - HTTP service checks team_exists → true → skips create_team → empty cache
///  - HTTP service calls add_teammate_message → must fall back to DB for session ID
#[tokio::test]
async fn test_persist_message_db_fallback_when_cache_empty() {
    let tracker = Arc::new(TeamStateTracker::new());
    let session_repo: Arc<dyn crate::domain::repositories::TeamSessionRepository> =
        Arc::new(crate::infrastructure::memory::MemoryTeamSessionRepository::new());
    let message_repo: Arc<dyn crate::domain::repositories::TeamMessageRepository> =
        Arc::new(MemoryTeamMessageRepository::new());

    // Service A: simulates the Tauri service that creates the team and populates its cache
    let svc_a = TeamService::new_with_repos_for_testing(
        tracker.clone(),
        Arc::clone(&session_repo),
        Arc::clone(&message_repo),
    );
    svc_a.create_team("team-x", "ctx-99", "ideation").await.unwrap();

    // Service B: simulates the HTTP service — same tracker + repos, but fresh empty cache
    let svc_b = TeamService::new_with_repos_for_testing(
        tracker.clone(),
        Arc::clone(&session_repo),
        Arc::clone(&message_repo),
    );

    // svc_b must find the session via DB fallback and persist the message
    let msg = svc_b
        .add_teammate_message(
            "team-x",
            "researcher-1",
            None,
            "Here are the results",
            TeamMessageType::TeammateMessage,
        )
        .await
        .unwrap();
    assert_eq!(msg.sender, "researcher-1");
    assert_eq!(msg.content, "Here are the results");

    // Verify message was actually persisted to the DB (not just in-memory tracker)
    let sessions = session_repo.get_by_context("ideation", "ctx-99").await.unwrap();
    let sid = &sessions[0].id;
    let persisted = message_repo.get_by_session(sid).await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].sender, "researcher-1");
    assert_eq!(persisted[0].content, "Here are the results");
}

#[tokio::test]
async fn test_disband_empty_team_succeeds() {
    // Disband a team with no teammates — no panic, phase → Disbanded
    let svc = test_service();
    svc.create_team("empty-team", "ctx-1", "ideation")
        .await
        .unwrap();

    svc.disband_team("empty-team").await.unwrap();

    let status = svc.get_team_status("empty-team").await.unwrap();
    assert_eq!(
        status.phase,
        super::super::team_state_tracker::TeamPhase::Disbanded
    );
    assert!(status.teammates.is_empty());
}
