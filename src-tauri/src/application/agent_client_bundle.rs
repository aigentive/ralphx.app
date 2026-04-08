use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::agents::{AgentHarnessKind, AgenticClient};

pub type AgentClientFactory = dyn Fn() -> Arc<dyn AgenticClient> + Send + Sync;

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

#[derive(Clone)]
pub struct AgentClientFactoryBundle {
    pub default_harness: AgentHarnessKind,
    pub default_factory: Arc<AgentClientFactory>,
    pub harness_factories: HashMap<AgentHarnessKind, Arc<AgentClientFactory>>,
}

impl AgentClientFactoryBundle {
    pub fn from_parts(
        default_harness: AgentHarnessKind,
        default_factory: Arc<AgentClientFactory>,
        harness_factories: HashMap<AgentHarnessKind, Arc<AgentClientFactory>>,
    ) -> Self {
        Self {
            default_harness,
            default_factory,
            harness_factories,
        }
    }

    pub fn instantiate(&self) -> AgentClientBundle {
        let harness_clients = self
            .harness_factories
            .iter()
            .map(|(harness, factory)| (*harness, (factory.as_ref())()))
            .collect();

        AgentClientBundle::from_parts(
            self.default_harness,
            (self.default_factory.as_ref())(),
            harness_clients,
        )
    }

    pub fn from_client_bundle(clients: &AgentClientBundle) -> Self {
        let default_client = Arc::clone(&clients.default_client);
        let harness_factories = clients
            .harness_clients
            .iter()
            .map(|(harness, client)| {
                let client = Arc::clone(client);
                (
                    *harness,
                    Arc::new(move || Arc::clone(&client)) as Arc<AgentClientFactory>,
                )
            })
            .collect();

        Self::from_parts(
            clients.default_harness,
            Arc::new(move || Arc::clone(&default_client)),
            harness_factories,
        )
    }
}
