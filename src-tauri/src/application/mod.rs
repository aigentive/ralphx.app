// Application layer - dependency injection and service orchestration
// This layer bridges the domain and infrastructure layers

pub mod app_state;
pub mod dependency_service;
pub mod priority_service;
pub mod qa_service;
pub mod review_service;
pub mod supervisor_service;

// Re-export commonly used items
pub use app_state::AppState;
pub use dependency_service::{DependencyAnalysis, DependencyService, ValidationResult};
pub use priority_service::PriorityService;
pub use qa_service::{QAPrepStatus, QAService, TaskQAState};
pub use review_service::ReviewService;
pub use supervisor_service::{SupervisorConfig, SupervisorService, TaskMonitorState};
