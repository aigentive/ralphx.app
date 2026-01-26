// Tauri commands for task context operations
// Thin layer that delegates to TaskContextService and repositories

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{Artifact, ArtifactId, ProjectId, TaskContext, TaskId};
use crate::error::AppError;

/// Get rich context for a task including source proposal, plan, and related artifacts
#[tauri::command]
pub async fn get_task_context(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskContext, String> {
    use crate::application::TaskContextService;

    let task_id = TaskId::from_string(task_id);

    // Create service with current repositories
    let service = TaskContextService::new(
        state.task_repo.clone(),
        state.task_proposal_repo.clone(),
        state.artifact_repo.clone(),
        state.task_step_repo.clone(),
    );

    service
        .get_task_context(&task_id)
        .await
        .map_err(|e| match e {
            AppError::NotFound(msg) => msg,
            _ => format!("Failed to get task context: {}", e),
        })
}

/// Get full artifact content by ID
#[tauri::command]
pub async fn get_artifact_full(
    artifact_id: String,
    state: State<'_, AppState>,
) -> Result<Artifact, String> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| format!("Failed to get artifact: {}", e))?
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))
}

/// Get artifact at a specific version
#[tauri::command]
pub async fn get_artifact_version(
    artifact_id: String,
    version: u32,
    state: State<'_, AppState>,
) -> Result<Artifact, String> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    state
        .artifact_repo
        .get_by_id_at_version(&artifact_id, version)
        .await
        .map_err(|e| format!("Failed to get artifact version: {}", e))?
        .ok_or_else(|| format!("Artifact version not found: {} v{}", artifact_id, version))
}

/// Get all artifacts related to a specific artifact
#[tauri::command]
pub async fn get_related_artifacts(
    artifact_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Artifact>, String> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    state
        .artifact_repo
        .get_related(&artifact_id)
        .await
        .map_err(|e| format!("Failed to get related artifacts: {}", e))
}

/// Input for artifact search
#[derive(Debug, Deserialize)]
pub struct SearchArtifactsInput {
    pub project_id: String,
    pub query: String,
    pub artifact_types: Option<Vec<String>>,
}

/// Response for artifact search (summary view)
#[derive(Debug, Serialize)]
pub struct ArtifactSearchResult {
    pub id: String,
    pub title: String,
    pub artifact_type: String,
    pub current_version: u32,
    pub content_preview: String,
}

/// Search for artifacts by query and optional type filter
#[tauri::command]
pub async fn search_artifacts(
    input: SearchArtifactsInput,
    state: State<'_, AppState>,
) -> Result<Vec<ArtifactSearchResult>, String> {
    let _project_id = ProjectId::from_string(input.project_id);

    // For MVP, we'll search by type if specified, otherwise return empty
    // TODO: Add proper search index with full-text search for production
    let all_artifacts = if let Some(ref types) = input.artifact_types {
        let mut results = Vec::new();
        for type_str in types {
            let artifact_type = type_str
                .parse::<crate::domain::entities::ArtifactType>()
                .map_err(|_| format!("Invalid artifact type: {}", type_str))?;
            let artifacts = state
                .artifact_repo
                .get_by_type(artifact_type)
                .await
                .map_err(|e| format!("Failed to search artifacts: {}", e))?;
            results.extend(artifacts);
        }
        results
    } else {
        // If no types specified, we can't efficiently search all artifacts
        // Return empty for now - TODO: implement proper search
        Vec::new()
    };

    let query_lower = input.query.to_lowercase();
    let results: Vec<ArtifactSearchResult> = all_artifacts
        .into_iter()
        .filter(|artifact| {
            // Search in title and content
            let title_match = artifact.name.to_lowercase().contains(&query_lower);
            let content_match = match &artifact.content {
                crate::domain::entities::ArtifactContent::Inline { text } => {
                    text.to_lowercase().contains(&query_lower)
                }
                crate::domain::entities::ArtifactContent::File { path } => {
                    path.to_lowercase().contains(&query_lower)
                }
            };

            title_match || content_match
        })
        .map(|artifact| {
            let content_preview = create_content_preview(&artifact);
            ArtifactSearchResult {
                id: artifact.id.as_str().to_string(),
                title: artifact.name.clone(),
                artifact_type: artifact.artifact_type.to_string(),
                current_version: artifact.metadata.version,
                content_preview,
            }
        })
        .collect();

    Ok(results)
}

/// Helper to create a content preview (first 500 chars)
fn create_content_preview(artifact: &Artifact) -> String {
    use crate::domain::entities::ArtifactContent;

    let full_content = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { path } => {
            format!("[File artifact at: {}]", path)
        }
    };

    if full_content.len() <= 500 {
        full_content
    } else {
        format!("{}...", &full_content[..500])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ArtifactType;

    #[test]
    fn test_content_preview_short() {
        let artifact = Artifact::new_inline(
            "Test",
            ArtifactType::Specification,
            "Short content",
            "user",
        );
        let preview = create_content_preview(&artifact);
        assert_eq!(preview, "Short content");
    }

    #[test]
    fn test_content_preview_long() {
        let long_content = "x".repeat(600);
        let artifact = Artifact::new_inline(
            "Test",
            ArtifactType::Specification,
            long_content,
            "user",
        );
        let preview = create_content_preview(&artifact);
        assert_eq!(preview.len(), 503); // 500 + "..."
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn test_content_preview_file() {
        let artifact = Artifact::new_file(
            "Test",
            ArtifactType::Specification,
            "/path/to/file.md",
            "user",
        );
        let preview = create_content_preview(&artifact);
        assert!(preview.contains("/path/to/file.md"));
    }
}
