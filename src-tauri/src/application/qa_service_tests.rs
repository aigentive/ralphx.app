use super::*;
use crate::domain::qa::{AcceptanceCriterion, QAStepResult, QATestStep};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock as StdRwLock;

// Mock repository
struct MockTaskQARepository {
    records: StdRwLock<HashMap<String, TaskQA>>,
}

impl MockTaskQARepository {
    fn new() -> Self {
        Self {
            records: StdRwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TaskQARepository for MockTaskQARepository {
    async fn create(&self, task_qa: &TaskQA) -> AppResult<()> {
        self.records
            .write()
            .unwrap()
            .insert(task_qa.id.as_str().to_string(), task_qa.clone());
        Ok(())
    }

    async fn get_by_id(&self, id: &TaskQAId) -> AppResult<Option<TaskQA>> {
        Ok(self.records.read().unwrap().get(id.as_str()).cloned())
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Option<TaskQA>> {
        Ok(self
            .records
            .read()
            .unwrap()
            .values()
            .find(|r| r.task_id == *task_id)
            .cloned())
    }

    async fn update_prep(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        criteria: &AcceptanceCriteria,
        steps: &QATestSteps,
    ) -> AppResult<()> {
        if let Some(record) = self.records.write().unwrap().get_mut(id.as_str()) {
            record.prep_agent_id = Some(agent_id.to_string());
            record.acceptance_criteria = Some(criteria.clone());
            record.qa_test_steps = Some(steps.clone());
            record.prep_completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn update_refinement(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        actual: &str,
        steps: &QATestSteps,
    ) -> AppResult<()> {
        if let Some(record) = self.records.write().unwrap().get_mut(id.as_str()) {
            record.refinement_agent_id = Some(agent_id.to_string());
            record.actual_implementation = Some(actual.to_string());
            record.refined_test_steps = Some(steps.clone());
            record.refinement_completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn update_results(
        &self,
        id: &TaskQAId,
        agent_id: &str,
        results: &QAResults,
        screenshots: &[String],
    ) -> AppResult<()> {
        if let Some(record) = self.records.write().unwrap().get_mut(id.as_str()) {
            record.test_agent_id = Some(agent_id.to_string());
            record.test_results = Some(results.clone());
            record.screenshots = screenshots.to_vec();
            record.test_completed_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn get_pending_prep(&self) -> AppResult<Vec<TaskQA>> {
        Ok(self
            .records
            .read()
            .unwrap()
            .values()
            .filter(|r| r.acceptance_criteria.is_none())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &TaskQAId) -> AppResult<()> {
        self.records.write().unwrap().remove(id.as_str());
        Ok(())
    }

    async fn delete_by_task_id(&self, task_id: &TaskId) -> AppResult<()> {
        self.records
            .write()
            .unwrap()
            .retain(|_, r| r.task_id != *task_id);
        Ok(())
    }

    async fn exists_for_task(&self, task_id: &TaskId) -> AppResult<bool> {
        Ok(self
            .records
            .read()
            .unwrap()
            .values()
            .any(|r| r.task_id == *task_id))
    }
}

// Mock agentic client
struct MockAgenticClient {
    output: String,
    success: bool,
}

impl MockAgenticClient {
    fn success_with(output: &str) -> Self {
        Self {
            output: output.to_string(),
            success: true,
        }
    }

    fn failing(error: &str) -> Self {
        Self {
            output: error.to_string(),
            success: false,
        }
    }
}

#[async_trait]
impl AgenticClient for MockAgenticClient {
    async fn spawn_agent(
        &self,
        config: AgentConfig,
    ) -> crate::domain::agents::error::AgentResult<AgentHandle> {
        Ok(AgentHandle::mock(config.role))
    }

    async fn stop_agent(
        &self,
        _handle: &AgentHandle,
    ) -> crate::domain::agents::error::AgentResult<()> {
        Ok(())
    }

    async fn wait_for_completion(
        &self,
        _handle: &AgentHandle,
    ) -> crate::domain::agents::error::AgentResult<crate::domain::agents::types::AgentOutput>
    {
        if self.success {
            Ok(crate::domain::agents::types::AgentOutput::success(
                &self.output,
            ))
        } else {
            Ok(crate::domain::agents::types::AgentOutput::failed(
                &self.output,
                1,
            ))
        }
    }

    async fn send_prompt(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> crate::domain::agents::error::AgentResult<crate::domain::agents::types::AgentResponse>
    {
        Ok(crate::domain::agents::types::AgentResponse::new(
            &self.output,
        ))
    }

    fn stream_response(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Stream<
                    Item = crate::domain::agents::error::AgentResult<
                        crate::domain::agents::types::ResponseChunk,
                    >,
                > + Send,
        >,
    > {
        Box::pin(futures::stream::empty())
    }

    fn capabilities(&self) -> &crate::domain::agents::capabilities::ClientCapabilities {
        static CAPS: std::sync::OnceLock<
            crate::domain::agents::capabilities::ClientCapabilities,
        > = std::sync::OnceLock::new();
        CAPS.get_or_init(crate::domain::agents::capabilities::ClientCapabilities::mock)
    }

    async fn is_available(&self) -> crate::domain::agents::error::AgentResult<bool> {
        Ok(true)
    }
}

// Test helpers
fn sample_qa_output() -> String {
    r#"```json
{
  "acceptance_criteria": [
{"id": "AC1", "description": "Button is visible", "testable": true, "type": "visual"}
  ],
  "qa_steps": [
{"id": "QA1", "criteria_id": "AC1", "description": "Check button", "commands": ["agent-browser is visible @e1"], "expected": "Button visible"}
  ]
}
```"#
        .to_string()
}

#[tokio::test]
async fn test_qa_service_new() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with("test"));
    let _service = QAService::new(repo, client);
}

#[tokio::test]
async fn test_start_qa_prep() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo.clone(), client);

    let task_id = TaskId::from_string("task-123".to_string());
    let handle = service.start_qa_prep(&task_id, "Build a button").await;

    assert!(handle.is_ok());
    assert!(repo.exists_for_task(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_check_prep_complete_not_started() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with("test"));
    let service = QAService::new(repo, client);

    let task_id = TaskId::from_string("task-123".to_string());
    let complete = service.check_prep_complete(&task_id).await.unwrap();

    assert!(!complete);
}

#[tokio::test]
async fn test_wait_for_prep_success() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo.clone(), client);

    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();

    let result = service.wait_for_prep(&task_id).await;
    assert!(result.is_ok());

    let (criteria, steps) = result.unwrap();
    assert_eq!(criteria.acceptance_criteria.len(), 1);
    assert_eq!(steps.qa_steps.len(), 1);
}

#[tokio::test]
async fn test_wait_for_prep_failure() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::failing("Agent crashed"));
    let service = QAService::new(repo, client);

    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();

    let result = service.wait_for_prep(&task_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_start_qa_testing() {
    let repo = Arc::new(MockTaskQARepository::new());

    // Create a task with prep complete
    let task_id = TaskId::from_string("task-123".to_string());
    let mut task_qa = TaskQA::new(task_id.clone());
    task_qa.start_prep("agent-1".to_string());
    task_qa.complete_prep(
        AcceptanceCriteria::from_criteria(vec![AcceptanceCriterion::visual(
            "AC1",
            "Button visible",
        )]),
        QATestSteps::from_steps(vec![QATestStep::new(
            "QA1",
            "AC1",
            "Check button",
            vec!["agent-browser is visible @e1".to_string()],
            "Visible",
        )]),
    );
    repo.create(&task_qa).await.unwrap();

    let client = Arc::new(MockAgenticClient::success_with("{}"));
    let service = QAService::new(Arc::clone(&repo), client);

    let handle = service.start_qa_testing(&task_id).await;
    assert!(handle.is_ok());
}

#[tokio::test]
async fn test_record_results() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo.clone(), client);

    // Start and complete prep
    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();
    service.wait_for_prep(&task_id).await.unwrap();

    // Record results
    let results = QAResults::from_results(
        "task-123",
        vec![QAStepResult::passed("QA1", Some("ss.png".into()))],
    );
    let screenshots = vec!["ss.png".to_string()];

    let result = service
        .record_results(&task_id, "agent-2", &results, &screenshots)
        .await;
    assert!(result.is_ok());

    // Verify
    let task_qa = repo.get_by_task_id(&task_id).await.unwrap().unwrap();
    assert!(task_qa.test_results.is_some());
    assert!(!task_qa.screenshots.is_empty());
}

#[tokio::test]
async fn test_is_qa_passed() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo.clone(), client);

    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();
    service.wait_for_prep(&task_id).await.unwrap();

    // Record passing results
    let results = QAResults::from_results(
        "task-123",
        vec![QAStepResult::passed("QA1", Some("ss.png".into()))],
    );
    service
        .record_results(&task_id, "agent-2", &results, &["ss.png".to_string()])
        .await
        .unwrap();

    assert!(service.is_qa_passed(&task_id).await.unwrap());
    assert!(!service.is_qa_failed(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_is_qa_failed() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo.clone(), client);

    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();
    service.wait_for_prep(&task_id).await.unwrap();

    // Record failing results
    let results = QAResults::from_results(
        "task-123",
        vec![QAStepResult::failed(
            "QA1",
            "Element not found",
            Some("ss.png".into()),
        )],
    );
    service
        .record_results(&task_id, "agent-2", &results, &["ss.png".to_string()])
        .await
        .unwrap();

    assert!(service.is_qa_failed(&task_id).await.unwrap());
    assert!(!service.is_qa_passed(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_get_state() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo, client);

    let task_id = TaskId::from_string("task-123".to_string());

    // Before starting, no state
    assert!(service.get_state(&task_id).await.is_none());

    // After starting prep
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();
    let state = service.get_state(&task_id).await;
    assert!(state.is_some());
    assert_eq!(state.unwrap().prep_status, QAPrepStatus::Running);
}

#[tokio::test]
async fn test_stop_agent() {
    let repo = Arc::new(MockTaskQARepository::new());
    let client = Arc::new(MockAgenticClient::success_with(&sample_qa_output()));
    let service = QAService::new(repo, client);

    let task_id = TaskId::from_string("task-123".to_string());
    service
        .start_qa_prep(&task_id, "Build a button")
        .await
        .unwrap();

    let result = service.stop_agent(&task_id).await;
    assert!(result.is_ok());

    let state = service.get_state(&task_id).await.unwrap();
    assert!(state.prep_handle.is_none());
}

#[test]
fn test_parse_qa_prep_output_with_code_block() {
    let output = sample_qa_output();
    let (criteria, steps) = parse_qa_prep_output(&output).unwrap();
    assert_eq!(criteria.acceptance_criteria.len(), 1);
    assert_eq!(steps.qa_steps.len(), 1);
}

#[test]
fn test_parse_qa_prep_output_raw_json() {
    let output = r#"{"acceptance_criteria": [], "qa_steps": []}"#;
    let result = parse_qa_prep_output(output);
    assert!(result.is_ok());
}

#[test]
fn test_parse_qa_prep_output_with_text() {
    let output = r#"Here are the acceptance criteria:
{"acceptance_criteria": [{"id": "AC1", "description": "Test", "testable": true, "type": "visual"}], "qa_steps": []}
That's all!"#;
    let result = parse_qa_prep_output(output);
    assert!(result.is_ok());
}

#[test]
fn test_extract_json_from_output_code_block() {
    let output = "```json\n{\"key\": \"value\"}\n```";
    let json = extract_json_from_output(output);
    assert_eq!(json, "{\"key\": \"value\"}");
}

#[test]
fn test_extract_json_from_output_raw() {
    let output = "prefix {\"key\": \"value\"} suffix";
    let json = extract_json_from_output(output);
    assert_eq!(json, "{\"key\": \"value\"}");
}

#[test]
fn test_qa_prep_status_variants() {
    assert_eq!(QAPrepStatus::Pending, QAPrepStatus::Pending);
    assert_eq!(QAPrepStatus::Running, QAPrepStatus::Running);
    assert_eq!(QAPrepStatus::Completed, QAPrepStatus::Completed);

    let failed = QAPrepStatus::Failed("error".into());
    assert!(matches!(failed, QAPrepStatus::Failed(_)));
}

#[test]
fn test_task_qa_state_new() {
    let task_id = TaskId::from_string("task-123".to_string());
    let state = TaskQAState::new(task_id.clone());

    assert_eq!(state.task_id, task_id);
    assert!(state.qa_id.is_none());
    assert!(state.prep_handle.is_none());
    assert_eq!(state.prep_status, QAPrepStatus::Pending);
    assert!(state.test_handle.is_none());
    assert!(!state.testing_in_progress);
}

#[test]
fn test_task_qa_state_is_prep_complete() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut state = TaskQAState::new(task_id);

    assert!(!state.is_prep_complete());

    state.prep_status = QAPrepStatus::Completed;
    assert!(state.is_prep_complete());
}

#[test]
fn test_task_qa_state_is_prep_failed() {
    let task_id = TaskId::from_string("task-123".to_string());
    let mut state = TaskQAState::new(task_id);

    assert!(!state.is_prep_failed());

    state.prep_status = QAPrepStatus::Failed("error".into());
    assert!(state.is_prep_failed());
}
