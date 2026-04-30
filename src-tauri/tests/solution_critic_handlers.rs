use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use http_body_util::BodyExt;
use ralphx_lib::application::solution_critic::{CompileContextRequest, CritiqueArtifactRequest};
use ralphx_lib::application::{AppState, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactType, IdeationSession, IdeationSessionId, Project, ProjectId,
    VerificationStatus,
};
use ralphx_lib::http_server::handlers::{
    get_compiled_context_artifact, get_compiled_context_history_for_target,
    get_latest_compiled_context, get_latest_compiled_context_for_target,
    get_latest_solution_critique, get_latest_solution_critique_for_target,
    get_solution_critique_artifact, get_solution_critique_history_for_target,
    get_solution_critique_projected_gaps, get_solution_critique_rollup, post_compiled_context,
    post_solution_critique, post_solution_critique_projected_gap_action,
};
use ralphx_lib::http_server::types::HttpServerState;
use ralphx_lib::infrastructure::MockAgenticClient;
use serde_json::Value;
use tower::ServiceExt;

fn solution_critic_app(state: HttpServerState) -> Router {
    Router::new()
        .route(
            "/api/ideation/sessions/:id/compiled-context",
            get(get_latest_compiled_context).post(post_compiled_context),
        )
        .route(
            "/api/ideation/sessions/:id/compiled-context/target/:target_type/:target_id",
            get(get_latest_compiled_context_for_target),
        )
        .route(
            "/api/ideation/sessions/:id/compiled-context/target/:target_type/:target_id/history",
            get(get_compiled_context_history_for_target),
        )
        .route(
            "/api/ideation/sessions/:id/compiled-context/:artifact_id",
            get(get_compiled_context_artifact),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique",
            get(get_latest_solution_critique).post(post_solution_critique),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/target/:target_type/:target_id",
            get(get_latest_solution_critique_for_target),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/target/:target_type/:target_id/history",
            get(get_solution_critique_history_for_target),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/rollup",
            get(get_solution_critique_rollup),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/:artifact_id/projected-gaps",
            get(get_solution_critique_projected_gaps),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/:artifact_id/projected-gaps/:gap_id/actions",
            axum::routing::post(post_solution_critique_projected_gap_action),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/:artifact_id",
            get(get_solution_critique_artifact),
        )
        .with_state(state)
}

fn handler_compile_response(plan_id: &ArtifactId) -> String {
    format!(
        r#"{{
            "claims": [{{
                "id": "claim-endpoints",
                "text": "The plan exposes solution critic endpoints.",
                "classification": "fact",
                "confidence": "high",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}]
            }}],
            "open_questions": [],
            "stale_assumptions": []
        }}"#
    )
}

fn handler_critique_response(plan_id: &ArtifactId) -> String {
    format!(
        r#"{{
            "verdict": "investigate",
            "confidence": "medium",
            "claims": [{{
                "id": "claim-endpoints-review",
                "claim": "The plan exposes solution critic endpoints.",
                "status": "unclear",
                "confidence": "medium",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}],
                "notes": "The endpoint claim still needs test evidence."
            }}],
            "recommendations": [],
            "risks": [{{
                "id": "risk-endpoint-proof",
                "risk": "Endpoint wiring may drift without handler coverage.",
                "severity": "medium",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}],
                "mitigation": "Run the solution critic handler test."
            }}],
            "verification_plan": [{{
                "id": "verify-endpoints",
                "requirement": "Verify compile and critique routes persist artifacts.",
                "priority": "medium",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}],
                "suggested_test": "cargo test --test solution_critic_handlers"
            }}],
            "safe_next_action": "Run the solution critic handler test."
        }}"#
    )
}

async fn setup_state() -> (HttpServerState, IdeationSessionId, ArtifactId) {
    let project_id = ProjectId::from_string("project-solution-critic-http".to_string());
    let session_id = IdeationSessionId::from_string("session-solution-critic-http");
    let plan_artifact_id = ArtifactId::from_string("plan-artifact-http");
    let mock_agent = Arc::new(MockAgenticClient::new());
    mock_agent
        .when_prompt_contains(
            "solution context compiler",
            &handler_compile_response(&plan_artifact_id),
        )
        .await;
    mock_agent
        .when_prompt_contains(
            "You are RalphX's solution critic",
            &handler_critique_response(&plan_artifact_id),
        )
        .await;
    let app_state = Arc::new(AppState::new_test().with_agent_client(mock_agent));
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
    let state = HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
        delegation_service: Default::default(),
    };

    let mut project = Project::new(
        "HTTP Solution Critic".to_string(),
        "/tmp/ralphx".to_string(),
    );
    project.id = project_id.clone();
    state.app_state.project_repo.create(project).await.unwrap();

    let mut plan = Artifact::new_inline(
        "Implementation Plan",
        ArtifactType::Specification,
        "Expose solution critic endpoints.",
        "orchestrator",
    );
    plan.id = plan_artifact_id.clone();
    state.app_state.artifact_repo.create(plan).await.unwrap();

    let session = IdeationSession::builder()
        .id(session_id.clone())
        .project_id(project_id)
        .plan_artifact_id(plan_artifact_id.clone())
        .verification_status(VerificationStatus::Unverified)
        .build();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    (state, session_id, plan_artifact_id)
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn solution_critic_routes_compile_context_and_critique_artifact() {
    let (state, session_id, plan_artifact_id) = setup_state().await;
    let app = solution_critic_app(state);

    let compile_body = serde_json::to_vec(&CompileContextRequest::for_plan_artifact(
        plan_artifact_id.as_str(),
    ))
    .unwrap();
    let compile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context",
                    session_id.as_str()
                ))
                .header("content-type", "application/json")
                .body(Body::from(compile_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(compile_response.status(), StatusCode::OK);
    let compile_json = response_json(compile_response).await;
    let context_artifact_id = compile_json["artifact_id"].as_str().unwrap().to_string();
    assert_eq!(
        compile_json["compiled_context"]["target"]["id"].as_str(),
        Some(plan_artifact_id.as_str())
    );

    let latest_context_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context",
                    session_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(latest_context_response.status(), StatusCode::OK);
    let latest_context_json = response_json(latest_context_response).await;
    assert_eq!(
        latest_context_json["artifact_id"].as_str(),
        Some(context_artifact_id.as_str())
    );

    let latest_target_context_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context/target/plan_artifact/{}",
                    session_id.as_str(),
                    plan_artifact_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(latest_target_context_response.status(), StatusCode::OK);
    let latest_target_context_json = response_json(latest_target_context_response).await;
    assert_eq!(
        latest_target_context_json["artifact_id"].as_str(),
        Some(context_artifact_id.as_str())
    );

    let context_read_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context/{}",
                    session_id.as_str(),
                    context_artifact_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(context_read_response.status(), StatusCode::OK);

    let critique_body = serde_json::to_vec(&CritiqueArtifactRequest::for_plan_artifact(
        plan_artifact_id.as_str(),
        context_artifact_id,
    ))
    .unwrap();
    let critique_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique",
                    session_id.as_str()
                ))
                .header("content-type", "application/json")
                .body(Body::from(critique_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(critique_response.status(), StatusCode::OK);
    let critique_json = response_json(critique_response).await;
    let critique_artifact_id = critique_json["artifact_id"].as_str().unwrap();
    assert_eq!(
        critique_json["solution_critique"]["artifact_id"].as_str(),
        Some(plan_artifact_id.as_str())
    );
    let projected_gaps = critique_json["projected_gaps"].as_array().unwrap();
    assert_eq!(projected_gaps.len(), 3);
    assert!(projected_gaps
        .iter()
        .all(|gap| gap["source"].as_str().is_none()));

    let latest_critique_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique",
                    session_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(latest_critique_response.status(), StatusCode::OK);
    let latest_critique_json = response_json(latest_critique_response).await;
    assert_eq!(
        latest_critique_json["artifact_id"].as_str(),
        Some(critique_artifact_id)
    );

    let latest_target_critique_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/target/plan_artifact/{}",
                    session_id.as_str(),
                    plan_artifact_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(latest_target_critique_response.status(), StatusCode::OK);
    let latest_target_critique_json = response_json(latest_target_critique_response).await;
    assert_eq!(
        latest_target_critique_json["artifact_id"].as_str(),
        Some(critique_artifact_id)
    );

    let critique_read_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/{}",
                    session_id.as_str(),
                    critique_artifact_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(critique_read_response.status(), StatusCode::OK);
    let critique_read_json = response_json(critique_read_response).await;
    assert_eq!(
        critique_read_json["projected_gaps"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        critique_read_json["projected_gap_items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let projected_gaps_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/{}/projected-gaps",
                    session_id.as_str(),
                    critique_artifact_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(projected_gaps_response.status(), StatusCode::OK);
    let projected_gaps_json = response_json(projected_gaps_response).await;
    let first_gap_id = projected_gaps_json[0]["id"].as_str().unwrap().to_string();
    let expected_source = format!("solution_critique:{critique_artifact_id}:{first_gap_id}");
    assert_eq!(
        projected_gaps_json[0]["verification_gap"]["source"].as_str(),
        Some(expected_source.as_str())
    );

    let action_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/{}/projected-gaps/{}/actions",
                    session_id.as_str(),
                    critique_artifact_id,
                    first_gap_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"action":"deferred","note":"Later"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(action_response.status(), StatusCode::OK);
    let action_json = response_json(action_response).await;
    assert_eq!(action_json["gap"]["status"].as_str(), Some("deferred"));
    assert_eq!(action_json["verification_updated"].as_bool(), Some(false));

    let critique_history_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/target/plan_artifact/{}/history",
                    session_id.as_str(),
                    plan_artifact_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(critique_history_response.status(), StatusCode::OK);
    let critique_history_json = response_json(critique_history_response).await;
    assert_eq!(critique_history_json.as_array().unwrap().len(), 1);
    assert_eq!(
        critique_history_json[0]["latest_gap_actions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let context_history_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context/target/plan_artifact/{}/history",
                    session_id.as_str(),
                    plan_artifact_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(context_history_response.status(), StatusCode::OK);
    let context_history_json = response_json(context_history_response).await;
    assert_eq!(context_history_json.as_array().unwrap().len(), 1);

    let rollup_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique/rollup",
                    session_id.as_str()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rollup_response.status(), StatusCode::OK);
    let rollup_json = response_json(rollup_response).await;
    assert_eq!(rollup_json["target_count"].as_u64(), Some(1));
    assert_eq!(rollup_json["critique_count"].as_u64(), Some(1));
    assert_eq!(rollup_json["deferred_gap_count"].as_u64(), Some(1));
}

#[tokio::test]
async fn solution_critic_routes_enforce_project_scope() {
    let (state, session_id, plan_artifact_id) = setup_state().await;
    let app = solution_critic_app(state.clone());

    let compile_body = serde_json::to_vec(&CompileContextRequest::for_plan_artifact(
        plan_artifact_id.as_str(),
    ))
    .unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ideation/sessions/{}/compiled-context",
                    session_id.as_str()
                ))
                .header("content-type", "application/json")
                .header("x-ralphx-project-scope", "some-other-project")
                .body(Body::from(compile_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let contexts = state
        .app_state
        .artifact_repo
        .get_by_type(ArtifactType::Context)
        .await
        .unwrap();
    assert!(contexts.is_empty());
}

#[tokio::test]
async fn solution_critic_route_requires_existing_context_artifact() {
    let (state, session_id, plan_artifact_id) = setup_state().await;
    let app = solution_critic_app(state.clone());

    let critique_body = serde_json::to_vec(&CritiqueArtifactRequest::for_plan_artifact(
        plan_artifact_id.as_str(),
        "missing-context-artifact",
    ))
    .unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/ideation/sessions/{}/solution-critique",
                    session_id.as_str()
                ))
                .header("content-type", "application/json")
                .body(Body::from(critique_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let findings = state
        .app_state
        .artifact_repo
        .get_by_type(ArtifactType::Findings)
        .await
        .unwrap();
    assert!(findings.is_empty());
}
