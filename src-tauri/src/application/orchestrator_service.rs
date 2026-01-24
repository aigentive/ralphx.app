// Orchestrator Service
// Connects the Orchestrator agent to the ideation chat system.
// Invokes claude CLI and streams responses back to the UI.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::domain::entities::{
    ChatMessage, IdeationSessionId, MessageRole, Priority, TaskCategory, TaskProposal,
    TaskProposalId,
};
use crate::domain::repositories::{ChatMessageRepository, TaskProposalRepository};

// ============================================================================
// Types
// ============================================================================

/// Tool call from the orchestrator agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Parsed stream-json message from Claude CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    #[serde(rename = "content_block_start")]
    ContentBlockStart { content_block: ContentBlock },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_start")]
    MessageStart,
    #[serde(rename = "message_delta")]
    MessageDelta,
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
    pub partial_json: Option<String>,
}

/// Result from orchestrator processing
#[derive(Debug, Clone)]
pub struct OrchestratorResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCallResult>,
    pub proposals_created: Vec<TaskProposal>,
}

/// Result of executing a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Event emitted during orchestrator processing
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// Text chunk received from agent
    TextChunk(String),
    /// Tool call detected
    ToolCallDetected(ToolCall),
    /// Tool call completed
    ToolCallCompleted(ToolCallResult),
    /// Proposal created
    ProposalCreated(TaskProposal),
    /// Processing complete
    Complete(OrchestratorResult),
    /// Error occurred
    Error(String),
}

// ============================================================================
// OrchestratorService trait
// ============================================================================

#[async_trait]
pub trait OrchestratorService: Send + Sync {
    /// Send a user message to the orchestrator and process the response
    async fn send_message(
        &self,
        session_id: &IdeationSessionId,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError>;

    /// Send a message with event streaming
    fn send_message_streaming(
        &self,
        session_id: IdeationSessionId,
        user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent>;

    /// Check if the orchestrator agent is available
    async fn is_available(&self) -> bool;
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone)]
pub enum OrchestratorError {
    AgentNotAvailable(String),
    SpawnFailed(String),
    CommunicationFailed(String),
    ToolCallFailed(String),
    ParseError(String),
    SessionNotFound(String),
    RepositoryError(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotAvailable(msg) => write!(f, "Agent not available: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn agent: {}", msg),
            Self::CommunicationFailed(msg) => write!(f, "Communication failed: {}", msg),
            Self::ToolCallFailed(msg) => write!(f, "Tool call failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
            Self::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for OrchestratorError {}

// ============================================================================
// ClaudeOrchestratorService - Production implementation
// ============================================================================

/// Production orchestrator service using Claude CLI
pub struct ClaudeOrchestratorService {
    cli_path: PathBuf,
    agent_definition_path: PathBuf,
    working_directory: PathBuf,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    proposal_repo: Arc<dyn TaskProposalRepository>,
    model: String,
}

impl ClaudeOrchestratorService {
    pub fn new(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        proposal_repo: Arc<dyn TaskProposalRepository>,
    ) -> Self {
        let cli_path = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));
        let working_directory =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let agent_definition_path = working_directory.join(".claude/agents/orchestrator-ideation.md");

        Self {
            cli_path,
            agent_definition_path,
            working_directory,
            chat_message_repo,
            proposal_repo,
            model: "sonnet".to_string(),
        }
    }

    pub fn with_cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cli_path = path.into();
        self
    }

    pub fn with_agent_definition(mut self, path: impl Into<PathBuf>) -> Self {
        self.agent_definition_path = path.into();
        self
    }

    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the conversation history prompt from stored messages
    async fn build_conversation_history(
        &self,
        session_id: &IdeationSessionId,
    ) -> Result<String, OrchestratorError> {
        let messages = self
            .chat_message_repo
            .get_by_session(session_id)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        let mut history = String::new();
        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Orchestrator => "Assistant",
                MessageRole::System => "System",
            };
            history.push_str(&format!("{}: {}\n\n", role, msg.content));
        }

        Ok(history)
    }

    /// Parse a stream-json line from Claude CLI output
    fn parse_stream_line(line: &str) -> Option<StreamMessage> {
        serde_json::from_str(line).ok()
    }

    /// Execute a tool call and return the result
    async fn execute_tool_call(
        &self,
        session_id: &IdeationSessionId,
        tool_call: &ToolCall,
    ) -> ToolCallResult {
        match tool_call.name.as_str() {
            "create_task_proposal" => {
                self.handle_create_task_proposal(session_id, &tool_call.arguments)
                    .await
            }
            "update_task_proposal" => {
                self.handle_update_task_proposal(&tool_call.arguments).await
            }
            "delete_task_proposal" => {
                self.handle_delete_task_proposal(&tool_call.arguments).await
            }
            _ => ToolCallResult {
                tool_name: tool_call.name.clone(),
                success: false,
                result: None,
                error: Some(format!("Unknown tool: {}", tool_call.name)),
            },
        }
    }

    async fn handle_create_task_proposal(
        &self,
        session_id: &IdeationSessionId,
        args: &serde_json::Value,
    ) -> ToolCallResult {
        let tool_name = "create_task_proposal".to_string();

        // Extract arguments
        let title = match args.get("title").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                return ToolCallResult {
                    tool_name,
                    success: false,
                    result: None,
                    error: Some("Missing required field: title".to_string()),
                }
            }
        };

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let category_str = args
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("feature");
        let category: TaskCategory = category_str.parse().unwrap_or(TaskCategory::Feature);

        let priority_str = args
            .get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
        let priority: Priority = priority_str.parse().unwrap_or(Priority::Medium);

        let priority_score = args
            .get("priority_score")
            .and_then(|v| v.as_i64())
            .unwrap_or(50) as i32;

        let priority_reason = args
            .get("priority_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let steps = args
            .get("steps")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            });

        let acceptance_criteria = args
            .get("acceptance_criteria")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            });

        // Create the proposal
        let mut proposal = TaskProposal::new(session_id.clone(), title, category, priority);
        proposal.description = description;
        proposal.priority_score = priority_score;
        proposal.priority_reason = priority_reason;
        if let Some(s) = steps {
            proposal.steps = Some(serde_json::to_string(&s).unwrap_or_default());
        }
        if let Some(ac) = acceptance_criteria {
            proposal.acceptance_criteria = Some(serde_json::to_string(&ac).unwrap_or_default());
        }

        match self.proposal_repo.create(proposal.clone()).await {
            Ok(created) => ToolCallResult {
                tool_name,
                success: true,
                result: Some(serde_json::json!({
                    "id": created.id.to_string(),
                    "title": created.title,
                    "category": created.category.to_string(),
                    "priority": created.suggested_priority.to_string(),
                })),
                error: None,
            },
            Err(e) => ToolCallResult {
                tool_name,
                success: false,
                result: None,
                error: Some(format!("Failed to create proposal: {}", e)),
            },
        }
    }

    async fn handle_update_task_proposal(&self, args: &serde_json::Value) -> ToolCallResult {
        let tool_name = "update_task_proposal".to_string();

        let proposal_id = match args.get("proposal_id").and_then(|v| v.as_str()) {
            Some(id) => TaskProposalId::from_string(id.to_string()),
            None => {
                return ToolCallResult {
                    tool_name,
                    success: false,
                    result: None,
                    error: Some("Missing required field: proposal_id".to_string()),
                }
            }
        };

        // Get existing proposal
        let proposal = match self.proposal_repo.get_by_id(&proposal_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return ToolCallResult {
                    tool_name,
                    success: false,
                    result: None,
                    error: Some("Proposal not found".to_string()),
                }
            }
            Err(e) => {
                return ToolCallResult {
                    tool_name,
                    success: false,
                    result: None,
                    error: Some(format!("Failed to get proposal: {}", e)),
                }
            }
        };

        // Build updated proposal
        let mut updated = proposal;

        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            updated.title = title.to_string();
        }
        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
            updated.description = Some(desc.to_string());
        }
        if let Some(cat) = args.get("category").and_then(|v| v.as_str()) {
            if let Ok(c) = cat.parse() {
                updated.category = c;
            }
        }
        if let Some(pri) = args.get("priority").and_then(|v| v.as_str()) {
            if let Ok(p) = pri.parse() {
                updated.suggested_priority = p;
            }
        }
        if let Some(score) = args.get("priority_score").and_then(|v| v.as_i64()) {
            updated.priority_score = score as i32;
        }

        let proposal_id_str = updated.id.to_string();
        let proposal_title = updated.title.clone();
        match self.proposal_repo.update(&updated).await {
            Ok(()) => ToolCallResult {
                tool_name,
                success: true,
                result: Some(serde_json::json!({
                    "id": proposal_id_str,
                    "title": proposal_title,
                })),
                error: None,
            },
            Err(e) => ToolCallResult {
                tool_name,
                success: false,
                result: None,
                error: Some(format!("Failed to update proposal: {}", e)),
            },
        }
    }

    async fn handle_delete_task_proposal(&self, args: &serde_json::Value) -> ToolCallResult {
        let tool_name = "delete_task_proposal".to_string();

        let proposal_id = match args.get("proposal_id").and_then(|v| v.as_str()) {
            Some(id) => TaskProposalId::from_string(id.to_string()),
            None => {
                return ToolCallResult {
                    tool_name,
                    success: false,
                    result: None,
                    error: Some("Missing required field: proposal_id".to_string()),
                }
            }
        };

        match self.proposal_repo.delete(&proposal_id).await {
            Ok(()) => ToolCallResult {
                tool_name,
                success: true,
                result: Some(serde_json::json!({ "deleted": true })),
                error: None,
            },
            Err(e) => ToolCallResult {
                tool_name,
                success: false,
                result: None,
                error: Some(format!("Failed to delete proposal: {}", e)),
            },
        }
    }
}

#[async_trait]
impl OrchestratorService for ClaudeOrchestratorService {
    async fn send_message(
        &self,
        session_id: &IdeationSessionId,
        user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        // First, store the user message
        let user_msg = ChatMessage::user_in_session(session_id.clone(), user_message);
        self.chat_message_repo
            .create(user_msg)
            .await
            .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;

        // Build conversation history
        let history = self.build_conversation_history(session_id).await?;

        // Build the full prompt
        let full_prompt = format!(
            "You are the Ideation Orchestrator. You help users brainstorm and create task proposals.\n\n\
             Previous conversation:\n{}\n\n\
             User's new message: {}\n\n\
             Please respond helpfully. If you identify tasks, use the create_task_proposal tool.",
            history, user_message
        );

        // Spawn the claude CLI
        let mut cmd = Command::new(&self.cli_path);
        cmd.args([
            "-p",
            &full_prompt,
            "--output-format",
            "stream-json",
            "--model",
            &self.model,
        ])
        .current_dir(&self.working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| OrchestratorError::SpawnFailed(e.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| OrchestratorError::SpawnFailed("Failed to capture stdout".to_string()))?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut response_text = String::new();
        let mut tool_calls = Vec::new();
        let mut proposals_created = Vec::new();
        let mut current_tool_name = String::new();
        let mut current_tool_input = String::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| OrchestratorError::CommunicationFailed(e.to_string()))?
        {
            if let Some(msg) = Self::parse_stream_line(&line) {
                match msg {
                    StreamMessage::ContentBlockStart { content_block } => {
                        if content_block.block_type == "tool_use" {
                            current_tool_name =
                                content_block.name.unwrap_or_default();
                            current_tool_input.clear();
                        }
                    }
                    StreamMessage::ContentBlockDelta { delta } => {
                        if delta.delta_type == "text_delta" {
                            if let Some(text) = delta.text {
                                response_text.push_str(&text);
                            }
                        } else if delta.delta_type == "input_json_delta" {
                            if let Some(json) = delta.partial_json {
                                current_tool_input.push_str(&json);
                            }
                        }
                    }
                    StreamMessage::ContentBlockStop => {
                        if !current_tool_name.is_empty() {
                            // Parse and execute tool call
                            let args: serde_json::Value =
                                serde_json::from_str(&current_tool_input).unwrap_or_default();
                            let tool_call = ToolCall {
                                name: current_tool_name.clone(),
                                arguments: args,
                            };
                            let result = self.execute_tool_call(session_id, &tool_call).await;

                            // If it was a create_task_proposal, track the proposal
                            if result.success && tool_call.name == "create_task_proposal" {
                                if let Some(ref res) = result.result {
                                    if let Some(id) = res.get("id").and_then(|v| v.as_str()) {
                                        let proposal_id = TaskProposalId::from_string(id.to_string());
                                        if let Ok(Some(p)) =
                                            self.proposal_repo.get_by_id(&proposal_id).await
                                        {
                                            proposals_created.push(p);
                                        }
                                    }
                                }
                            }

                            tool_calls.push(result);
                            current_tool_name.clear();
                            current_tool_input.clear();
                        }
                    }
                    _ => {}
                }
            }
        }

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| OrchestratorError::CommunicationFailed(e.to_string()))?;

        if !status.success() && response_text.is_empty() {
            return Err(OrchestratorError::CommunicationFailed(
                "Agent exited with non-zero status".to_string(),
            ));
        }

        // Store the orchestrator's response
        if !response_text.is_empty() {
            let orchestrator_msg =
                ChatMessage::orchestrator_in_session(session_id.clone(), &response_text);
            self.chat_message_repo
                .create(orchestrator_msg)
                .await
                .map_err(|e| OrchestratorError::RepositoryError(e.to_string()))?;
        }

        Ok(OrchestratorResult {
            response_text,
            tool_calls,
            proposals_created,
        })
    }

    fn send_message_streaming(
        &self,
        session_id: IdeationSessionId,
        user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent> {
        let (tx, rx) = mpsc::channel(100);

        let cli_path = self.cli_path.clone();
        let working_directory = self.working_directory.clone();
        let model = self.model.clone();
        let chat_repo = Arc::clone(&self.chat_message_repo);
        let _proposal_repo = Arc::clone(&self.proposal_repo);

        tokio::spawn(async move {
            // Store user message
            let user_msg = ChatMessage::user_in_session(session_id.clone(), &user_message);
            if let Err(e) = chat_repo.create(user_msg).await {
                let _ = tx.send(OrchestratorEvent::Error(e.to_string())).await;
                return;
            }

            // Build the full prompt (simplified - in production would get history)
            let full_prompt = format!(
                "You are the Ideation Orchestrator. Respond helpfully.\n\nUser: {}",
                user_message
            );

            // Spawn CLI
            let mut cmd = Command::new(&cli_path);
            cmd.args([
                "-p",
                &full_prompt,
                "--output-format",
                "stream-json",
                "--model",
                &model,
            ])
            .current_dir(&working_directory)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(OrchestratorEvent::Error(e.to_string())).await;
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = tx
                        .send(OrchestratorEvent::Error(
                            "Failed to capture stdout".to_string(),
                        ))
                        .await;
                    return;
                }
            };

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            let mut response_text = String::new();
            let mut tool_calls = Vec::new();
            let proposals_created = Vec::new();
            let mut current_tool_name = String::new();
            let mut current_tool_input = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(msg) = ClaudeOrchestratorService::parse_stream_line(&line) {
                    match msg {
                        StreamMessage::ContentBlockStart { content_block } => {
                            if content_block.block_type == "tool_use" {
                                current_tool_name = content_block.name.unwrap_or_default();
                                current_tool_input.clear();
                            }
                        }
                        StreamMessage::ContentBlockDelta { delta } => {
                            if delta.delta_type == "text_delta" {
                                if let Some(text) = delta.text {
                                    response_text.push_str(&text);
                                    let _ = tx.send(OrchestratorEvent::TextChunk(text)).await;
                                }
                            } else if delta.delta_type == "input_json_delta" {
                                if let Some(json) = delta.partial_json {
                                    current_tool_input.push_str(&json);
                                }
                            }
                        }
                        StreamMessage::ContentBlockStop => {
                            if !current_tool_name.is_empty() {
                                let args: serde_json::Value =
                                    serde_json::from_str(&current_tool_input).unwrap_or_default();
                                let tool_call = ToolCall {
                                    name: current_tool_name.clone(),
                                    arguments: args,
                                };

                                let _ = tx
                                    .send(OrchestratorEvent::ToolCallDetected(tool_call.clone()))
                                    .await;

                                // Execute tool call (simplified - would need full service access)
                                let result = if tool_call.name == "create_task_proposal" {
                                    // Would call handle_create_task_proposal here
                                    ToolCallResult {
                                        tool_name: tool_call.name.clone(),
                                        success: true,
                                        result: Some(serde_json::json!({"id": "mock"})),
                                        error: None,
                                    }
                                } else {
                                    ToolCallResult {
                                        tool_name: tool_call.name.clone(),
                                        success: false,
                                        result: None,
                                        error: Some("Unsupported in streaming mode".to_string()),
                                    }
                                };

                                let _ = tx
                                    .send(OrchestratorEvent::ToolCallCompleted(result.clone()))
                                    .await;
                                tool_calls.push(result);

                                current_tool_name.clear();
                                current_tool_input.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Wait for process
            let _ = child.wait().await;

            // Store response
            if !response_text.is_empty() {
                let orchestrator_msg =
                    ChatMessage::orchestrator_in_session(session_id.clone(), &response_text);
                let _ = chat_repo.create(orchestrator_msg).await;
            }

            let _ = tx
                .send(OrchestratorEvent::Complete(OrchestratorResult {
                    response_text,
                    tool_calls,
                    proposals_created,
                }))
                .await;
        });

        rx
    }

    async fn is_available(&self) -> bool {
        if self.cli_path.exists() {
            return true;
        }
        which::which(&self.cli_path).is_ok()
    }
}

// ============================================================================
// MockOrchestratorService - For testing
// ============================================================================

/// Mock orchestrator service for testing
pub struct MockOrchestratorService {
    responses: Mutex<Vec<MockResponse>>,
    is_available: Mutex<bool>,
}

pub struct MockResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub proposals: Vec<TaskProposal>,
}

impl MockOrchestratorService {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            is_available: Mutex::new(true),
        }
    }

    pub async fn set_available(&self, available: bool) {
        *self.is_available.lock().await = available;
    }

    pub async fn queue_response(&self, response: MockResponse) {
        self.responses.lock().await.push(response);
    }

    pub async fn queue_text_response(&self, text: impl Into<String>) {
        self.queue_response(MockResponse {
            text: text.into(),
            tool_calls: Vec::new(),
            proposals: Vec::new(),
        })
        .await;
    }
}

impl Default for MockOrchestratorService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrchestratorService for MockOrchestratorService {
    async fn send_message(
        &self,
        _session_id: &IdeationSessionId,
        _user_message: &str,
    ) -> Result<OrchestratorResult, OrchestratorError> {
        if !*self.is_available.lock().await {
            return Err(OrchestratorError::AgentNotAvailable(
                "Mock agent not available".to_string(),
            ));
        }

        let mut responses = self.responses.lock().await;
        if let Some(response) = responses.pop() {
            Ok(OrchestratorResult {
                response_text: response.text,
                tool_calls: response
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCallResult {
                        tool_name: tc.name.clone(),
                        success: true,
                        result: Some(tc.arguments.clone()),
                        error: None,
                    })
                    .collect(),
                proposals_created: response.proposals,
            })
        } else {
            Ok(OrchestratorResult {
                response_text: "I'm here to help with your ideation session.".to_string(),
                tool_calls: Vec::new(),
                proposals_created: Vec::new(),
            })
        }
    }

    fn send_message_streaming(
        &self,
        _session_id: IdeationSessionId,
        _user_message: String,
    ) -> mpsc::Receiver<OrchestratorEvent> {
        let (tx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let _ = tx
                .send(OrchestratorEvent::TextChunk(
                    "Mock streaming response".to_string(),
                ))
                .await;
            let _ = tx
                .send(OrchestratorEvent::Complete(OrchestratorResult {
                    response_text: "Mock streaming response".to_string(),
                    tool_calls: Vec::new(),
                    proposals_created: Vec::new(),
                }))
                .await;
        });

        rx
    }

    async fn is_available(&self) -> bool {
        *self.is_available.lock().await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_service_default_response() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        let result = service.send_message(&session_id, "Hello").await.unwrap();

        assert!(result.response_text.contains("help"));
        assert!(result.tool_calls.is_empty());
        assert!(result.proposals_created.is_empty());
    }

    #[tokio::test]
    async fn test_mock_service_queued_response() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        service.queue_text_response("Custom response").await;

        let result = service.send_message(&session_id, "Hello").await.unwrap();

        assert_eq!(result.response_text, "Custom response");
    }

    #[tokio::test]
    async fn test_mock_service_not_available() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        service.set_available(false).await;

        let result = service.send_message(&session_id, "Hello").await;

        assert!(matches!(result, Err(OrchestratorError::AgentNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_mock_service_is_available() {
        let service = MockOrchestratorService::new();

        assert!(service.is_available().await);

        service.set_available(false).await;
        assert!(!service.is_available().await);
    }

    #[tokio::test]
    async fn test_mock_service_streaming() {
        let service = MockOrchestratorService::new();
        let session_id = IdeationSessionId::new();

        let mut rx = service.send_message_streaming(session_id, "Hello".to_string());

        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }

        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, OrchestratorEvent::TextChunk(_))));
        assert!(events.iter().any(|e| matches!(e, OrchestratorEvent::Complete(_))));
    }

    #[test]
    fn test_parse_stream_line_text_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        let msg = ClaudeOrchestratorService::parse_stream_line(line);

        assert!(msg.is_some());
        if let Some(StreamMessage::ContentBlockDelta { delta }) = msg {
            assert_eq!(delta.delta_type, "text_delta");
            assert_eq!(delta.text, Some("Hello".to_string()));
        }
    }

    #[test]
    fn test_parse_stream_line_tool_use() {
        let line = r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"create_task_proposal"}}"#;
        let msg = ClaudeOrchestratorService::parse_stream_line(line);

        assert!(msg.is_some());
        if let Some(StreamMessage::ContentBlockStart { content_block }) = msg {
            assert_eq!(content_block.block_type, "tool_use");
            assert_eq!(content_block.name, Some("create_task_proposal".to_string()));
        }
    }

    #[test]
    fn test_tool_call_result_success() {
        let result = ToolCallResult {
            tool_name: "test_tool".to_string(),
            success: true,
            result: Some(serde_json::json!({"key": "value"})),
            error: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_call_result_failure() {
        let result = ToolCallResult {
            tool_name: "test_tool".to_string(),
            success: false,
            result: None,
            error: Some("Something went wrong".to_string()),
        };

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_orchestrator_error_display() {
        let err = OrchestratorError::AgentNotAvailable("test".to_string());
        assert!(err.to_string().contains("Agent not available"));

        let err = OrchestratorError::SpawnFailed("test".to_string());
        assert!(err.to_string().contains("spawn"));
    }
}
