use async_trait::async_trait;

use crate::ideation::IdeationEffortSettings;

/// Repository for ideation effort settings.
///
/// Follows the `execution_settings` pattern:
/// - `project_id = None`  → global row (project_id IS NULL in DB)
/// - `project_id = Some(s)` → per-project override row
#[async_trait]
pub trait IdeationEffortSettingsRepository: Send + Sync {
    /// Fetch the settings row for the given project_id.
    /// Returns `Ok(None)` if no row exists for that project_id.
    async fn get_by_project_id(
        &self,
        project_id: Option<&str>,
    ) -> Result<Option<IdeationEffortSettings>, Box<dyn std::error::Error>>;

    /// Upsert (insert-or-replace) the primary_effort and verifier_effort for a project_id.
    /// Returns the resulting row after the upsert.
    async fn upsert(
        &self,
        project_id: Option<&str>,
        primary_effort: &str,
        verifier_effort: &str,
    ) -> Result<IdeationEffortSettings, Box<dyn std::error::Error>>;
}
