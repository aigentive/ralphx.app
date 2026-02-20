use super::*;

#[test]
fn test_health_check_returns_ok_status() {
    let response = health_check();
    assert_eq!(response.status, "ok");
}

#[test]
fn test_health_response_serializes_correctly() {
    let response = health_check();
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"ok\""));
}
