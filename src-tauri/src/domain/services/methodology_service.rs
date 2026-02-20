// MethodologyService - domain service for methodology management
//
// Provides business logic for:
// - Activating and deactivating methodologies
// - Switching between methodologies
// - Managing workflow, agent profile, and skill associations
// - Retrieving built-in methodologies

use std::sync::Arc;

use crate::domain::entities::methodology::{
    MethodologyExtension, MethodologyId, MethodologyPhase, MethodologyTemplate,
};
use crate::domain::entities::workflow::WorkflowSchema;
use crate::domain::repositories::MethodologyRepository;
use crate::error::{AppError, AppResult};

/// Result of activating a methodology
#[derive(Debug, Clone)]
pub struct MethodologyActivationResult {
    /// The activated methodology
    pub methodology: MethodologyExtension,
    /// The workflow to apply
    pub workflow: WorkflowSchema,
    /// Agent profile IDs to load
    pub agent_profiles: Vec<String>,
    /// Skill paths to inject
    pub skills: Vec<String>,
    /// The previously active methodology (if any) that was deactivated
    pub previous_methodology: Option<MethodologyId>,
}

/// Service for methodology management
pub struct MethodologyService<R: MethodologyRepository> {
    methodology_repo: Arc<R>,
}

impl<R: MethodologyRepository> MethodologyService<R> {
    /// Create a new MethodologyService with the given repository
    pub fn new(methodology_repo: Arc<R>) -> Self {
        Self { methodology_repo }
    }

    /// Activate a methodology, deactivating any currently active one
    pub async fn activate_methodology(
        &self,
        id: &MethodologyId,
    ) -> AppResult<MethodologyActivationResult> {
        // Get the methodology to activate
        let methodology = self
            .methodology_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Methodology not found: {}", id)))?;

        if methodology.is_active {
            return Err(AppError::Validation(format!(
                "Methodology '{}' is already active",
                methodology.name
            )));
        }

        // Get current active methodology (if any)
        let previous = self.methodology_repo.get_active().await?;
        let previous_id = previous.as_ref().map(|m| m.id.clone());

        // Deactivate previous if exists
        if let Some(prev) = &previous {
            self.methodology_repo.deactivate(&prev.id).await?;
        }

        // Activate the new methodology
        self.methodology_repo.activate(id).await?;

        // Re-fetch to get updated state
        let activated = self.methodology_repo.get_by_id(id).await?.ok_or_else(|| {
            AppError::NotFound(format!("Methodology not found after activation: {}", id))
        })?;

        Ok(MethodologyActivationResult {
            workflow: activated.workflow.clone(),
            agent_profiles: activated.agent_profiles.clone(),
            skills: activated.skills.clone(),
            methodology: activated,
            previous_methodology: previous_id,
        })
    }

    /// Deactivate a methodology
    pub async fn deactivate_methodology(
        &self,
        id: &MethodologyId,
    ) -> AppResult<MethodologyExtension> {
        let methodology = self
            .methodology_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Methodology not found: {}", id)))?;

        if !methodology.is_active {
            return Err(AppError::Validation(format!(
                "Methodology '{}' is not active",
                methodology.name
            )));
        }

        self.methodology_repo.deactivate(id).await?;

        // Re-fetch to get updated state
        let deactivated = self.methodology_repo.get_by_id(id).await?.ok_or_else(|| {
            AppError::NotFound(format!("Methodology not found after deactivation: {}", id))
        })?;

        Ok(deactivated)
    }

    /// Get the currently active methodology (if any)
    pub async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
        self.methodology_repo.get_active().await
    }

    /// Get a methodology by ID
    pub async fn get_methodology(
        &self,
        id: &MethodologyId,
    ) -> AppResult<Option<MethodologyExtension>> {
        self.methodology_repo.get_by_id(id).await
    }

    /// Get all methodologies
    pub async fn get_all_methodologies(&self) -> AppResult<Vec<MethodologyExtension>> {
        self.methodology_repo.get_all().await
    }

    /// Create a new methodology
    pub async fn create_methodology(
        &self,
        methodology: MethodologyExtension,
    ) -> AppResult<MethodologyExtension> {
        self.methodology_repo.create(methodology).await
    }

    /// Update a methodology
    pub async fn update_methodology(&self, methodology: &MethodologyExtension) -> AppResult<()> {
        self.methodology_repo.update(methodology).await
    }

    /// Delete a methodology
    pub async fn delete_methodology(&self, id: &MethodologyId) -> AppResult<()> {
        // Cannot delete active methodology
        let methodology = self
            .methodology_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Methodology not found: {}", id)))?;

        if methodology.is_active {
            return Err(AppError::Validation(
                "Cannot delete an active methodology. Deactivate it first.".to_string(),
            ));
        }

        self.methodology_repo.delete(id).await
    }

    /// Check if a methodology exists
    pub async fn methodology_exists(&self, id: &MethodologyId) -> AppResult<bool> {
        self.methodology_repo.exists(id).await
    }

    /// Switch to a different methodology (convenience method for activate)
    pub async fn switch_methodology(
        &self,
        new_id: &MethodologyId,
    ) -> AppResult<MethodologyActivationResult> {
        self.activate_methodology(new_id).await
    }

    /// Get the workflow for a methodology
    pub async fn get_workflow(&self, id: &MethodologyId) -> AppResult<Option<WorkflowSchema>> {
        let methodology = self.methodology_repo.get_by_id(id).await?;
        Ok(methodology.map(|m| m.workflow))
    }

    /// Get the agent profiles for a methodology
    pub async fn get_agent_profiles(&self, id: &MethodologyId) -> AppResult<Option<Vec<String>>> {
        let methodology = self.methodology_repo.get_by_id(id).await?;
        Ok(methodology.map(|m| m.agent_profiles))
    }

    /// Get the skills for a methodology
    pub async fn get_skills(&self, id: &MethodologyId) -> AppResult<Option<Vec<String>>> {
        let methodology = self.methodology_repo.get_by_id(id).await?;
        Ok(methodology.map(|m| m.skills))
    }

    /// Get the phases for a methodology
    pub async fn get_phases(&self, id: &MethodologyId) -> AppResult<Option<Vec<MethodologyPhase>>> {
        let methodology = self.methodology_repo.get_by_id(id).await?;
        Ok(methodology.map(|m| m.phases))
    }

    /// Get the templates for a methodology
    pub async fn get_templates(
        &self,
        id: &MethodologyId,
    ) -> AppResult<Option<Vec<MethodologyTemplate>>> {
        let methodology = self.methodology_repo.get_by_id(id).await?;
        Ok(methodology.map(|m| m.templates))
    }

    /// Get built-in methodologies (BMAD and GSD)
    pub fn get_builtin_methodologies() -> Vec<MethodologyExtension> {
        MethodologyExtension::builtin_methodologies()
    }

    /// Get the BMAD methodology
    pub fn get_bmad() -> MethodologyExtension {
        MethodologyExtension::bmad()
    }

    /// Get the GSD methodology
    pub fn get_gsd() -> MethodologyExtension {
        MethodologyExtension::gsd()
    }

    /// Seed built-in methodologies into the repository
    pub async fn seed_builtins(&self) -> AppResult<Vec<MethodologyExtension>> {
        let mut seeded = vec![];

        for builtin in Self::get_builtin_methodologies() {
            if !self.methodology_repo.exists(&builtin.id).await? {
                let created = self.methodology_repo.create(builtin).await?;
                seeded.push(created);
            }
        }

        Ok(seeded)
    }
}

#[cfg(test)]
#[path = "methodology_service_tests.rs"]
mod tests;
