// Application layer - dependency injection and service orchestration
// This layer bridges the domain and infrastructure layers

pub mod app_state;
pub mod qa_service;
pub mod supervisor_service;

// Re-export commonly used items
pub use app_state::AppState;
pub use qa_service::{QAPrepStatus, QAService, TaskQAState};
pub use supervisor_service::{SupervisorConfig, SupervisorService, TaskMonitorState};
