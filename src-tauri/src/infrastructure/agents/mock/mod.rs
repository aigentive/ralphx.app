// Mock agent implementations
// For testing without making real API calls

mod mock_client;

pub use mock_client::{MockAgenticClient, MockCall, MockCallType};
