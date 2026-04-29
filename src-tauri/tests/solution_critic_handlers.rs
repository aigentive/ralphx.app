use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use http_body_util::BodyExt;
use ralphx_lib::application::solution_critic::{
    CompileContextRequest, CritiqueArtifactRequest, SourceLimits,
};
use ralphx_lib::application::{AppState, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactType, IdeationSession, IdeationSessionId, Project, ProjectId,
    VerificationStatus,
};
use ralphx_lib::http_server::handlers::{
    get_compiled_context_artifact, get_solution_critique_artifact, post_compiled_context,
    post_solution_critique,
};
use ralphx_lib::http_server::types::HttpServerState;
use serde_json::Value;
use tower::ServiceExt;

fn solution_critic_app(state: HttpServerState) -> Router {
    Router::new()
        .route(
            "/api/ideation/sessions/:id/compiled-context",
            post(post_compiled_context),
        )
        .route(
            "/api/ideation/sessions/:id/compiled-context/:artifact_id",
            get(get_compiled_context_artifact),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique",
            post(post_solution_critique),
        )
        .route(
            "/api/ideation/sessions/:id/solution-critique/:artifact_id",
            get(get_solution_critique_artifact),
        )
        .with_state(state)
}

async fn setup_state() -> (HttpServerState, IdeationSessionId, ArtifactId) {
    let app_state = Arc::new(AppState::new_test());
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

    let project_id = ProjectId::from_string("project-solution-critic-http".to_string());
    let session_id = IdeationSessionId::from_string("session-solution-critic-http");
    let plan_artifact_id = ArtifactId::from_string("plan-artifact-http");

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

    let compile_body = serde_json::to_vec(&CompileContextRequest {
        target_artifact_id: plan_artifact_id.as_str().to_string(),
        source_limits: SourceLimits::default(),
    })
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

    let critique_body = serde_json::to_vec(&CritiqueArtifactRequest {
        target_artifact_id: plan_artifact_id.as_str().to_string(),
        compiled_context_artifact_id: context_artifact_id,
    })
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

    let critique_read_response = app
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
}

#[tokio::test]
async fn solution_critic_routes_enforce_project_scope() {
    let (state, session_id, plan_artifact_id) = setup_state().await;
    let app = solution_critic_app(state.clone());

    let compile_body = serde_json::to_vec(&CompileContextRequest {
        target_artifact_id: plan_artifact_id.as_str().to_string(),
        source_limits: SourceLimits::default(),
    })
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
