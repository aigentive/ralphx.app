use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::agents::{AgentHarnessKind, AgenticClient};

#[derive(Clone)]
pub struct AgentClientBundle {
    pub default_harness: AgentHarnessKind,
    pub default_client: Arc<dyn AgenticClient>,
    pub harness_clients: HashMap<AgentHarnessKind, Arc<dyn AgenticClient>>,
}

impl AgentClientBundle {
    pub fn from_parts(
        default_harness: AgentHarnessKind,
        default_client: Arc<dyn AgenticClient>,
        harness_clients: HashMap<AgentHarnessKind, Arc<dyn AgenticClient>>,
    ) -> Self {
        Self {
            default_harness,
            default_client,
            harness_clients,
        }
    }

    pub fn resolve(&self, harness: AgentHarnessKind) -> Arc<dyn AgenticClient> {
        self.harness_clients
            .get(&harness)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.default_client))
    }
}
