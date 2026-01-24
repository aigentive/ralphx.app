// Health check command for verifying backend is running

use serde::Serialize;

/// Response returned by the health check command
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Health check Tauri command
/// Returns { status: "ok" } if the backend is running
#[tauri::command]
pub fn health_check() -> HealthResponse {
    HealthResponse {
        status: "ok".to_string(),
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_health_response_has_debug_trait() {
        let response = health_check();
        let debug = format!("{:?}", response);
        assert!(debug.contains("HealthResponse"));
        assert!(debug.contains("ok"));
    }

    #[test]
    fn test_health_response_clone_works() {
        let response = health_check();
        let cloned = response.clone();
        assert_eq!(response.status, cloned.status);
    }
}
