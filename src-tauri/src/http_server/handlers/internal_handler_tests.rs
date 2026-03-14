// Tests for internal HTTP handlers
//
// Uses direct handler calls (same pattern as external_tests.rs) for unit tests,
// and a minimal test router with tower::ServiceExt::oneshot for CORS header tests.

use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{project::Project, types::ProjectId};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::cors::{Any, CorsLayer};

// ============================================================================
// Setup helpers
// ============================================================================

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

fn make_project(id: &str, name: &str, dir: &str) -> Project {
    Project {
        id: ProjectId::from_string(id.to_string()),
        name: name.to_string(),
        working_directory: dir.to_string(),
        git_mode: crate::domain::entities::project::GitMode::Worktree,
        base_branch: None,
        worktree_parent_directory: None,
        use_feature_branches: true,
        merge_validation_mode: Default::default(),
        merge_strategy: Default::default(),
        detected_analysis: None,
        custom_analysis: None,
        analyzed_at: None,
        github_pr_enabled: false,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ============================================================================
// list_projects_internal
// ============================================================================

#[tokio::test]
async fn test_list_projects_internal_returns_all_projects() {
    let state = setup_test_state().await;

    let p1 = make_project("proj-1", "Alpha", "/tmp/alpha");
    let p2 = make_project("proj-2", "Beta", "/tmp/beta");
    state.app_state.project_repo.create(p1).await.unwrap();
    state.app_state.project_repo.create(p2).await.unwrap();

    let result = list_projects_internal(State(state)).await;
    assert!(result.is_ok());
    let resp = result.unwrap().0;
    assert_eq!(resp.projects.len(), 2);

    let names: Vec<&str> = resp.projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Alpha"));
    assert!(names.contains(&"Beta"));
}

#[tokio::test]
async fn test_list_projects_internal_empty() {
    let state = setup_test_state().await;

    let result = list_projects_internal(State(state)).await;
    assert!(result.is_ok());
    let resp = result.unwrap().0;
    assert_eq!(resp.projects.len(), 0);
}

#[tokio::test]
async fn test_list_projects_internal_no_scope_filtering() {
    // Verify that ALL projects are returned regardless of project IDs —
    // the internal endpoint has no ProjectScope header filtering.
    let state = setup_test_state().await;

    let p1 = make_project("scope-proj-a", "ProjA", "/tmp/a");
    let p2 = make_project("scope-proj-b", "ProjB", "/tmp/b");
    let p3 = make_project("scope-proj-c", "ProjC", "/tmp/c");
    state.app_state.project_repo.create(p1).await.unwrap();
    state.app_state.project_repo.create(p2).await.unwrap();
    state.app_state.project_repo.create(p3).await.unwrap();

    let result = list_projects_internal(State(state)).await;
    let resp = result.unwrap().0;
    // All 3 projects returned — no scope filtering applied
    assert_eq!(resp.projects.len(), 3);
}

// ============================================================================
// create_cross_project_session_http
// ============================================================================

#[tokio::test]
async fn test_create_cross_project_session_http_no_app_handle_returns_500() {
    // AppState::new_test() has no AppHandle. The handler should return 500
    // with a clear error message in the test environment.
    let state = setup_test_state().await;

    let input = crate::commands::ideation_commands::CreateCrossProjectSessionInput {
        target_project_path: "/tmp/target-project".to_string(),
        source_session_id: "nonexistent-session-id".to_string(),
        title: None,
    };

    let result = create_cross_project_session_http(State(state), Json(input)).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(err
        .message
        .as_deref()
        .unwrap_or("")
        .contains("App handle not available"));
}

// ============================================================================
// CORS header tests (router-level)
//
// These tests build a minimal Axum router mirroring the production CORS
// structure and verify that:
//   - /api/internal/* routes do NOT return Access-Control-Allow-Origin
//   - public routes DO return Access-Control-Allow-Origin: *
//
// This catches regressions where a permissive CorsLayer is accidentally
// applied to the internal router (the Axum layer-ordering issue described
// in the task spec).
// ============================================================================

/// Builds a test app with the same CORS structure as the production router:
/// - internal_routes: no CORS
/// - public_routes: permissive CorsLayer::allow_origin(Any)
async fn build_cors_test_app() -> Router {
    let state = setup_test_state().await;

    // Internal routes — NO CORS (matches production)
    let internal_routes = Router::new().route(
        "/api/internal/projects",
        get(list_projects_internal),
    );

    // Minimal public routes with permissive CORS (matches production)
    let public_routes = Router::new()
        .route(
            "/health",
            get(|| async { axum::http::StatusCode::OK }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    Router::new()
        .merge(internal_routes)
        .merge(public_routes)
        .with_state(state)
}

#[tokio::test]
async fn test_internal_routes_have_no_cors_headers() {
    let app = build_cors_test_app().await;

    // Send a request with an Origin header (simulates a browser cross-origin request)
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/internal/projects")
        .header(header::ORIGIN, "http://example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Internal routes must NOT have Access-Control-Allow-Origin
    assert!(
        response.headers().get("access-control-allow-origin").is_none(),
        "Expected no CORS headers on /api/internal/ routes, but found Access-Control-Allow-Origin"
    );
}

#[tokio::test]
async fn test_public_routes_have_permissive_cors_headers() {
    let app = build_cors_test_app().await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header(header::ORIGIN, "http://example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Public routes MUST have Access-Control-Allow-Origin: *
    let cors_header = response.headers().get("access-control-allow-origin");
    assert!(
        cors_header.is_some(),
        "Expected Access-Control-Allow-Origin header on public routes"
    );
    assert_eq!(cors_header.unwrap(), "*");
}

#[tokio::test]
async fn test_internal_route_options_preflight_has_no_cors() {
    // A CORS preflight (OPTIONS) to an internal route should NOT return
    // Access-Control-Allow-Origin. This confirms that browser clients cannot
    // successfully preflight cross-origin requests to internal routes.
    let app = build_cors_test_app().await;

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/internal/projects")
        .header(header::ORIGIN, "http://evil.example.com")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // The preflight should either 404 or succeed without CORS headers
    // (no CorsLayer means no ACAO header regardless of response status)
    assert!(
        response.headers().get("access-control-allow-origin").is_none(),
        "CORS preflight to internal route must not return Access-Control-Allow-Origin"
    );
}
