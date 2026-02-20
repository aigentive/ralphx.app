// Tauri commands for QA operations
// Thin layer that delegates to TaskQARepository and QASettings

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{TaskId, TaskQA};
use crate::domain::qa::{AcceptanceCriterion, QAResults, QASettings, QAStepResult, QATestStep};

// ============================================================================
// Response Types
// ============================================================================

/// Response for AcceptanceCriterion
#[derive(Debug, Clone, Serialize)]
pub struct AcceptanceCriterionResponse {
    pub id: String,
    pub description: String,
    pub testable: bool,
    pub criteria_type: String,
}

impl From<AcceptanceCriterion> for AcceptanceCriterionResponse {
    fn from(c: AcceptanceCriterion) -> Self {
        Self {
            id: c.id,
            description: c.description,
            testable: c.testable,
            criteria_type: c.criteria_type.as_str().to_string(),
        }
    }
}

/// Response for QATestStep
#[derive(Debug, Clone, Serialize)]
pub struct QATestStepResponse {
    pub id: String,
    pub criteria_id: String,
    pub description: String,
    pub commands: Vec<String>,
    pub expected: String,
}

impl From<QATestStep> for QATestStepResponse {
    fn from(s: QATestStep) -> Self {
        Self {
            id: s.id,
            criteria_id: s.criteria_id,
            description: s.description,
            commands: s.commands,
            expected: s.expected,
        }
    }
}

/// Response for QAStepResult
#[derive(Debug, Clone, Serialize)]
pub struct QAStepResultResponse {
    pub step_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<QAStepResult> for QAStepResultResponse {
    fn from(r: QAStepResult) -> Self {
        Self {
            step_id: r.step_id,
            status: r.status.as_str().to_string(),
            screenshot: r.screenshot,
            actual: r.actual,
            expected: r.expected,
            error: r.error,
        }
    }
}

/// Response for QAResults
#[derive(Debug, Clone, Serialize)]
pub struct QAResultsResponse {
    pub task_id: String,
    pub overall_status: String,
    pub total_steps: usize,
    pub passed_steps: usize,
    pub failed_steps: usize,
    pub steps: Vec<QAStepResultResponse>,
}

impl From<QAResults> for QAResultsResponse {
    fn from(r: QAResults) -> Self {
        Self {
            task_id: r.task_id,
            overall_status: r.overall_status.as_str().to_string(),
            total_steps: r.total_steps,
            passed_steps: r.passed_steps,
            failed_steps: r.failed_steps,
            steps: r
                .steps
                .into_iter()
                .map(QAStepResultResponse::from)
                .collect(),
        }
    }
}

/// Response for TaskQA
#[derive(Debug, Clone, Serialize)]
pub struct TaskQAResponse {
    pub id: String,
    pub task_id: String,

    // Phase 1: QA Prep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<Vec<AcceptanceCriterionResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_test_steps: Option<Vec<QATestStepResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_completed_at: Option<String>,

    // Phase 2: QA Refinement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_implementation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refined_test_steps: Option<Vec<QATestStepResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refinement_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refinement_completed_at: Option<String>,

    // Phase 3: QA Testing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_results: Option<QAResultsResponse>,
    pub screenshots: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_completed_at: Option<String>,

    pub created_at: String,
}

impl From<TaskQA> for TaskQAResponse {
    fn from(qa: TaskQA) -> Self {
        Self {
            id: qa.id.as_str().to_string(),
            task_id: qa.task_id.as_str().to_string(),

            acceptance_criteria: qa.acceptance_criteria.map(|ac| {
                ac.acceptance_criteria
                    .into_iter()
                    .map(AcceptanceCriterionResponse::from)
                    .collect()
            }),
            qa_test_steps: qa.qa_test_steps.map(|qs| {
                qs.qa_steps
                    .into_iter()
                    .map(QATestStepResponse::from)
                    .collect()
            }),
            prep_agent_id: qa.prep_agent_id,
            prep_started_at: qa.prep_started_at.map(|dt| dt.to_rfc3339()),
            prep_completed_at: qa.prep_completed_at.map(|dt| dt.to_rfc3339()),

            actual_implementation: qa.actual_implementation,
            refined_test_steps: qa.refined_test_steps.map(|rs| {
                rs.qa_steps
                    .into_iter()
                    .map(QATestStepResponse::from)
                    .collect()
            }),
            refinement_agent_id: qa.refinement_agent_id,
            refinement_completed_at: qa.refinement_completed_at.map(|dt| dt.to_rfc3339()),

            test_results: qa.test_results.map(QAResultsResponse::from),
            screenshots: qa.screenshots,
            test_agent_id: qa.test_agent_id,
            test_completed_at: qa.test_completed_at.map(|dt| dt.to_rfc3339()),

            created_at: qa.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Input Types
// ============================================================================

/// Input for updating QA settings
#[derive(Debug, Deserialize)]
pub struct UpdateQASettingsInput {
    #[serde(default)]
    pub qa_enabled: Option<bool>,
    #[serde(default)]
    pub auto_qa_for_ui_tasks: Option<bool>,
    #[serde(default)]
    pub auto_qa_for_api_tasks: Option<bool>,
    #[serde(default)]
    pub qa_prep_enabled: Option<bool>,
    #[serde(default)]
    pub browser_testing_enabled: Option<bool>,
    #[serde(default)]
    pub browser_testing_url: Option<String>,
}

// ============================================================================
// Commands
// ============================================================================

/// Get global QA settings
#[tauri::command]
pub async fn get_qa_settings(state: State<'_, AppState>) -> Result<QASettings, String> {
    let settings = state.qa_settings.read().await;
    Ok(settings.clone())
}

/// Update global QA settings
#[tauri::command]
pub async fn update_qa_settings(
    input: UpdateQASettingsInput,
    state: State<'_, AppState>,
) -> Result<QASettings, String> {
    let mut settings = state.qa_settings.write().await;

    if let Some(qa_enabled) = input.qa_enabled {
        settings.qa_enabled = qa_enabled;
    }
    if let Some(auto_qa_for_ui_tasks) = input.auto_qa_for_ui_tasks {
        settings.auto_qa_for_ui_tasks = auto_qa_for_ui_tasks;
    }
    if let Some(auto_qa_for_api_tasks) = input.auto_qa_for_api_tasks {
        settings.auto_qa_for_api_tasks = auto_qa_for_api_tasks;
    }
    if let Some(qa_prep_enabled) = input.qa_prep_enabled {
        settings.qa_prep_enabled = qa_prep_enabled;
    }
    if let Some(browser_testing_enabled) = input.browser_testing_enabled {
        settings.browser_testing_enabled = browser_testing_enabled;
    }
    if let Some(browser_testing_url) = input.browser_testing_url {
        settings.browser_testing_url = browser_testing_url;
    }

    Ok(settings.clone())
}

/// Get TaskQA for a specific task
#[tauri::command]
pub async fn get_task_qa(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TaskQAResponse>, String> {
    let task_id = TaskId::from_string(task_id);
    state
        .task_qa_repo
        .get_by_task_id(&task_id)
        .await
        .map(|opt| opt.map(TaskQAResponse::from))
        .map_err(|e| e.to_string())
}

/// Get QA results for a specific task
#[tauri::command]
pub async fn get_qa_results(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Option<QAResultsResponse>, String> {
    let task_id = TaskId::from_string(task_id);
    let task_qa = state
        .task_qa_repo
        .get_by_task_id(&task_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task_qa.and_then(|qa| qa.test_results.map(QAResultsResponse::from)))
}

/// Retry QA tests for a task
/// Resets test results and triggers re-testing
#[tauri::command]
pub async fn retry_qa(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskQAResponse, String> {
    let task_id_parsed = TaskId::from_string(task_id.clone());

    // Get existing TaskQA record
    let task_qa = state
        .task_qa_repo
        .get_by_task_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No QA record found for task: {}", task_id))?;

    // Create fresh results with pending status for all steps
    let step_ids: Vec<String> = task_qa
        .effective_test_steps()
        .map(|steps| steps.qa_steps.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();

    let fresh_results = QAResults::new(&task_id, step_ids);

    // Update with fresh results (clears previous test data)
    state
        .task_qa_repo
        .update_results(&task_qa.id, "", &fresh_results, &[])
        .await
        .map_err(|e| e.to_string())?;

    // Fetch updated record
    let updated = state
        .task_qa_repo
        .get_by_task_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Failed to fetch updated TaskQA".to_string())?;

    Ok(TaskQAResponse::from(updated))
}

/// Skip QA for a task (mark as passed to bypass failure)
#[tauri::command]
pub async fn skip_qa(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskQAResponse, String> {
    let task_id_parsed = TaskId::from_string(task_id.clone());

    // Get existing TaskQA record
    let task_qa = state
        .task_qa_repo
        .get_by_task_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No QA record found for task: {}", task_id))?;

    // Create results that indicate skipped/passed
    let step_ids: Vec<String> = task_qa
        .effective_test_steps()
        .map(|steps| steps.qa_steps.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();

    // Mark all steps as passed (skipped behavior)
    let passed_results = QAResults::from_results(
        &task_id,
        step_ids
            .into_iter()
            .map(|id| QAStepResult::skipped(id, Some("QA skipped by user".to_string())))
            .collect(),
    );

    // Update with skipped results
    state
        .task_qa_repo
        .update_results(&task_qa.id, "user-skip", &passed_results, &[])
        .await
        .map_err(|e| e.to_string())?;

    // Fetch updated record
    let updated = state
        .task_qa_repo
        .get_by_task_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Failed to fetch updated TaskQA".to_string())?;

    Ok(TaskQAResponse::from(updated))
}

#[cfg(test)]
#[path = "qa_commands_tests.rs"]
mod tests;
