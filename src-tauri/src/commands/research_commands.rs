// Tauri commands for Research Process operations
// Thin layer that delegates to ProcessRepository

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::research::{
    CustomDepth, ResearchBrief, ResearchDepth, ResearchDepthPreset, ResearchOutput,
    ResearchProcess, ResearchProcessId, ResearchProcessStatus,
};

/// Input for creating/starting a new research process
#[derive(Debug, Deserialize)]
pub struct StartResearchInput {
    pub name: String,
    pub question: String,
    pub context: Option<String>,
    pub scope: Option<String>,
    pub constraints: Option<Vec<String>>,
    pub agent_profile_id: String,
    pub depth_preset: Option<String>, // "quick-scan", "standard", "deep-dive", "exhaustive"
    pub custom_depth: Option<CustomDepthInput>,
    pub target_bucket: Option<String>,
}

/// Input for custom depth configuration
#[derive(Debug, Deserialize)]
pub struct CustomDepthInput {
    pub max_iterations: u32,
    pub timeout_hours: f32,
    pub checkpoint_interval: u32,
}

impl From<CustomDepthInput> for CustomDepth {
    fn from(input: CustomDepthInput) -> Self {
        CustomDepth::new(
            input.max_iterations,
            input.timeout_hours,
            input.checkpoint_interval,
        )
    }
}

/// Response wrapper for research process operations
#[derive(Debug, Serialize)]
pub struct ResearchProcessResponse {
    pub id: String,
    pub name: String,
    pub question: String,
    pub context: Option<String>,
    pub scope: Option<String>,
    pub constraints: Vec<String>,
    pub agent_profile_id: String,
    pub depth_preset: Option<String>,
    pub max_iterations: u32,
    pub timeout_hours: f32,
    pub checkpoint_interval: u32,
    pub target_bucket: String,
    pub status: String,
    pub current_iteration: u32,
    pub progress_percentage: f32,
    pub error_message: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<ResearchProcess> for ResearchProcessResponse {
    fn from(process: ResearchProcess) -> Self {
        let resolved = process.resolved_depth();
        let depth_preset = match &process.depth {
            ResearchDepth::Preset(preset) => Some(preset.to_string()),
            ResearchDepth::Custom(_) => None,
        };

        Self {
            id: process.id.as_str().to_string(),
            name: process.name.clone(),
            question: process.brief.question.clone(),
            context: process.brief.context.clone(),
            scope: process.brief.scope.clone(),
            constraints: process.brief.constraints.clone(),
            agent_profile_id: process.agent_profile_id.clone(),
            depth_preset,
            max_iterations: resolved.max_iterations,
            timeout_hours: resolved.timeout_hours,
            checkpoint_interval: resolved.checkpoint_interval,
            target_bucket: process.output.target_bucket.clone(),
            status: process.status().to_string(),
            current_iteration: process.progress.current_iteration,
            progress_percentage: process.progress_percentage(),
            error_message: process.progress.error_message.clone(),
            created_at: process.created_at.to_rfc3339(),
            started_at: process.started_at.map(|t| t.to_rfc3339()),
            completed_at: process.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Response wrapper for research depth preset info
#[derive(Debug, Serialize)]
pub struct ResearchPresetResponse {
    pub id: String,
    pub name: String,
    pub max_iterations: u32,
    pub timeout_hours: f32,
    pub checkpoint_interval: u32,
    pub description: String,
}

// ===== Research Process Commands =====

/// Start a new research process
#[tauri::command]
pub async fn start_research(
    input: StartResearchInput,
    state: State<'_, AppState>,
) -> Result<ResearchProcessResponse, String> {
    // Build the research brief
    let mut brief = ResearchBrief::new(&input.question);
    if let Some(ref context) = input.context {
        brief = brief.with_context(context);
    }
    if let Some(ref scope) = input.scope {
        brief = brief.with_scope(scope);
    }
    if let Some(ref constraints) = input.constraints {
        brief = brief.with_constraints(constraints.clone());
    }

    // Create the process
    let mut process = ResearchProcess::new(&input.name, brief, &input.agent_profile_id);

    // Set depth
    if let Some(custom) = input.custom_depth {
        process = process.with_custom_depth(CustomDepth::from(custom));
    } else if let Some(ref preset_str) = input.depth_preset {
        let preset: ResearchDepthPreset = preset_str
            .parse()
            .map_err(|_| format!("Invalid depth preset: {}", preset_str))?;
        process = process.with_preset(preset);
    }

    // Set output bucket if provided
    if let Some(ref bucket) = input.target_bucket {
        process = process.with_output(ResearchOutput::new(bucket));
    }

    // Start the process
    process.start();

    // Save to repository
    state
        .process_repo
        .create(process)
        .await
        .map(ResearchProcessResponse::from)
        .map_err(|e| e.to_string())
}

/// Pause a running research process
#[tauri::command]
pub async fn pause_research(
    id: String,
    state: State<'_, AppState>,
) -> Result<ResearchProcessResponse, String> {
    let process_id = ResearchProcessId::from_string(id);

    let mut process = state
        .process_repo
        .get_by_id(&process_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Research process not found: {}", process_id.as_str()))?;

    if process.status() != ResearchProcessStatus::Running {
        return Err(format!(
            "Cannot pause research in status: {}",
            process.status()
        ));
    }

    process.pause();

    state
        .process_repo
        .update(&process)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ResearchProcessResponse::from(process))
}

/// Resume a paused research process
#[tauri::command]
pub async fn resume_research(
    id: String,
    state: State<'_, AppState>,
) -> Result<ResearchProcessResponse, String> {
    let process_id = ResearchProcessId::from_string(id);

    let mut process = state
        .process_repo
        .get_by_id(&process_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Research process not found: {}", process_id.as_str()))?;

    if process.status() != ResearchProcessStatus::Paused {
        return Err(format!(
            "Cannot resume research in status: {}",
            process.status()
        ));
    }

    process.resume();

    state
        .process_repo
        .update(&process)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ResearchProcessResponse::from(process))
}

/// Stop/cancel a research process
#[tauri::command]
pub async fn stop_research(
    id: String,
    state: State<'_, AppState>,
) -> Result<ResearchProcessResponse, String> {
    let process_id = ResearchProcessId::from_string(id);

    let mut process = state
        .process_repo
        .get_by_id(&process_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Research process not found: {}", process_id.as_str()))?;

    if process.is_terminal() {
        return Err(format!(
            "Research process already completed with status: {}",
            process.status()
        ));
    }

    process.fail("Stopped by user");

    state
        .process_repo
        .update(&process)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ResearchProcessResponse::from(process))
}

/// Get all research processes (optionally filtered by status)
#[tauri::command]
pub async fn get_research_processes(
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ResearchProcessResponse>, String> {
    match status {
        Some(status_str) => {
            let parsed_status: ResearchProcessStatus = status_str
                .parse()
                .map_err(|_| format!("Invalid status: {}", status_str))?;
            state
                .process_repo
                .get_by_status(parsed_status)
                .await
                .map(|processes| {
                    processes
                        .into_iter()
                        .map(ResearchProcessResponse::from)
                        .collect()
                })
                .map_err(|e| e.to_string())
        }
        None => state
            .process_repo
            .get_all()
            .await
            .map(|processes| {
                processes
                    .into_iter()
                    .map(ResearchProcessResponse::from)
                    .collect()
            })
            .map_err(|e| e.to_string()),
    }
}

/// Get a single research process by ID
#[tauri::command]
pub async fn get_research_process(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<ResearchProcessResponse>, String> {
    let process_id = ResearchProcessId::from_string(id);
    state
        .process_repo
        .get_by_id(&process_id)
        .await
        .map(|opt| opt.map(ResearchProcessResponse::from))
        .map_err(|e| e.to_string())
}

/// Get available research depth presets
#[tauri::command]
pub async fn get_research_presets() -> Result<Vec<ResearchPresetResponse>, String> {
    Ok(ResearchDepthPreset::all()
        .iter()
        .map(|preset| {
            let depth = preset.to_custom_depth();
            let (name, description) = match preset {
                ResearchDepthPreset::QuickScan => (
                    "Quick Scan",
                    "Fast overview - 10 iterations, 30 min timeout",
                ),
                ResearchDepthPreset::Standard => (
                    "Standard",
                    "Thorough investigation - 50 iterations, 2 hrs timeout",
                ),
                ResearchDepthPreset::DeepDive => (
                    "Deep Dive",
                    "Comprehensive analysis - 200 iterations, 8 hrs timeout",
                ),
                ResearchDepthPreset::Exhaustive => (
                    "Exhaustive",
                    "Leave no stone unturned - 500 iterations, 24 hrs timeout",
                ),
            };
            ResearchPresetResponse {
                id: preset.to_string(),
                name: name.to_string(),
                max_iterations: depth.max_iterations,
                timeout_hours: depth.timeout_hours,
                checkpoint_interval: depth.checkpoint_interval,
                description: description.to_string(),
            }
        })
        .collect())
}
