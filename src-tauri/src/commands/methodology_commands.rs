// Tauri commands for Methodology operations
// Thin layer that delegates to MethodologyRepository

use serde::Serialize;
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::methodology::{
    MethodologyExtension, MethodologyId, MethodologyPhase, MethodologyTemplate,
};
use crate::domain::entities::workflow::WorkflowSchema;

/// Response wrapper for methodology operations
#[derive(Debug, Serialize)]
pub struct MethodologyResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub agent_profiles: Vec<String>,
    pub skills: Vec<String>,
    pub workflow_id: String,
    pub workflow_name: String,
    pub phases: Vec<MethodologyPhaseResponse>,
    pub templates: Vec<MethodologyTemplateResponse>,
    pub is_active: bool,
    pub phase_count: usize,
    pub agent_count: usize,
    pub created_at: String,
}

impl From<MethodologyExtension> for MethodologyResponse {
    fn from(m: MethodologyExtension) -> Self {
        Self {
            id: m.id.as_str().to_string(),
            name: m.name.clone(),
            description: m.description.clone(),
            agent_profiles: m.agent_profiles.clone(),
            skills: m.skills.clone(),
            workflow_id: m.workflow.id.as_str().to_string(),
            workflow_name: m.workflow.name.clone(),
            phases: m.phases.iter().map(MethodologyPhaseResponse::from).collect(),
            templates: m.templates.iter().map(MethodologyTemplateResponse::from).collect(),
            is_active: m.is_active,
            phase_count: m.phase_count(),
            agent_count: m.agent_count(),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

/// Response wrapper for methodology phase
#[derive(Debug, Serialize)]
pub struct MethodologyPhaseResponse {
    pub id: String,
    pub name: String,
    pub order: u32,
    pub description: Option<String>,
    pub agent_profiles: Vec<String>,
    pub column_ids: Vec<String>,
}

impl From<&MethodologyPhase> for MethodologyPhaseResponse {
    fn from(p: &MethodologyPhase) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            order: p.order,
            description: p.description.clone(),
            agent_profiles: p.agent_profiles.clone(),
            column_ids: p.column_ids.clone(),
        }
    }
}

/// Response wrapper for methodology template
#[derive(Debug, Serialize)]
pub struct MethodologyTemplateResponse {
    pub artifact_type: String,
    pub template_path: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl From<&MethodologyTemplate> for MethodologyTemplateResponse {
    fn from(t: &MethodologyTemplate) -> Self {
        Self {
            artifact_type: t.artifact_type.clone(),
            template_path: t.template_path.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
        }
    }
}

/// Response for methodology activation including workflow and agent info
#[derive(Debug, Serialize)]
pub struct MethodologyActivationResponse {
    pub methodology: MethodologyResponse,
    pub workflow: WorkflowSchemaResponse,
    pub agent_profiles: Vec<String>,
    pub skills: Vec<String>,
    pub previous_methodology_id: Option<String>,
}

/// Simplified workflow schema response for activation
#[derive(Debug, Serialize)]
pub struct WorkflowSchemaResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub column_count: usize,
}

impl From<&WorkflowSchema> for WorkflowSchemaResponse {
    fn from(w: &WorkflowSchema) -> Self {
        Self {
            id: w.id.as_str().to_string(),
            name: w.name.clone(),
            description: w.description.clone(),
            column_count: w.columns.len(),
        }
    }
}

// ===== Methodology Commands =====

/// Get all methodologies
#[tauri::command]
pub async fn get_methodologies(
    state: State<'_, AppState>,
) -> Result<Vec<MethodologyResponse>, String> {
    state
        .methodology_repo
        .get_all()
        .await
        .map(|methodologies| {
            methodologies
                .into_iter()
                .map(MethodologyResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get the currently active methodology (if any)
#[tauri::command]
pub async fn get_active_methodology(
    state: State<'_, AppState>,
) -> Result<Option<MethodologyResponse>, String> {
    state
        .methodology_repo
        .get_active()
        .await
        .map(|opt| opt.map(MethodologyResponse::from))
        .map_err(|e| e.to_string())
}

/// Activate a methodology by ID, deactivating any currently active one
#[tauri::command]
pub async fn activate_methodology(
    id: String,
    state: State<'_, AppState>,
) -> Result<MethodologyActivationResponse, String> {
    let methodology_id = MethodologyId::from_string(id);

    // Get the methodology to activate
    let methodology = state
        .methodology_repo
        .get_by_id(&methodology_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Methodology not found: {}", methodology_id.as_str()))?;

    if methodology.is_active {
        return Err(format!(
            "Methodology '{}' is already active",
            methodology.name
        ));
    }

    // Get current active methodology (if any)
    let previous = state
        .methodology_repo
        .get_active()
        .await
        .map_err(|e| e.to_string())?;
    let previous_id = previous.as_ref().map(|m| m.id.as_str().to_string());

    // Deactivate previous if exists
    if let Some(prev) = &previous {
        state
            .methodology_repo
            .deactivate(&prev.id)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Activate the new methodology
    state
        .methodology_repo
        .activate(&methodology_id)
        .await
        .map_err(|e| e.to_string())?;

    // Re-fetch to get updated state
    let activated = state
        .methodology_repo
        .get_by_id(&methodology_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Methodology not found after activation: {}",
                methodology_id.as_str()
            )
        })?;

    Ok(MethodologyActivationResponse {
        workflow: WorkflowSchemaResponse::from(&activated.workflow),
        agent_profiles: activated.agent_profiles.clone(),
        skills: activated.skills.clone(),
        previous_methodology_id: previous_id,
        methodology: MethodologyResponse::from(activated),
    })
}

/// Deactivate a methodology by ID
#[tauri::command]
pub async fn deactivate_methodology(
    id: String,
    state: State<'_, AppState>,
) -> Result<MethodologyResponse, String> {
    let methodology_id = MethodologyId::from_string(id);

    // Get the methodology to deactivate
    let methodology = state
        .methodology_repo
        .get_by_id(&methodology_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Methodology not found: {}", methodology_id.as_str()))?;

    if !methodology.is_active {
        return Err(format!(
            "Methodology '{}' is not active",
            methodology.name
        ));
    }

    // Deactivate the methodology
    state
        .methodology_repo
        .deactivate(&methodology_id)
        .await
        .map_err(|e| e.to_string())?;

    // Re-fetch to get updated state
    let deactivated = state
        .methodology_repo
        .get_by_id(&methodology_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Methodology not found after deactivation: {}",
                methodology_id.as_str()
            )
        })?;

    Ok(MethodologyResponse::from(deactivated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::methodology::MethodologyExtension;
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::{WorkflowColumn, WorkflowSchema};

    fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    fn create_test_workflow() -> WorkflowSchema {
        WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    fn create_test_methodology() -> MethodologyExtension {
        MethodologyExtension::new("Test Method", create_test_workflow())
            .with_description("A test methodology")
            .with_agent_profiles(["analyst", "developer"])
            .with_skills(["skill1", "skill2"])
    }

    // ===== get_methodologies Tests =====

    #[tokio::test]
    async fn test_get_methodologies_empty() {
        let state = setup_test_state();

        let result = state.methodology_repo.get_all().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_methodologies_returns_all() {
        let state = setup_test_state();

        // Add two methodologies
        let m1 = create_test_methodology();
        let mut m2 = create_test_methodology();
        m2.name = "Second Method".to_string();

        state.methodology_repo.create(m1).await.unwrap();
        state.methodology_repo.create(m2).await.unwrap();

        let result = state.methodology_repo.get_all().await.unwrap();
        assert_eq!(result.len(), 2);
    }

    // ===== get_active_methodology Tests =====

    #[tokio::test]
    async fn test_get_active_methodology_none() {
        let state = setup_test_state();

        let methodology = create_test_methodology();
        state.methodology_repo.create(methodology).await.unwrap();

        let result = state.methodology_repo.get_active().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_active_methodology_some() {
        let state = setup_test_state();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        state.methodology_repo.create(methodology).await.unwrap();
        state.methodology_repo.activate(&id).await.unwrap();

        let result = state.methodology_repo.get_active().await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, id);
    }

    // ===== activate_methodology Tests =====

    #[tokio::test]
    async fn test_activate_methodology_success() {
        let state = setup_test_state();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        state.methodology_repo.create(methodology).await.unwrap();

        // Activate
        state.methodology_repo.activate(&id).await.unwrap();

        // Verify active
        let active = state.methodology_repo.get_active().await.unwrap();
        assert!(active.is_some());
        assert!(active.unwrap().is_active);
    }

    #[tokio::test]
    async fn test_activate_methodology_deactivates_previous() {
        let state = setup_test_state();

        // Create and activate first methodology
        let m1 = create_test_methodology();
        let id1 = m1.id.clone();
        state.methodology_repo.create(m1).await.unwrap();
        state.methodology_repo.activate(&id1).await.unwrap();

        // Create second methodology
        let mut m2 = create_test_methodology();
        m2.name = "Second Method".to_string();
        let id2 = m2.id.clone();
        state.methodology_repo.create(m2).await.unwrap();

        // Deactivate first, activate second
        state.methodology_repo.deactivate(&id1).await.unwrap();
        state.methodology_repo.activate(&id2).await.unwrap();

        // Verify second is active
        let active = state.methodology_repo.get_active().await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, id2);

        // Verify first is not active
        let m1_now = state.methodology_repo.get_by_id(&id1).await.unwrap().unwrap();
        assert!(!m1_now.is_active);
    }

    // ===== deactivate_methodology Tests =====

    #[tokio::test]
    async fn test_deactivate_methodology_success() {
        let state = setup_test_state();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        state.methodology_repo.create(methodology).await.unwrap();
        state.methodology_repo.activate(&id).await.unwrap();

        // Deactivate
        state.methodology_repo.deactivate(&id).await.unwrap();

        // Verify no active methodology
        let active = state.methodology_repo.get_active().await.unwrap();
        assert!(active.is_none());

        // Verify methodology is not active
        let methodology = state.methodology_repo.get_by_id(&id).await.unwrap().unwrap();
        assert!(!methodology.is_active);
    }

    // ===== Response Serialization Tests =====

    #[test]
    fn test_methodology_response_serialization() {
        let methodology = create_test_methodology();
        let response = MethodologyResponse::from(methodology);

        assert_eq!(response.name, "Test Method");
        assert_eq!(response.description, Some("A test methodology".to_string()));
        assert_eq!(response.agent_profiles.len(), 2);
        assert_eq!(response.skills.len(), 2);
        assert!(!response.is_active);
        assert_eq!(response.phase_count, 0);
        assert_eq!(response.agent_count, 2);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Test Method\""));
        assert!(json.contains("\"is_active\":false"));
    }

    #[test]
    fn test_methodology_activation_response_serialization() {
        let methodology = create_test_methodology();
        let workflow = methodology.workflow.clone();

        let response = MethodologyActivationResponse {
            methodology: MethodologyResponse::from(methodology),
            workflow: WorkflowSchemaResponse::from(&workflow),
            agent_profiles: vec!["analyst".to_string(), "developer".to_string()],
            skills: vec!["skill1".to_string()],
            previous_methodology_id: Some("prev-id".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent_profiles\":[\"analyst\",\"developer\"]"));
        assert!(json.contains("\"previous_methodology_id\":\"prev-id\""));
    }

    #[test]
    fn test_builtin_methodologies_response() {
        let bmad = MethodologyExtension::bmad();
        let response = MethodologyResponse::from(bmad);

        assert_eq!(response.id, "bmad-method");
        assert_eq!(response.name, "BMAD Method");
        assert_eq!(response.agent_count, 8);
        assert_eq!(response.phase_count, 4);

        let gsd = MethodologyExtension::gsd();
        let response = MethodologyResponse::from(gsd);

        assert_eq!(response.id, "gsd-method");
        assert_eq!(response.name, "GSD (Get Shit Done)");
        assert_eq!(response.agent_count, 11);
        assert_eq!(response.phase_count, 4);
    }
}
