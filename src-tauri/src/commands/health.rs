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
#[path = "health_tests.rs"]
mod tests;
