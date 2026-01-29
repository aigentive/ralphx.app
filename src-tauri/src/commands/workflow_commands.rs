// Tauri commands for Workflow CRUD operations
// Thin layer that delegates to WorkflowRepository

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{InternalStatus, WorkflowColumn, WorkflowDefaults, WorkflowId, WorkflowSchema, ColumnBehavior};

/// Input for creating a new column
#[derive(Debug, Deserialize)]
pub struct WorkflowColumnInput {
    pub id: String,
    pub name: String,
    pub maps_to: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub skip_review: Option<bool>,
    pub auto_advance: Option<bool>,
    pub agent_profile: Option<String>,
}

impl WorkflowColumnInput {
    fn to_column(&self) -> Result<WorkflowColumn, String> {
        let maps_to: InternalStatus = self.maps_to.parse()
            .map_err(|_| format!("Invalid internal status: {}", self.maps_to))?;

        let mut column = WorkflowColumn::new(&self.id, &self.name, maps_to);

        if let Some(ref color) = self.color {
            column = column.with_color(color);
        }
        if let Some(ref icon) = self.icon {
            column = column.with_icon(icon);
        }

        // Add behavior if any behavior options are set
        if self.skip_review.is_some() || self.auto_advance.is_some() || self.agent_profile.is_some() {
            let mut behavior = ColumnBehavior::new();
            if let Some(skip) = self.skip_review {
                behavior = behavior.with_skip_review(skip);
            }
            if let Some(advance) = self.auto_advance {
                behavior = behavior.with_auto_advance(advance);
            }
            if let Some(ref profile) = self.agent_profile {
                behavior = behavior.with_agent_profile(profile);
            }
            column = column.with_behavior(behavior);
        }

        Ok(column)
    }
}

/// Input for creating a new workflow
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowInput {
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<WorkflowColumnInput>,
    pub is_default: Option<bool>,
    pub worker_profile: Option<String>,
    pub reviewer_profile: Option<String>,
}

/// Input for updating a workflow
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub columns: Option<Vec<WorkflowColumnInput>>,
    pub is_default: Option<bool>,
    pub worker_profile: Option<String>,
    pub reviewer_profile: Option<String>,
}

/// Response wrapper for state group within a column
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateGroupResponse {
    pub id: String,
    pub label: String,
    pub statuses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_drag_from: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_drop_to: Option<bool>,
}

/// Response wrapper for workflow column
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowColumnResponse {
    pub id: String,
    pub name: String,
    pub maps_to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_review: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<StateGroupResponse>>,
}

impl From<&WorkflowColumn> for WorkflowColumnResponse {
    fn from(col: &WorkflowColumn) -> Self {
        let (skip_review, auto_advance, agent_profile) = match &col.behavior {
            Some(b) => (b.skip_review, b.auto_advance, b.agent_profile.clone()),
            None => (None, None, None),
        };

        let groups = col.groups.as_ref().map(|gs| {
            gs.iter()
                .map(|g| StateGroupResponse {
                    id: g.id.clone(),
                    label: g.label.clone(),
                    statuses: g.statuses.iter().map(|s| s.to_string()).collect(),
                    icon: g.icon.clone(),
                    accent_color: g.accent_color.clone(),
                    can_drag_from: g.can_drag_from,
                    can_drop_to: g.can_drop_to,
                })
                .collect()
        });

        Self {
            id: col.id.clone(),
            name: col.name.clone(),
            maps_to: col.maps_to.to_string(),
            color: col.color.clone(),
            icon: col.icon.clone(),
            skip_review,
            auto_advance,
            agent_profile,
            groups,
        }
    }
}

/// Response wrapper for workflow operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub columns: Vec<WorkflowColumnResponse>,
    pub is_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer_profile: Option<String>,
}

impl From<WorkflowSchema> for WorkflowResponse {
    fn from(workflow: WorkflowSchema) -> Self {
        Self {
            id: workflow.id.as_str().to_string(),
            name: workflow.name,
            description: workflow.description,
            columns: workflow.columns.iter().map(WorkflowColumnResponse::from).collect(),
            is_default: workflow.is_default,
            worker_profile: workflow.defaults.worker_profile,
            reviewer_profile: workflow.defaults.reviewer_profile,
        }
    }
}

/// List all workflows
#[tauri::command]
pub async fn get_workflows(state: State<'_, AppState>) -> Result<Vec<WorkflowResponse>, String> {
    state
        .workflow_repo
        .get_all()
        .await
        .map(|workflows| workflows.into_iter().map(WorkflowResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Get a single workflow by ID
#[tauri::command]
pub async fn get_workflow(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<WorkflowResponse>, String> {
    let workflow_id = WorkflowId::from_string(id);
    state
        .workflow_repo
        .get_by_id(&workflow_id)
        .await
        .map(|opt| opt.map(WorkflowResponse::from))
        .map_err(|e| e.to_string())
}

/// Create a new workflow
#[tauri::command]
pub async fn create_workflow(
    input: CreateWorkflowInput,
    state: State<'_, AppState>,
) -> Result<WorkflowResponse, String> {
    // Parse columns
    let columns: Result<Vec<WorkflowColumn>, String> = input.columns
        .iter()
        .map(|c| c.to_column())
        .collect();
    let columns = columns?;

    let mut workflow = WorkflowSchema::new(&input.name, columns);

    if let Some(ref desc) = input.description {
        workflow = workflow.with_description(desc);
    }

    if input.is_default.unwrap_or(false) {
        workflow = workflow.as_default();
    }

    // Set defaults
    workflow.defaults = WorkflowDefaults {
        worker_profile: input.worker_profile,
        reviewer_profile: input.reviewer_profile,
    };

    state
        .workflow_repo
        .create(workflow)
        .await
        .map(WorkflowResponse::from)
        .map_err(|e| e.to_string())
}

/// Update an existing workflow
#[tauri::command]
pub async fn update_workflow(
    id: String,
    input: UpdateWorkflowInput,
    state: State<'_, AppState>,
) -> Result<WorkflowResponse, String> {
    let workflow_id = WorkflowId::from_string(id);

    // Get existing workflow
    let mut workflow = state
        .workflow_repo
        .get_by_id(&workflow_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Workflow not found: {}", workflow_id.as_str()))?;

    // Apply updates
    if let Some(name) = input.name {
        workflow.name = name;
    }
    if let Some(description) = input.description {
        workflow.description = Some(description);
    }
    if let Some(columns_input) = input.columns {
        let columns: Result<Vec<WorkflowColumn>, String> = columns_input
            .iter()
            .map(|c| c.to_column())
            .collect();
        workflow.columns = columns?;
    }
    if let Some(is_default) = input.is_default {
        workflow.is_default = is_default;
    }
    if let Some(worker_profile) = input.worker_profile {
        workflow.defaults.worker_profile = Some(worker_profile);
    }
    if let Some(reviewer_profile) = input.reviewer_profile {
        workflow.defaults.reviewer_profile = Some(reviewer_profile);
    }

    state
        .workflow_repo
        .update(&workflow)
        .await
        .map_err(|e| e.to_string())?;

    Ok(WorkflowResponse::from(workflow))
}

/// Delete a workflow
#[tauri::command]
pub async fn delete_workflow(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let workflow_id = WorkflowId::from_string(id);
    state
        .workflow_repo
        .delete(&workflow_id)
        .await
        .map_err(|e| e.to_string())
}

/// Seed builtin workflows if they don't exist
/// Returns the number of workflows created
#[tauri::command]
pub async fn seed_builtin_workflows(state: State<'_, AppState>) -> Result<usize, String> {
    let mut created = 0;

    // Check and create RalphX default workflow
    let default_id = WorkflowId::from_string("ralphx-default");
    if state
        .workflow_repo
        .get_by_id(&default_id)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        state
            .workflow_repo
            .create(WorkflowSchema::default_ralphx())
            .await
            .map_err(|e| e.to_string())?;
        created += 1;
    }

    // Check and create Jira workflow
    let jira_id = WorkflowId::from_string("jira-compatible");
    if state
        .workflow_repo
        .get_by_id(&jira_id)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        state
            .workflow_repo
            .create(WorkflowSchema::jira_compatible())
            .await
            .map_err(|e| e.to_string())?;
        created += 1;
    }

    Ok(created)
}

/// Set the default workflow
#[tauri::command]
pub async fn set_default_workflow(
    id: String,
    state: State<'_, AppState>,
) -> Result<WorkflowResponse, String> {
    let workflow_id = WorkflowId::from_string(id);

    state
        .workflow_repo
        .set_default(&workflow_id)
        .await
        .map_err(|e| e.to_string())?;

    // Return the updated workflow
    state
        .workflow_repo
        .get_by_id(&workflow_id)
        .await
        .map_err(|e| e.to_string())?
        .map(WorkflowResponse::from)
        .ok_or_else(|| format!("Workflow not found: {}", workflow_id.as_str()))
}

/// Get the columns for the currently active/default workflow
#[tauri::command]
pub async fn get_active_workflow_columns(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowColumnResponse>, String> {
    let default_workflow = state
        .workflow_repo
        .get_default()
        .await
        .map_err(|e| e.to_string())?;

    match default_workflow {
        Some(workflow) => Ok(workflow.columns.iter().map(WorkflowColumnResponse::from).collect()),
        None => {
            // Return the RalphX default columns if no workflow is set
            let default = WorkflowSchema::default_ralphx();
            Ok(default.columns.iter().map(WorkflowColumnResponse::from).collect())
        }
    }
}

/// Get the builtin workflow schemas
#[tauri::command]
pub async fn get_builtin_workflows() -> Result<Vec<WorkflowResponse>, String> {
    Ok(vec![
        WorkflowResponse::from(WorkflowSchema::default_ralphx()),
        WorkflowResponse::from(WorkflowSchema::jira_compatible()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    #[tokio::test]
    async fn test_create_workflow() {
        let state = setup_test_state();

        let workflow = WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        );

        let created = state.workflow_repo.create(workflow).await.unwrap();
        assert_eq!(created.name, "Test Workflow");
        assert_eq!(created.columns.len(), 2);
    }

    #[tokio::test]
    async fn test_get_workflow_by_id() {
        let state = setup_test_state();

        let workflow = WorkflowSchema::new(
            "Find Me",
            vec![WorkflowColumn::new("col", "Column", InternalStatus::Ready)],
        );
        let id = workflow.id.clone();

        state.workflow_repo.create(workflow).await.unwrap();

        let found = state.workflow_repo.get_by_id(&id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Find Me");
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let state = setup_test_state();

        state.workflow_repo.create(
            WorkflowSchema::new("WF 1", vec![WorkflowColumn::new("a", "A", InternalStatus::Backlog)])
        ).await.unwrap();
        state.workflow_repo.create(
            WorkflowSchema::new("WF 2", vec![WorkflowColumn::new("b", "B", InternalStatus::Ready)])
        ).await.unwrap();

        let all = state.workflow_repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_workflow() {
        let state = setup_test_state();

        let workflow = WorkflowSchema::new(
            "To Delete",
            vec![WorkflowColumn::new("col", "Col", InternalStatus::Backlog)],
        );
        let id = workflow.id.clone();

        state.workflow_repo.create(workflow).await.unwrap();
        state.workflow_repo.delete(&id).await.unwrap();

        let found = state.workflow_repo.get_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_set_default_workflow() {
        let state = setup_test_state();

        let wf1 = WorkflowSchema::default_ralphx();
        let wf2 = WorkflowSchema::new("Second", vec![WorkflowColumn::new("x", "X", InternalStatus::Backlog)]);
        let wf2_id = wf2.id.clone();

        state.workflow_repo.create(wf1).await.unwrap();
        state.workflow_repo.create(wf2).await.unwrap();

        state.workflow_repo.set_default(&wf2_id).await.unwrap();

        let default = state.workflow_repo.get_default().await.unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, wf2_id);
    }

    #[tokio::test]
    async fn test_workflow_response_serialization() {
        let workflow = WorkflowSchema::new(
            "Response Test",
            vec![
                WorkflowColumn::new("col1", "Column 1", InternalStatus::Backlog)
                    .with_color("#ff0000"),
            ],
        ).with_description("A test workflow");

        let response = WorkflowResponse::from(workflow);

        assert_eq!(response.name, "Response Test");
        assert_eq!(response.description, Some("A test workflow".to_string()));
        assert_eq!(response.columns.len(), 1);
        assert_eq!(response.columns[0].color, Some("#ff0000".to_string()));

        // Verify JSON serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Response Test\""));
    }

    #[tokio::test]
    async fn test_column_input_to_column() {
        let input = WorkflowColumnInput {
            id: "test-col".to_string(),
            name: "Test Column".to_string(),
            maps_to: "ready".to_string(),
            color: Some("#00ff00".to_string()),
            icon: None,
            skip_review: Some(true),
            auto_advance: None,
            agent_profile: Some("fast-worker".to_string()),
        };

        let column = input.to_column().unwrap();

        assert_eq!(column.id, "test-col");
        assert_eq!(column.name, "Test Column");
        assert_eq!(column.maps_to, InternalStatus::Ready);
        assert_eq!(column.color, Some("#00ff00".to_string()));

        let behavior = column.behavior.unwrap();
        assert_eq!(behavior.skip_review, Some(true));
        assert_eq!(behavior.agent_profile, Some("fast-worker".to_string()));
    }

    #[tokio::test]
    async fn test_column_input_invalid_status() {
        let input = WorkflowColumnInput {
            id: "test".to_string(),
            name: "Test".to_string(),
            maps_to: "invalid_status".to_string(),
            color: None,
            icon: None,
            skip_review: None,
            auto_advance: None,
            agent_profile: None,
        };

        let result = input.to_column();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid internal status"));
    }

    #[tokio::test]
    async fn test_get_builtin_workflows() {
        let result = get_builtin_workflows().await.unwrap();

        assert_eq!(result.len(), 2);

        let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"RalphX Default"));
        assert!(names.contains(&"Jira Compatible"));
    }

    #[tokio::test]
    async fn test_get_active_workflow_columns_with_default() {
        let state = setup_test_state();

        // Create and set a default workflow
        let workflow = WorkflowSchema::new(
            "My Default",
            vec![
                WorkflowColumn::new("a", "A", InternalStatus::Backlog),
                WorkflowColumn::new("b", "B", InternalStatus::Approved),
            ],
        ).as_default();
        let _id = workflow.id.clone();

        state.workflow_repo.create(workflow).await.unwrap();

        let default = state.workflow_repo.get_default().await.unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().columns.len(), 2);
    }
}
