// Supervisor module - watchdog system for monitoring agent execution
// Provides event types, pattern detection, and intervention actions

pub mod actions;
pub mod events;
pub mod patterns;

pub use actions::{action_for_detection, action_for_severity, Severity, SupervisorAction};
pub use events::{ErrorInfo, ProgressInfo, SupervisorEvent, ToolCallInfo};
pub use patterns::{DetectionResult, Pattern, ToolCallWindow};
