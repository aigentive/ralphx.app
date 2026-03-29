pub mod settings;
pub mod status_counting;

pub use settings::{ExecutionSettings, GlobalExecutionSettings};
pub use status_counting::{
    ExecutionStatusCounts, ScopedExecutionSubject, context_matches_running_status,
    count_execution_status,
};
