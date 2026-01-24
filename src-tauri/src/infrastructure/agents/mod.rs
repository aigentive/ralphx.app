// Agent implementations
// Infrastructure layer implementations of the AgenticClient trait

pub mod claude;
pub mod mock;
pub mod spawner;

// Re-export commonly used items
pub use claude::ClaudeCodeClient;
pub use mock::{MockAgenticClient, MockCall, MockCallType};
pub use spawner::AgenticClientSpawner;
