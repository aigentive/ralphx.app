// Supervisor module - watchdog system for monitoring agent execution
// Provides event types, pattern detection, and intervention actions

pub mod events;
pub mod patterns;
pub mod actions;

pub use events::{SupervisorEvent, ToolCallInfo, ErrorInfo, ProgressInfo};
pub use patterns::{ToolCallWindow, DetectionResult, Pattern};
pub use actions::{action_for_detection, action_for_severity, SupervisorAction, Severity};
