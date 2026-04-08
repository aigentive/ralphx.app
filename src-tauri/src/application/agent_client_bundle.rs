use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::agents::{
    standard_harness_map, AgentHarnessKind, AgenticClient, DEFAULT_AGENT_HARNESS,
};
use crate::infrastructure::agents::CodexCliClient;
use crate::infrastructure::{ClaudeCodeClient, MockAgenticClient};

pub type AgentClientFactory = dyn Fn() -> Arc<dyn AgenticClient> + Send + Sync;

#[derive(Clone)]
pub struct AgentClientBundle {
    pub default_harness: AgentHarnessKind,
    pub default_client: Arc<dyn AgenticClient>,
    pub harness_clients: HashMap<AgentHarnessKind, Arc<dyn AgenticClient>>,
}

impl AgentClientBundle {
    pub fn from_default_client(
        default_harness: AgentHarnessKind,
        default_client: Arc<dyn AgenticClient>,
    ) -> Self {
        Self::from_parts(default_harness, default_client, HashMap::new())
    }

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

    pub fn with_harness_client(
        mut self,
        harness: AgentHarnessKind,
        client: Arc<dyn AgenticClient>,
    ) -> Self {
        self.harness_clients.insert(harness, client);
        self
    }

    pub fn with_harness_clients<I>(mut self, clients: I) -> Self
    where
        I: IntoIterator<Item = (AgentHarnessKind, Arc<dyn AgenticClient>)>,
    {
        self.harness_clients.extend(clients);
        self
    }

    pub fn standard_runtime_clients(
        default_client: Arc<dyn AgenticClient>,
        extra_clients: impl IntoIterator<Item = (AgentHarnessKind, Arc<dyn AgenticClient>)>,
    ) -> Self {
        Self::from_default_client(DEFAULT_AGENT_HARNESS, default_client)
            .with_harness_clients(extra_clients)
    }

    fn split_default_harness_value<T>(
        mut harness_values: HashMap<AgentHarnessKind, T>,
    ) -> (T, HashMap<AgentHarnessKind, T>) {
        let default_value = harness_values
            .remove(&DEFAULT_AGENT_HARNESS)
            .expect("standard runtime harness set must include DEFAULT_AGENT_HARNESS");
        (default_value, harness_values)
    }

    fn standard_runtime_client_for_harness(
        harness: AgentHarnessKind,
        mock: bool,
    ) -> Arc<dyn AgenticClient> {
        match (harness, mock) {
            (AgentHarnessKind::Claude, false) => Arc::new(ClaudeCodeClient::new()),
            (AgentHarnessKind::Claude, true) => Arc::new(MockAgenticClient::new()),
            (AgentHarnessKind::Codex, false) => Arc::new(CodexCliClient::new()),
            (AgentHarnessKind::Codex, true) => Arc::new(MockAgenticClient::new()),
        }
    }

    fn standard_runtime_clients_by_harness(
        mock: bool,
    ) -> HashMap<AgentHarnessKind, Arc<dyn AgenticClient>> {
        standard_harness_map(
            Self::standard_runtime_client_for_harness(AgentHarnessKind::Claude, mock),
            Self::standard_runtime_client_for_harness(AgentHarnessKind::Codex, mock),
        )
    }

    pub fn standard_production_runtime_clients() -> Self {
        let (default_client, harness_clients) =
            Self::split_default_harness_value(Self::standard_runtime_clients_by_harness(false));
        Self::from_default_client(DEFAULT_AGENT_HARNESS, default_client)
            .with_harness_clients(harness_clients)
    }

    pub fn standard_mock_runtime_clients() -> Self {
        let (default_client, harness_clients) =
            Self::split_default_harness_value(Self::standard_runtime_clients_by_harness(true));
        Self::from_default_client(DEFAULT_AGENT_HARNESS, default_client)
            .with_harness_clients(harness_clients)
    }

    pub fn resolve(&self, harness: AgentHarnessKind) -> Arc<dyn AgenticClient> {
        self.harness_clients
            .get(&harness)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.default_client))
    }

    pub fn explicit_harness_client(
        &self,
        harness: AgentHarnessKind,
    ) -> Option<Arc<dyn AgenticClient>> {
        self.harness_clients.get(&harness).cloned()
    }

    pub fn has_explicit_harness_client(&self, harness: AgentHarnessKind) -> bool {
        self.harness_clients.contains_key(&harness)
    }

    pub fn iter_explicit_harness_clients(
        &self,
    ) -> impl Iterator<Item = (AgentHarnessKind, Arc<dyn AgenticClient>)> + '_ {
        self.harness_clients
            .iter()
            .map(|(harness, client)| (*harness, Arc::clone(client)))
    }
}

#[derive(Clone)]
pub struct AgentClientFactoryBundle {
    pub default_harness: AgentHarnessKind,
    pub default_factory: Arc<AgentClientFactory>,
    pub harness_factories: HashMap<AgentHarnessKind, Arc<AgentClientFactory>>,
}

impl AgentClientFactoryBundle {
    pub fn from_default_factory(
        default_harness: AgentHarnessKind,
        default_factory: Arc<AgentClientFactory>,
    ) -> Self {
        Self::from_parts(default_harness, default_factory, HashMap::new())
    }

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

    pub fn with_harness_factory(
        mut self,
        harness: AgentHarnessKind,
        factory: Arc<AgentClientFactory>,
    ) -> Self {
        self.harness_factories.insert(harness, factory);
        self
    }

    pub fn with_harness_factories<I>(mut self, factories: I) -> Self
    where
        I: IntoIterator<Item = (AgentHarnessKind, Arc<AgentClientFactory>)>,
    {
        self.harness_factories.extend(factories);
        self
    }

    pub fn standard_runtime_factories(
        default_factory: Arc<AgentClientFactory>,
        extra_factories: impl IntoIterator<Item = (AgentHarnessKind, Arc<AgentClientFactory>)>,
    ) -> Self {
        Self::from_default_factory(DEFAULT_AGENT_HARNESS, default_factory)
            .with_harness_factories(extra_factories)
    }

    fn standard_runtime_factory_for_harness(harness: AgentHarnessKind) -> Arc<AgentClientFactory> {
        match harness {
            AgentHarnessKind::Claude => {
                Arc::new(|| Arc::new(ClaudeCodeClient::new()) as Arc<dyn AgenticClient>)
            }
            AgentHarnessKind::Codex => {
                Arc::new(|| Arc::new(CodexCliClient::new()) as Arc<dyn AgenticClient>)
            }
        }
    }

    fn standard_runtime_factories_by_harness() -> HashMap<AgentHarnessKind, Arc<AgentClientFactory>>
    {
        standard_harness_map(
            Self::standard_runtime_factory_for_harness(AgentHarnessKind::Claude),
            Self::standard_runtime_factory_for_harness(AgentHarnessKind::Codex),
        )
    }

    pub fn standard_production_runtime_factories() -> Self {
        let (default_factory, harness_factories) = AgentClientBundle::split_default_harness_value(
            Self::standard_runtime_factories_by_harness(),
        );
        Self::from_default_factory(DEFAULT_AGENT_HARNESS, default_factory)
            .with_harness_factories(harness_factories)
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

    pub fn has_explicit_harness_factory(&self, harness: AgentHarnessKind) -> bool {
        self.harness_factories.contains_key(&harness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_runtime_clients_uses_domain_default_harness() {
        let bundle = AgentClientBundle::standard_mock_runtime_clients();

        assert_eq!(bundle.default_harness, DEFAULT_AGENT_HARNESS);
        assert!(bundle.has_explicit_harness_client(AgentHarnessKind::Codex));
    }

    #[test]
    fn standard_runtime_factories_uses_domain_default_harness() {
        let bundle = AgentClientFactoryBundle::standard_production_runtime_factories();

        assert_eq!(bundle.default_harness, DEFAULT_AGENT_HARNESS);
        assert!(bundle.has_explicit_harness_factory(AgentHarnessKind::Codex));
    }
}
