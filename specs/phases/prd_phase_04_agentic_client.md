# RalphX - Phase 4: Agentic Client

## Overview

This phase implements the **Agentic Client Abstraction Layer** - a trait-based architecture that decouples RalphX from any specific AI agent provider. The default implementation uses Claude Code CLI, but the abstraction allows swapping to Codex, Gemini, or other agentic clients in the future. This phase also includes a MockAgenticClient for cost-effective testing.

## Dependencies

- Phase 1 (Foundation) must be complete:
  - AppError and AppResult types
  - TaskId, ProjectId newtypes
  - Basic Rust project structure

- Phase 2 (Data Layer) must be complete:
  - Repository pattern established
  - AppState container for dependency injection

- Phase 3 (State Machine) must be complete:
  - AgentSpawner trait defined (stub implementation)
  - TaskServices container expecting agent_spawner

## Scope

### Included
- AgenticClient trait with all methods
- AgentConfig, AgentRole, ClientType enums
- ClientCapabilities struct
- AgentHandle, AgentOutput, AgentResponse, ResponseChunk types
- AgentError enum and AgentResult type alias
- ClaudeCodeClient implementation (uses `claude` CLI)
- MockAgenticClient for testing
- Integration with AppState
- Cost-optimized testing infrastructure
- Test prompt constants and markers

### Excluded
- Actual agent profiles/prompts (Phase 7)
- Worker/Reviewer/QA agent behavior (Phases 7-8)
- Future clients (Codex, Gemini) - placeholders only
- Configuration file parsing (simple hardcoded defaults for now)

## Detailed Requirements

### 1. Architecture Overview

From the master plan (lines 5066-5098):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DOMAIN LAYER                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    trait AgenticClient                               │   │
│  │  + spawn_agent(config) -> AgentHandle                                │   │
│  │  + send_prompt(handle, prompt) -> Response                           │   │
│  │  + stream_response(handle) -> Stream<Chunk>                          │   │
│  │  + stop_agent(handle)                                                │   │
│  │  + capabilities() -> ClientCapabilities                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────────┘
                                        │ implements
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       INFRASTRUCTURE LAYER                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │ ClaudeCodeClient│  │  CodexClient    │  │   GeminiClient              │ │
│  │ (default)       │  │  (future)       │  │   (future)                  │ │
│  │ - claude CLI    │  │ - codex CLI     │  │ - gemini CLI                │ │
│  │ - Agent SDK     │  │ - OpenAI API    │  │ - Google AI API             │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ MockAgenticClient (testing) - predefined responses, records calls   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Folder Structure

From the master plan (lines 5100-5118):

```
src-tauri/src/
├── domain/
│   └── agents/                 # Agent abstractions
│       ├── mod.rs
│       ├── agentic_client.rs   # trait AgenticClient
│       ├── agent_config.rs     # AgentConfig, AgentRole
│       ├── capabilities.rs     # ClientCapabilities
│       ├── error.rs            # AgentError, AgentResult
│       └── types.rs            # AgentHandle, AgentOutput, etc.
├── infrastructure/
│   └── agents/                 # Implementations
│       ├── mod.rs
│       ├── claude/
│       │   ├── mod.rs
│       │   └── claude_code_client.rs
│       └── mock/
│           ├── mod.rs
│           └── mock_client.rs
├── testing/                    # Test utilities
│   └── test_prompts.rs         # Minimal test prompts
```

### 3. Core Trait Definition

From the master plan (lines 5120-5157):

```rust
// src-tauri/src/domain/agents/agentic_client.rs

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Abstraction over agentic AI clients (Claude, Codex, Gemini, etc.)
#[async_trait]
pub trait AgenticClient: Send + Sync {
    /// Spawn a new agent with the given configuration
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle>;

    /// Stop a running agent
    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()>;

    /// Wait for an agent to complete
    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput>;

    /// Send a prompt and get a complete response
    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse>;

    /// Stream responses
    fn stream_response(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>>;

    /// Get client capabilities
    fn capabilities(&self) -> &ClientCapabilities;

    /// Check if client is available (CLI installed, API key set)
    async fn is_available(&self) -> AgentResult<bool>;
}
```

### 4. AgentConfig and AgentRole

From the master plan (lines 5158-5174):

```rust
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub prompt: String,
    pub working_directory: PathBuf,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRole {
    Worker,
    Reviewer,
    QaPrep,
    QaRefiner,
    QaTester,
    Supervisor,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientType {
    ClaudeCode,
    Codex,
    Gemini,
    Mock,
    Custom(String),
}
```

### 5. ClientCapabilities

From the master plan (lines 5176-5184):

```rust
#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub client_type: ClientType,
    pub supports_shell: bool,
    pub supports_filesystem: bool,
    pub supports_streaming: bool,
    pub supports_mcp: bool,
    pub max_context_tokens: u32,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub max_tokens: u32,
}
```

### 6. AgentHandle and Response Types

These types track spawned agents and capture their output:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentHandle {
    pub id: String,
    pub client_type: ClientType,
    pub role: AgentRole,
}

impl AgentHandle {
    pub fn new(client_type: ClientType, role: AgentRole) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            client_type,
            role,
        }
    }

    pub fn mock(role: AgentRole) -> Self {
        Self::new(ClientType::Mock, role)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentOutput {
    pub success: bool,
    pub content: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentResponse {
    pub content: String,
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
    pub is_final: bool,
}
```

### 7. Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Agent communication failed: {0}")]
    CommunicationFailed(String),

    #[error("Agent timeout after {0}ms")]
    Timeout(u64),

    #[error("CLI not available: {0}")]
    CliNotAvailable(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type AgentResult<T> = Result<T, AgentError>;
```

### 8. ClaudeCodeClient Implementation

From the master plan (lines 5187-5245):

```rust
// src-tauri/src/infrastructure/agents/claude/claude_code_client.rs

use tokio::process::Command;
use std::process::Stdio;

pub struct ClaudeCodeClient {
    cli_path: PathBuf,
    capabilities: ClientCapabilities,
}

impl ClaudeCodeClient {
    pub fn new() -> Self {
        Self {
            cli_path: which::which("claude").unwrap_or_else(|_| "claude".into()),
            capabilities: ClientCapabilities {
                client_type: ClientType::ClaudeCode,
                supports_shell: true,
                supports_filesystem: true,
                supports_streaming: true,
                supports_mcp: true,
                max_context_tokens: 200_000,
                models: vec![
                    ModelInfo {
                        id: "claude-sonnet-4-5-20250929".into(),
                        name: "Claude Sonnet 4.5".into(),
                        max_tokens: 64_000,
                    },
                    ModelInfo {
                        id: "claude-opus-4-5-20251101".into(),
                        name: "Claude Opus 4.5".into(),
                        max_tokens: 32_000,
                    },
                ],
            },
        }
    }

    pub fn with_cli_path(mut self, path: PathBuf) -> Self {
        self.cli_path = path;
        self
    }
}

#[async_trait]
impl AgenticClient for ClaudeCodeClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        let mut args = vec![
            "-p".into(),
            config.prompt.clone(),
            "--output-format".into(),
            "stream-json".into(),
        ];

        if let Some(model) = &config.model {
            args.extend(["--model".into(), model.clone()]);
        }

        let child = Command::new(&self.cli_path)
            .args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;

        let handle = AgentHandle::new(ClientType::ClaudeCode, config.role);
        // Store child process for later (use tokio::sync::Mutex for process tracking)
        PROCESSES.lock().await.insert(handle.id.clone(), child);

        Ok(handle)
    }

    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput> {
        let mut child = PROCESSES.lock().await
            .remove(&handle.id)
            .ok_or_else(|| AgentError::NotFound(handle.id.clone()))?;

        let output = child.wait_with_output().await?;

        Ok(AgentOutput {
            success: output.status.success(),
            content: String::from_utf8_lossy(&output.stdout).into(),
            exit_code: output.status.code(),
            duration_ms: None, // TODO: track start time
        })
    }

    // ... other methods
}

// Global process tracker (lazy_static or once_cell)
lazy_static::lazy_static! {
    static ref PROCESSES: tokio::sync::Mutex<HashMap<String, tokio::process::Child>> =
        tokio::sync::Mutex::new(HashMap::new());
}
```

### 9. MockAgenticClient Implementation

From the master plan (lines 5248-5285):

```rust
// src-tauri/src/infrastructure/agents/mock/mock_client.rs

pub struct MockAgenticClient {
    responses: Arc<RwLock<HashMap<String, String>>>,
    call_history: Arc<RwLock<Vec<MockCall>>>,
    capabilities: ClientCapabilities,
}

#[derive(Debug, Clone)]
pub struct MockCall {
    pub call_type: MockCallType,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum MockCallType {
    Spawn { role: AgentRole, prompt: String },
    Stop { handle_id: String },
    SendPrompt { handle_id: String, prompt: String },
}

impl MockAgenticClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            call_history: Arc::new(RwLock::new(Vec::new())),
            capabilities: ClientCapabilities {
                client_type: ClientType::Mock,
                supports_shell: true,
                supports_filesystem: true,
                supports_streaming: true,
                supports_mcp: false,
                max_context_tokens: 200_000,
                models: vec![ModelInfo {
                    id: "mock".into(),
                    name: "Mock Model".into(),
                    max_tokens: 100_000,
                }],
            },
        }
    }

    /// Set response for prompts containing pattern
    pub async fn when_prompt_contains(&self, pattern: &str, response: &str) {
        self.responses.write().await.insert(pattern.into(), response.into());
    }

    /// Get recorded calls for assertions
    pub async fn get_calls(&self) -> Vec<MockCall> {
        self.call_history.read().await.clone()
    }

    /// Clear call history
    pub async fn clear_calls(&self) {
        self.call_history.write().await.clear();
    }

    async fn find_matching_response(&self, prompt: &str) -> String {
        let responses = self.responses.read().await;
        for (pattern, response) in responses.iter() {
            if prompt.contains(pattern) {
                return response.clone();
            }
        }
        "MOCK_DEFAULT_RESPONSE".to_string()
    }
}

#[async_trait]
impl AgenticClient for MockAgenticClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        self.call_history.write().await.push(MockCall {
            call_type: MockCallType::Spawn {
                role: config.role.clone(),
                prompt: config.prompt.clone(),
            },
            timestamp: std::time::Instant::now(),
        });
        Ok(AgentHandle::mock(config.role))
    }

    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
        self.call_history.write().await.push(MockCall {
            call_type: MockCallType::SendPrompt {
                handle_id: handle.id.clone(),
                prompt: prompt.to_string(),
            },
            timestamp: std::time::Instant::now(),
        });

        let response = self.find_matching_response(prompt).await;
        Ok(AgentResponse {
            content: response,
            model: Some("mock".into()),
            tokens_used: Some(10),
        })
    }

    async fn wait_for_completion(&self, _handle: &AgentHandle) -> AgentResult<AgentOutput> {
        Ok(AgentOutput {
            success: true,
            content: "MOCK_COMPLETION".into(),
            exit_code: Some(0),
            duration_ms: Some(100),
        })
    }

    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()> {
        self.call_history.write().await.push(MockCall {
            call_type: MockCallType::Stop {
                handle_id: handle.id.clone(),
            },
            timestamp: std::time::Instant::now(),
        });
        Ok(())
    }

    fn stream_response(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>> {
        // Return a simple stream with mock chunks
        let chunks = vec![
            ResponseChunk { content: "MOCK_".into(), is_final: false },
            ResponseChunk { content: "STREAM".into(), is_final: true },
        ];
        Box::pin(futures::stream::iter(chunks.into_iter().map(Ok)))
    }

    fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> AgentResult<bool> {
        Ok(true) // Mock is always available
    }
}
```

### 10. Updated AppState

From the master plan (lines 5288-5323):

```rust
// Update src-tauri/src/application/app_state.rs

pub struct AppState {
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub agent_client: Arc<dyn AgenticClient>,  // Added
}

impl AppState {
    /// Production: SQLite + Claude Code (default)
    pub fn new_production(db_path: &str) -> Self {
        Self {
            project_repo: Arc::new(SqliteProjectRepository::new(db_path)),
            task_repo: Arc::new(SqliteTaskRepository::new(db_path)),
            agent_client: Arc::new(ClaudeCodeClient::new()),
        }
    }

    /// Testing: In-memory + Mock agent (no API calls)
    pub fn new_test() -> Self {
        Self {
            project_repo: Arc::new(MemoryProjectRepository::new()),
            task_repo: Arc::new(MemoryTaskRepository::new()),
            agent_client: Arc::new(MockAgenticClient::new()),
        }
    }

    /// Swap to different provider
    pub fn with_agent_client(mut self, client: Arc<dyn AgenticClient>) -> Self {
        self.agent_client = client;
        self
    }
}
```

### 11. Cost-Optimized Test Patterns

From the master plan (lines 3162-3391):

**Critical**: Integration tests that spawn Claude agents MUST use minimal prompts to avoid expensive API costs.

```rust
// src-tauri/src/testing/test_prompts.rs

pub mod test_prompts {
    /// Minimal prompt that verifies agent received input and can respond
    pub const ECHO_MARKER: &str = "Respond with exactly: TEST_ECHO_OK";

    /// Minimal prompt for testing worker agent spawning
    pub const WORKER_SPAWN_TEST: &str =
        "Respond with exactly: WORKER_SPAWNED_SUCCESSFULLY";

    /// Minimal prompt for testing QA prep agent
    pub const QA_PREP_TEST: &str =
        "Respond with exactly: QA_PREP_COMPLETE";

    /// Minimal prompt for testing reviewer agent
    pub const REVIEWER_TEST: &str =
        "Respond with exactly: REVIEW_COMPLETE_APPROVED";

    /// Minimal prompt for loop iteration testing
    pub fn iteration_test_prompt(n: u32) -> String {
        format!("Respond with exactly: ITERATION_{}_COMPLETE", n)
    }

    /// Verify expected marker in output
    pub fn assert_marker(output: &str, marker: &str) {
        assert!(
            output.contains(marker),
            "Expected output to contain '{}', got: {}",
            marker,
            &output[..output.len().min(200)]
        );
    }
}
```

**Cost Estimation:**

| Test Type | Real Prompts (est.) | Minimal Prompts (est.) | Savings |
|-----------|---------------------|------------------------|---------|
| Agent spawn (1 test) | ~$0.05 | ~$0.001 | 98% |
| Loop 3 iterations | ~$0.30 | ~$0.005 | 98% |
| Full integration suite (50 tests) | ~$5.00 | ~$0.10 | 98% |

### 12. Bridge to State Machine

The AgenticClient connects to the state machine's AgentSpawner trait from Phase 3:

```rust
// Implement AgentSpawner using AgenticClient
pub struct AgenticClientSpawner {
    client: Arc<dyn AgenticClient>,
}

#[async_trait]
impl AgentSpawner for AgenticClientSpawner {
    async fn spawn(&self, agent_type: &str, task_id: &str) {
        let role = match agent_type {
            "worker" => AgentRole::Worker,
            "qa-prep" => AgentRole::QaPrep,
            "qa-refiner" => AgentRole::QaRefiner,
            "qa-tester" => AgentRole::QaTester,
            "reviewer" => AgentRole::Reviewer,
            _ => AgentRole::Custom(agent_type.to_string()),
        };

        let config = AgentConfig {
            role,
            prompt: format!("Execute task {}", task_id),
            working_directory: std::env::current_dir().unwrap(),
            ..Default::default()
        };

        let _ = self.client.spawn_agent(config).await;
    }

    async fn spawn_background(&self, agent_type: &str, task_id: &str) {
        // Same as spawn but don't await completion
        self.spawn(agent_type, task_id).await;
    }

    async fn wait_for(&self, _agent_type: &str, _task_id: &str) {
        // TODO: Implement handle tracking to wait for specific agent
    }
}
```

## Implementation Notes

### Cargo Dependencies

```toml
[dependencies]
async-trait = "0.1"
tokio = { version = "1", features = ["full", "process"] }
futures = "0.3"
which = "6.0"           # For finding CLI paths
lazy_static = "1.4"     # For global process tracker
uuid = { version = "1", features = ["v4"] }
thiserror = "1.0"
```

### Testing Strategy

1. **Unit tests**: Mock responses, verify call recording
2. **Integration tests**: Use minimal test prompts (cost-optimized)
3. **Availability check**: Test is_available() for CLI detection
4. **Stream handling**: Test streaming with mock chunks

### CLI Detection

The ClaudeCodeClient should gracefully handle missing CLI:
- `is_available()` returns false if CLI not found
- `spawn_agent()` returns `CliNotAvailable` error if CLI missing

## Task List

```json
[
  {
    "category": "setup",
    "description": "Add agent client dependencies to Cargo.toml",
    "steps": [
      "Write test that imports async_trait, futures, which",
      "Add async-trait = \"0.1\" to Cargo.toml",
      "Add futures = \"0.3\" to Cargo.toml",
      "Add which = \"6.0\" to Cargo.toml",
      "Add lazy_static = \"1.4\" to Cargo.toml",
      "Verify tokio has process feature enabled",
      "Run cargo build to verify dependencies resolve"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgentError enum and AgentResult type",
    "steps": [
      "Write tests for each AgentError variant conversion",
      "Create src-tauri/src/domain/agents/mod.rs",
      "Create src-tauri/src/domain/agents/error.rs",
      "Implement AgentError with NotFound, SpawnFailed, CommunicationFailed, Timeout, CliNotAvailable, Io",
      "Add From<std::io::Error> implementation",
      "Define AgentResult<T> type alias",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgentRole and ClientType enums",
    "steps": [
      "Write tests for AgentRole variants",
      "Write tests for ClientType variants",
      "Create src-tauri/src/domain/agents/types.rs",
      "Implement AgentRole with Worker, Reviewer, QaPrep, QaRefiner, QaTester, Supervisor, Custom",
      "Implement ClientType with ClaudeCode, Codex, Gemini, Mock, Custom",
      "Derive Debug, Clone, PartialEq, Eq for both",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgentConfig struct",
    "steps": [
      "Write tests for AgentConfig creation and defaults",
      "Add AgentConfig to types.rs or create agent_config.rs",
      "Implement fields: role, prompt, working_directory, model, max_tokens, timeout_secs, env",
      "Implement Default trait with sensible defaults",
      "Derive Debug, Clone",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create ModelInfo and ClientCapabilities structs",
    "steps": [
      "Write tests for ClientCapabilities field access",
      "Create src-tauri/src/domain/agents/capabilities.rs",
      "Implement ModelInfo { id, name, max_tokens }",
      "Implement ClientCapabilities with client_type, supports_shell, supports_filesystem, supports_streaming, supports_mcp, max_context_tokens, models",
      "Derive Debug, Clone",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgentHandle struct",
    "steps": [
      "Write tests for AgentHandle::new and AgentHandle::mock",
      "Add AgentHandle to types.rs",
      "Implement AgentHandle { id, client_type, role }",
      "Add new(client_type, role) constructor with UUID generation",
      "Add mock(role) convenience constructor",
      "Derive Debug, Clone, PartialEq, Eq, Hash",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgentOutput, AgentResponse, ResponseChunk structs",
    "steps": [
      "Write tests for default values of each struct",
      "Add AgentOutput { success, content, exit_code, duration_ms } with Default",
      "Add AgentResponse { content, model, tokens_used } with Default",
      "Add ResponseChunk { content, is_final }",
      "Derive Debug, Clone, Default where appropriate",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Define AgenticClient trait",
    "steps": [
      "Create src-tauri/src/domain/agents/agentic_client.rs",
      "Import async_trait, futures::Stream, std::pin::Pin",
      "Define trait with spawn_agent, stop_agent, wait_for_completion methods",
      "Add send_prompt, stream_response methods",
      "Add capabilities, is_available methods",
      "Mark trait with #[async_trait] and Send + Sync bounds",
      "Run cargo build to verify trait compiles"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement MockAgenticClient",
    "steps": [
      "Write tests for spawn_agent recording calls",
      "Write tests for when_prompt_contains setting responses",
      "Write tests for get_calls and clear_calls",
      "Create src-tauri/src/infrastructure/agents/mock/mod.rs",
      "Create mock_client.rs with MockAgenticClient struct",
      "Implement MockCall and MockCallType for call tracking",
      "Implement all AgenticClient trait methods",
      "Use Arc<RwLock<...>> for thread-safe response/call storage",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - CLI detection and capabilities",
    "steps": [
      "Write tests for ClaudeCodeClient::new()",
      "Write tests for with_cli_path builder",
      "Write tests for capabilities() returning correct values",
      "Create src-tauri/src/infrastructure/agents/claude/mod.rs",
      "Create claude_code_client.rs with ClaudeCodeClient struct",
      "Use which::which(\"claude\") for CLI path detection",
      "Define capabilities with supports_shell, supports_mcp, etc.",
      "Add models: claude-sonnet-4-5-20250929 (Sonnet 4.5), claude-opus-4-5-20251101 (Opus 4.5), claude-haiku-4-5-20251001 (Haiku 4.5)",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - is_available method",
    "steps": [
      "Write tests for is_available when CLI exists",
      "Write tests for is_available when CLI is missing",
      "Implement is_available() checking if CLI path exists",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - spawn_agent method",
    "steps": [
      "Write tests for spawn_agent returning handle",
      "Add global PROCESSES tracker using lazy_static with tokio::sync::Mutex",
      "Implement spawn_agent using tokio::process::Command",
      "Build args with -p, --output-format stream-json, optional --model",
      "Spawn child process and store in PROCESSES",
      "Return AgentHandle with new UUID",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - stop_agent method",
    "steps": [
      "Write tests for stop_agent removing process from tracker",
      "Implement stop_agent to kill child process",
      "Remove handle from PROCESSES on stop",
      "Handle NotFound error for missing handles",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - wait_for_completion method",
    "steps": [
      "Write tests for wait_for_completion returning output",
      "Implement wait_for_completion using child.wait_with_output()",
      "Extract stdout content into AgentOutput",
      "Set success based on exit status",
      "Handle NotFound for missing handles",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - send_prompt method",
    "steps": [
      "Write tests for send_prompt with mock (defer real test to integration)",
      "Implement send_prompt spawning new agent with prompt",
      "Wait for completion and return AgentResponse",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement ClaudeCodeClient - stream_response method",
    "steps": [
      "Write tests for stream_response returning chunks",
      "Implement stream_response reading stdout line by line",
      "Parse stream-json output into ResponseChunk",
      "Return Pin<Box<dyn Stream<...>>>",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create test_prompts module with minimal test prompts",
    "steps": [
      "Create src-tauri/src/testing/mod.rs",
      "Create src-tauri/src/testing/test_prompts.rs",
      "Add ECHO_MARKER, WORKER_SPAWN_TEST, QA_PREP_TEST, REVIEWER_TEST constants",
      "Add iteration_test_prompt(n) function",
      "Add assert_marker(output, marker) helper",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create AgenticClientSpawner bridging to state machine",
    "steps": [
      "Write tests for AgenticClientSpawner implementing AgentSpawner trait",
      "Create src-tauri/src/infrastructure/agents/spawner.rs",
      "Implement AgenticClientSpawner wrapping Arc<dyn AgenticClient>",
      "Map agent_type strings to AgentRole enum",
      "Implement spawn, spawn_background, wait_for methods",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Update AppState to include agent_client",
    "steps": [
      "Write tests for AppState::new_production including agent_client",
      "Write tests for AppState::new_test using MockAgenticClient",
      "Write tests for with_agent_client builder",
      "Add agent_client: Arc<dyn AgenticClient> field to AppState",
      "Update new_production to use ClaudeCodeClient",
      "Update new_test to use MockAgenticClient",
      "Add with_agent_client method",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create integration test for MockAgenticClient",
    "steps": [
      "Write test spawning worker agent with mock",
      "Write test verifying when_prompt_contains works",
      "Write test verifying call history is recorded",
      "Verify stream_response returns mock chunks",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create integration test for ClaudeCodeClient availability",
    "steps": [
      "Write test that is_available returns correctly based on CLI presence",
      "Write test that spawn_agent fails gracefully when CLI missing",
      "Use #[ignore] for tests that require real CLI (run manually)",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create cost-optimized integration test for real agent spawn",
    "steps": [
      "Write #[ignore] test for spawning real Claude agent",
      "Use ECHO_MARKER prompt for minimal cost",
      "Verify output contains expected marker",
      "Document cost estimate in test comment",
      "Run with cargo test -- --ignored when ready"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Export agents module from domain and infrastructure layers",
    "steps": [
      "Add pub mod agents to src-tauri/src/domain/mod.rs",
      "Add pub mod agents to src-tauri/src/infrastructure/mod.rs",
      "Add pub mod testing to src-tauri/src/lib.rs",
      "Re-export key types: AgenticClient, AgentConfig, AgentRole, AgentHandle, etc.",
      "Re-export implementations: ClaudeCodeClient, MockAgenticClient",
      "Re-export test utilities: test_prompts",
      "Run cargo build"
    ],
    "passes": false
  }
]
```
