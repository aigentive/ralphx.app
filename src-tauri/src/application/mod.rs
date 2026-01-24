// Application layer - dependency injection and service orchestration
// This layer bridges the domain and infrastructure layers

pub mod app_state;

// Re-export commonly used items
pub use app_state::AppState;
