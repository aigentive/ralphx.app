// Agent Profile
// Complete agent configuration composed of Claude Code components

use serde::{Deserialize, Serialize};

/// Role of an agent in the RalphX system (for profiles)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRole {
    Worker,
    Reviewer,
    Supervisor,
    Orchestrator,
    Researcher,
}

impl std::fmt::Display for ProfileRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileRole::Worker => write!(f, "worker"),
            ProfileRole::Reviewer => write!(f, "reviewer"),
            ProfileRole::Supervisor => write!(f, "supervisor"),
            ProfileRole::Orchestrator => write!(f, "orchestrator"),
            ProfileRole::Researcher => write!(f, "researcher"),
        }
    }
}

impl std::str::FromStr for ProfileRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "worker" => Ok(ProfileRole::Worker),
            "reviewer" => Ok(ProfileRole::Reviewer),
            "supervisor" => Ok(ProfileRole::Supervisor),
            "orchestrator" => Ok(ProfileRole::Orchestrator),
            "researcher" => Ok(ProfileRole::Researcher),
            _ => Err(format!("Invalid profile role: {}", s)),
        }
    }
}

/// Model short forms for Claude 4.5 models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// claude-opus-4-5-20251101
    Opus,
    /// claude-sonnet-4-5-20250929
    Sonnet,
    /// claude-haiku-4-5-20251001
    Haiku,
}

impl Model {
    /// Get the full model ID
    pub fn model_id(&self) -> &'static str {
        match self {
            Model::Opus => "claude-opus-4-5-20251101",
            Model::Sonnet => "claude-sonnet-4-5-20250929",
            Model::Haiku => "claude-haiku-4-5-20251001",
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::Opus => write!(f, "opus"),
            Model::Sonnet => write!(f, "sonnet"),
            Model::Haiku => write!(f, "haiku"),
        }
    }
}

/// Permission mode for agent execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    BypassPermissions,
}


/// Autonomy level for agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AutonomyLevel {
    #[default]
    Supervised,
    SemiAutonomous,
    FullyAutonomous,
}


/// Claude Code component references
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeConfig {
    /// Agent name (resolved via --plugin-dir, e.g. "worker" resolves to ralphx-plugin/agents/worker.md)
    pub agent: String,
    /// Skills to inject at startup (resolved via plugin discovery)
    #[serde(default)]
    pub skills: Vec<String>,
    /// Agent-scoped hooks configuration (JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<serde_json::Value>,
    /// MCP servers to enable
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}

impl ClaudeCodeConfig {
    pub fn new(agent: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            skills: vec![],
            hooks: None,
            mcp_servers: vec![],
        }
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }
}

/// Execution configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionConfig {
    /// Model to use
    pub model: Model,
    /// Maximum iterations before stopping
    pub max_iterations: u32,
    /// Timeout in minutes
    pub timeout_minutes: u32,
    /// Permission mode for file operations
    #[serde(default)]
    pub permission_mode: PermissionMode,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            model: Model::Sonnet,
            max_iterations: 30,
            timeout_minutes: 30,
            permission_mode: PermissionMode::Default,
        }
    }
}

impl ExecutionConfig {
    pub fn with_model(mut self, model: Model) -> Self {
        self.model = model;
        self
    }

    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_timeout(mut self, minutes: u32) -> Self {
        self.timeout_minutes = minutes;
        self
    }

    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }
}

/// Artifact I/O configuration
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IoConfig {
    /// Types of artifacts this agent can consume
    #[serde(default)]
    pub input_artifact_types: Vec<String>,
    /// Types of artifacts this agent can produce
    #[serde(default)]
    pub output_artifact_types: Vec<String>,
}

impl IoConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_inputs(mut self, types: Vec<String>) -> Self {
        self.input_artifact_types = types;
        self
    }

    pub fn with_outputs(mut self, types: Vec<String>) -> Self {
        self.output_artifact_types = types;
        self
    }
}

/// Behavioral configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorConfig {
    /// Whether this agent can spawn sub-agents
    #[serde(default)]
    pub can_spawn_sub_agents: bool,
    /// Whether to auto-commit changes
    #[serde(default)]
    pub auto_commit: bool,
    /// Autonomy level
    #[serde(default)]
    pub autonomy_level: AutonomyLevel,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            can_spawn_sub_agents: false,
            auto_commit: false,
            autonomy_level: AutonomyLevel::Supervised,
        }
    }
}

impl BehaviorConfig {
    pub fn with_sub_agents(mut self, allowed: bool) -> Self {
        self.can_spawn_sub_agents = allowed;
        self
    }

    pub fn with_auto_commit(mut self, enabled: bool) -> Self {
        self.auto_commit = enabled;
        self
    }

    pub fn with_autonomy(mut self, level: AutonomyLevel) -> Self {
        self.autonomy_level = level;
        self
    }
}

/// Complete agent profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description of what this agent does
    pub description: String,
    /// Role of this agent
    pub role: ProfileRole,
    /// Claude Code component configuration
    pub claude_code: ClaudeCodeConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Artifact I/O configuration
    pub io: IoConfig,
    /// Behavioral flags
    pub behavior: BehaviorConfig,
}

impl AgentProfile {
    /// Create a new agent profile
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        role: ProfileRole,
        agent_definition: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            role,
            claude_code: ClaudeCodeConfig::new(agent_definition),
            execution: ExecutionConfig::default(),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }

    /// Create the built-in worker profile
    pub fn worker() -> Self {
        Self {
            id: "worker".to_string(),
            name: "Worker".to_string(),
            description: "Executes implementation tasks autonomously".to_string(),
            role: ProfileRole::Worker,
            claude_code: ClaudeCodeConfig::new("worker")
                .with_skills(vec![
                    "coding-standards".to_string(),
                    "testing-patterns".to_string(),
                    "git-workflow".to_string(),
                ]),
            execution: ExecutionConfig::default()
                .with_model(Model::Sonnet)
                .with_max_iterations(30)
                .with_permission_mode(PermissionMode::AcceptEdits),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default()
                .with_auto_commit(true)
                .with_autonomy(AutonomyLevel::SemiAutonomous),
        }
    }

    /// Create the built-in reviewer profile
    pub fn reviewer() -> Self {
        Self {
            id: "reviewer".to_string(),
            name: "Reviewer".to_string(),
            description: "Reviews code changes for quality and correctness".to_string(),
            role: ProfileRole::Reviewer,
            claude_code: ClaudeCodeConfig::new("reviewer")
                .with_skills(vec![
                    "code-review-checklist".to_string(),
                ]),
            execution: ExecutionConfig::default()
                .with_model(Model::Sonnet)
                .with_max_iterations(10),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }

    /// Create the built-in supervisor profile
    pub fn supervisor() -> Self {
        Self {
            id: "supervisor".to_string(),
            name: "Supervisor".to_string(),
            description: "Monitors task execution and intervenes when problems occur".to_string(),
            role: ProfileRole::Supervisor,
            claude_code: ClaudeCodeConfig::new("supervisor"),
            execution: ExecutionConfig::default()
                .with_model(Model::Haiku)
                .with_max_iterations(100),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }

    /// Create the built-in orchestrator profile
    pub fn orchestrator() -> Self {
        Self {
            id: "orchestrator".to_string(),
            name: "Orchestrator".to_string(),
            description: "Plans and coordinates complex multi-step tasks".to_string(),
            role: ProfileRole::Orchestrator,
            claude_code: ClaudeCodeConfig::new("orchestrator"),
            execution: ExecutionConfig::default()
                .with_model(Model::Opus)
                .with_max_iterations(50),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default()
                .with_sub_agents(true)
                .with_autonomy(AutonomyLevel::FullyAutonomous),
        }
    }

    /// Create the built-in deep-researcher profile
    pub fn deep_researcher() -> Self {
        Self {
            id: "deep-researcher".to_string(),
            name: "Deep Researcher".to_string(),
            description: "Conducts thorough research and analysis".to_string(),
            role: ProfileRole::Researcher,
            claude_code: ClaudeCodeConfig::new("deep-researcher")
                .with_skills(vec![
                    "research-methodology".to_string(),
                ]),
            execution: ExecutionConfig::default()
                .with_model(Model::Opus)
                .with_max_iterations(200),
            io: IoConfig::default(),
            behavior: BehaviorConfig::default()
                .with_autonomy(AutonomyLevel::FullyAutonomous),
        }
    }

    /// Get all built-in profiles
    pub fn builtin_profiles() -> Vec<Self> {
        vec![
            Self::worker(),
            Self::reviewer(),
            Self::supervisor(),
            Self::orchestrator(),
            Self::deep_researcher(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ProfileRole tests
    #[test]
    fn test_profile_role_display() {
        assert_eq!(ProfileRole::Worker.to_string(), "worker");
        assert_eq!(ProfileRole::Reviewer.to_string(), "reviewer");
        assert_eq!(ProfileRole::Supervisor.to_string(), "supervisor");
        assert_eq!(ProfileRole::Orchestrator.to_string(), "orchestrator");
        assert_eq!(ProfileRole::Researcher.to_string(), "researcher");
    }

    #[test]
    fn test_profile_role_from_str() {
        assert_eq!("worker".parse::<ProfileRole>().unwrap(), ProfileRole::Worker);
        assert_eq!("reviewer".parse::<ProfileRole>().unwrap(), ProfileRole::Reviewer);
        assert_eq!("supervisor".parse::<ProfileRole>().unwrap(), ProfileRole::Supervisor);
        assert_eq!("orchestrator".parse::<ProfileRole>().unwrap(), ProfileRole::Orchestrator);
        assert_eq!("researcher".parse::<ProfileRole>().unwrap(), ProfileRole::Researcher);
    }

    #[test]
    fn test_profile_role_from_str_invalid() {
        assert!("invalid".parse::<ProfileRole>().is_err());
    }

    #[test]
    fn test_profile_role_serialize() {
        let json = serde_json::to_string(&ProfileRole::Worker).unwrap();
        assert_eq!(json, "\"worker\"");
    }

    #[test]
    fn test_profile_role_deserialize() {
        let role: ProfileRole = serde_json::from_str("\"orchestrator\"").unwrap();
        assert_eq!(role, ProfileRole::Orchestrator);
    }

    // Model tests
    #[test]
    fn test_model_display() {
        assert_eq!(Model::Opus.to_string(), "opus");
        assert_eq!(Model::Sonnet.to_string(), "sonnet");
        assert_eq!(Model::Haiku.to_string(), "haiku");
    }

    #[test]
    fn test_model_id() {
        assert_eq!(Model::Opus.model_id(), "claude-opus-4-5-20251101");
        assert_eq!(Model::Sonnet.model_id(), "claude-sonnet-4-5-20250929");
        assert_eq!(Model::Haiku.model_id(), "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_model_serialize() {
        let json = serde_json::to_string(&Model::Sonnet).unwrap();
        assert_eq!(json, "\"sonnet\"");
    }

    #[test]
    fn test_model_deserialize() {
        let model: Model = serde_json::from_str("\"opus\"").unwrap();
        assert_eq!(model, Model::Opus);
    }

    // PermissionMode tests
    #[test]
    fn test_permission_mode_default() {
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }

    #[test]
    fn test_permission_mode_serialize() {
        let json = serde_json::to_string(&PermissionMode::AcceptEdits).unwrap();
        assert_eq!(json, "\"acceptEdits\"");
    }

    // AutonomyLevel tests
    #[test]
    fn test_autonomy_level_default() {
        assert_eq!(AutonomyLevel::default(), AutonomyLevel::Supervised);
    }

    #[test]
    fn test_autonomy_level_serialize() {
        let json = serde_json::to_string(&AutonomyLevel::FullyAutonomous).unwrap();
        assert_eq!(json, "\"fully_autonomous\"");
    }

    // ClaudeCodeConfig tests
    #[test]
    fn test_claude_code_config_new() {
        let config = ClaudeCodeConfig::new("worker");
        assert_eq!(config.agent, "worker");
        assert!(config.skills.is_empty());
        assert!(config.hooks.is_none());
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_claude_code_config_with_skills() {
        let config = ClaudeCodeConfig::new("worker")
            .with_skills(vec!["skill1".to_string(), "skill2".to_string()]);
        assert_eq!(config.skills.len(), 2);
    }

    // ExecutionConfig tests
    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.model, Model::Sonnet);
        assert_eq!(config.max_iterations, 30);
        assert_eq!(config.timeout_minutes, 30);
        assert_eq!(config.permission_mode, PermissionMode::Default);
    }

    #[test]
    fn test_execution_config_builder() {
        let config = ExecutionConfig::default()
            .with_model(Model::Opus)
            .with_max_iterations(50)
            .with_timeout(60)
            .with_permission_mode(PermissionMode::BypassPermissions);

        assert_eq!(config.model, Model::Opus);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.timeout_minutes, 60);
        assert_eq!(config.permission_mode, PermissionMode::BypassPermissions);
    }

    // IoConfig tests
    #[test]
    fn test_io_config_default() {
        let config = IoConfig::default();
        assert!(config.input_artifact_types.is_empty());
        assert!(config.output_artifact_types.is_empty());
    }

    #[test]
    fn test_io_config_builder() {
        let config = IoConfig::new()
            .with_inputs(vec!["prd".to_string()])
            .with_outputs(vec!["code".to_string(), "tests".to_string()]);

        assert_eq!(config.input_artifact_types.len(), 1);
        assert_eq!(config.output_artifact_types.len(), 2);
    }

    // BehaviorConfig tests
    #[test]
    fn test_behavior_config_default() {
        let config = BehaviorConfig::default();
        assert!(!config.can_spawn_sub_agents);
        assert!(!config.auto_commit);
        assert_eq!(config.autonomy_level, AutonomyLevel::Supervised);
    }

    #[test]
    fn test_behavior_config_builder() {
        let config = BehaviorConfig::default()
            .with_sub_agents(true)
            .with_auto_commit(true)
            .with_autonomy(AutonomyLevel::FullyAutonomous);

        assert!(config.can_spawn_sub_agents);
        assert!(config.auto_commit);
        assert_eq!(config.autonomy_level, AutonomyLevel::FullyAutonomous);
    }

    // AgentProfile tests
    #[test]
    fn test_agent_profile_new() {
        let profile = AgentProfile::new(
            "test",
            "Test Agent",
            "A test agent",
            ProfileRole::Worker,
            "test",
        );

        assert_eq!(profile.id, "test");
        assert_eq!(profile.name, "Test Agent");
        assert_eq!(profile.description, "A test agent");
        assert_eq!(profile.role, ProfileRole::Worker);
        assert_eq!(profile.claude_code.agent, "test");
    }

    #[test]
    fn test_agent_profile_worker() {
        let profile = AgentProfile::worker();
        assert_eq!(profile.id, "worker");
        assert_eq!(profile.role, ProfileRole::Worker);
        assert_eq!(profile.execution.model, Model::Sonnet);
        assert_eq!(profile.execution.max_iterations, 30);
        assert!(profile.behavior.auto_commit);
        assert_eq!(profile.claude_code.skills.len(), 3);
    }

    #[test]
    fn test_agent_profile_reviewer() {
        let profile = AgentProfile::reviewer();
        assert_eq!(profile.id, "reviewer");
        assert_eq!(profile.role, ProfileRole::Reviewer);
        assert_eq!(profile.execution.model, Model::Sonnet);
        assert_eq!(profile.execution.max_iterations, 10);
    }

    #[test]
    fn test_agent_profile_supervisor() {
        let profile = AgentProfile::supervisor();
        assert_eq!(profile.id, "supervisor");
        assert_eq!(profile.role, ProfileRole::Supervisor);
        assert_eq!(profile.execution.model, Model::Haiku);
        assert_eq!(profile.execution.max_iterations, 100);
    }

    #[test]
    fn test_agent_profile_orchestrator() {
        let profile = AgentProfile::orchestrator();
        assert_eq!(profile.id, "orchestrator");
        assert_eq!(profile.role, ProfileRole::Orchestrator);
        assert_eq!(profile.execution.model, Model::Opus);
        assert_eq!(profile.execution.max_iterations, 50);
        assert!(profile.behavior.can_spawn_sub_agents);
    }

    #[test]
    fn test_agent_profile_deep_researcher() {
        let profile = AgentProfile::deep_researcher();
        assert_eq!(profile.id, "deep-researcher");
        assert_eq!(profile.role, ProfileRole::Researcher);
        assert_eq!(profile.execution.model, Model::Opus);
        assert_eq!(profile.execution.max_iterations, 200);
    }

    #[test]
    fn test_builtin_profiles() {
        let profiles = AgentProfile::builtin_profiles();
        assert_eq!(profiles.len(), 5);
        assert_eq!(profiles[0].id, "worker");
        assert_eq!(profiles[1].id, "reviewer");
        assert_eq!(profiles[2].id, "supervisor");
        assert_eq!(profiles[3].id, "orchestrator");
        assert_eq!(profiles[4].id, "deep-researcher");
    }

    #[test]
    fn test_agent_profile_serialize() {
        let profile = AgentProfile::worker();
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"id\":\"worker\""));
        assert!(json.contains("\"role\":\"worker\""));
    }

    #[test]
    fn test_agent_profile_deserialize() {
        let json = r#"{
            "id": "test",
            "name": "Test",
            "description": "Test agent",
            "role": "worker",
            "claudeCode": {
                "agent": "test",
                "skills": [],
                "mcpServers": []
            },
            "execution": {
                "model": "sonnet",
                "maxIterations": 30,
                "timeoutMinutes": 30,
                "permissionMode": "default"
            },
            "io": {
                "inputArtifactTypes": [],
                "outputArtifactTypes": []
            },
            "behavior": {
                "canSpawnSubAgents": false,
                "autoCommit": false,
                "autonomyLevel": "supervised"
            }
        }"#;

        let profile: AgentProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, "test");
        assert_eq!(profile.role, ProfileRole::Worker);
        assert_eq!(profile.execution.model, Model::Sonnet);
    }

    #[test]
    fn test_agent_profile_roundtrip() {
        let original = AgentProfile::orchestrator();
        let json = serde_json::to_string(&original).unwrap();
        let restored: AgentProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
