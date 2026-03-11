/// Integration tests for the `/health` HTTP endpoint.
///
/// Tests verify:
/// - GET /health returns HTTP 200 with no body
/// - The handler is unauthenticated (no auth middleware required)
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::http_server::health_handler;

/// Build a minimal router containing only the /health route.
/// Mirrors how start_http_server registers the health endpoint.
fn health_router() -> Router {
    Router::new().route("/health", get(health_handler))
}

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    let app = health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GET /health must return 200 OK"
    );
}

#[tokio::test]
async fn test_health_endpoint_returns_empty_body() {
    let app = health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(
        body_bytes.is_empty(),
        "GET /health must return empty body, got: {:?}",
        body_bytes
    );
}

#[tokio::test]
async fn test_health_endpoint_not_found_for_other_paths() {
    let app = health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/other")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::OK,
        "Only /health should return 200, not arbitrary paths"
    );
}
