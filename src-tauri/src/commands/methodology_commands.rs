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
            phases: m
                .phases
                .iter()
                .map(MethodologyPhaseResponse::from)
                .collect(),
            templates: m
                .templates
                .iter()
                .map(MethodologyTemplateResponse::from)
                .collect(),
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
        return Err(format!("Methodology '{}' is not active", methodology.name));
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
#[path = "methodology_commands_tests.rs"]
mod tests;
