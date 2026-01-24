// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod health;

// Re-export commands for registration
pub use health::health_check;
