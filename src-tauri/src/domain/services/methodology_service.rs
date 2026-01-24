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
        let activated = self
            .methodology_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Methodology not found after activation: {}", id)))?;

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
        let deactivated = self
            .methodology_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Methodology not found after deactivation: {}", id)))?;

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
    pub async fn update_methodology(
        &self,
        methodology: &MethodologyExtension,
    ) -> AppResult<()> {
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
mod tests {
    use super::*;
    use crate::domain::entities::methodology::MethodologyPhase;
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::WorkflowColumn;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // ==================== Mock Methodology Repository ====================

    struct MockMethodologyRepository {
        methodologies: Mutex<HashMap<String, MethodologyExtension>>,
    }

    impl MockMethodologyRepository {
        fn new() -> Self {
            Self {
                methodologies: Mutex::new(HashMap::new()),
            }
        }

        async fn add_methodology(&self, methodology: MethodologyExtension) {
            let mut methodologies = self.methodologies.lock().await;
            methodologies.insert(methodology.id.as_str().to_string(), methodology);
        }
    }

    #[async_trait]
    impl MethodologyRepository for MockMethodologyRepository {
        async fn create(
            &self,
            methodology: MethodologyExtension,
        ) -> AppResult<MethodologyExtension> {
            self.add_methodology(methodology.clone()).await;
            Ok(methodology)
        }

        async fn get_by_id(
            &self,
            id: &MethodologyId,
        ) -> AppResult<Option<MethodologyExtension>> {
            let methodologies = self.methodologies.lock().await;
            Ok(methodologies.get(id.as_str()).cloned())
        }

        async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>> {
            let methodologies = self.methodologies.lock().await;
            Ok(methodologies.values().cloned().collect())
        }

        async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
            let methodologies = self.methodologies.lock().await;
            Ok(methodologies.values().find(|m| m.is_active).cloned())
        }

        async fn activate(&self, id: &MethodologyId) -> AppResult<()> {
            let mut methodologies = self.methodologies.lock().await;
            if let Some(methodology) = methodologies.get_mut(id.as_str()) {
                methodology.activate();
            }
            Ok(())
        }

        async fn deactivate(&self, id: &MethodologyId) -> AppResult<()> {
            let mut methodologies = self.methodologies.lock().await;
            if let Some(methodology) = methodologies.get_mut(id.as_str()) {
                methodology.deactivate();
            }
            Ok(())
        }

        async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()> {
            let mut methodologies = self.methodologies.lock().await;
            methodologies.insert(methodology.id.as_str().to_string(), methodology.clone());
            Ok(())
        }

        async fn delete(&self, id: &MethodologyId) -> AppResult<()> {
            let mut methodologies = self.methodologies.lock().await;
            methodologies.remove(id.as_str());
            Ok(())
        }

        async fn exists(&self, id: &MethodologyId) -> AppResult<bool> {
            let methodologies = self.methodologies.lock().await;
            Ok(methodologies.contains_key(id.as_str()))
        }
    }

    // ==================== Test Helpers ====================

    fn create_service() -> (MethodologyService<MockMethodologyRepository>, Arc<MockMethodologyRepository>) {
        let methodology_repo = Arc::new(MockMethodologyRepository::new());
        let service = MethodologyService::new(methodology_repo.clone());
        (service, methodology_repo)
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
            .with_phase(MethodologyPhase::new("analysis", "Analysis", 0))
    }

    // ==================== activate_methodology Tests ====================

    #[tokio::test]
    async fn activate_methodology_success() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.activate_methodology(&id).await;

        assert!(result.is_ok());
        let activation = result.unwrap();
        assert!(activation.methodology.is_active);
        assert_eq!(activation.agent_profiles.len(), 2);
        assert_eq!(activation.skills.len(), 2);
        assert!(activation.previous_methodology.is_none());
    }

    #[tokio::test]
    async fn activate_methodology_not_found() {
        let (service, _) = create_service();

        let id = MethodologyId::new();
        let result = service.activate_methodology(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn activate_methodology_already_active() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        methodology.activate();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.activate_methodology(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already active"));
    }

    #[tokio::test]
    async fn activate_methodology_deactivates_previous() {
        let (service, repo) = create_service();

        // Add first methodology and activate it
        let mut first = create_test_methodology();
        first.id = MethodologyId::from_string("first");
        first.activate();
        repo.add_methodology(first.clone()).await;

        // Add second methodology
        let mut second = create_test_methodology();
        second.id = MethodologyId::from_string("second");
        second.name = "Second Method".to_string();
        repo.add_methodology(second.clone()).await;

        // Activate second
        let result = service.activate_methodology(&second.id).await;

        assert!(result.is_ok());
        let activation = result.unwrap();
        assert_eq!(activation.previous_methodology, Some(first.id));

        // Verify first is now inactive
        let first_now = repo.get_by_id(&MethodologyId::from_string("first")).await.unwrap().unwrap();
        assert!(!first_now.is_active);
    }

    // ==================== deactivate_methodology Tests ====================

    #[tokio::test]
    async fn deactivate_methodology_success() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        methodology.activate();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.deactivate_methodology(&id).await;

        assert!(result.is_ok());
        let deactivated = result.unwrap();
        assert!(!deactivated.is_active);
    }

    #[tokio::test]
    async fn deactivate_methodology_not_found() {
        let (service, _) = create_service();

        let id = MethodologyId::new();
        let result = service.deactivate_methodology(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn deactivate_methodology_not_active() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.deactivate_methodology(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not active"));
    }

    // ==================== get_active Tests ====================

    #[tokio::test]
    async fn get_active_none() {
        let (service, repo) = create_service();

        // Add inactive methodology
        let methodology = create_test_methodology();
        repo.add_methodology(methodology).await;

        let result = service.get_active().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_active_some() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        methodology.activate();
        repo.add_methodology(methodology.clone()).await;

        let result = service.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, methodology.id);
    }

    // ==================== Repository Method Tests ====================

    #[tokio::test]
    async fn get_methodology_found() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_methodology(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn get_methodology_not_found() {
        let (service, _) = create_service();

        let id = MethodologyId::new();
        let result = service.get_methodology(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_all_methodologies_empty() {
        let (service, _) = create_service();

        let result = service.get_all_methodologies().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_all_methodologies_returns_all() {
        let (service, repo) = create_service();

        let mut m1 = create_test_methodology();
        m1.id = MethodologyId::from_string("m1");
        let mut m2 = create_test_methodology();
        m2.id = MethodologyId::from_string("m2");

        repo.add_methodology(m1).await;
        repo.add_methodology(m2).await;

        let result = service.get_all_methodologies().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn create_methodology_persists() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();

        let result = service.create_methodology(methodology).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn update_methodology_modifies() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology.clone()).await;

        methodology.name = "Updated Name".to_string();
        service.update_methodology(&methodology).await.unwrap();

        let found = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
    }

    #[tokio::test]
    async fn delete_methodology_removes() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        service.delete_methodology(&id).await.unwrap();

        let found = repo.get_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn delete_methodology_fails_if_active() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        methodology.activate();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.delete_methodology(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("active"));
    }

    #[tokio::test]
    async fn methodology_exists_true() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.methodology_exists(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn methodology_exists_false() {
        let (service, _) = create_service();

        let id = MethodologyId::new();
        let result = service.methodology_exists(&id).await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==================== switch_methodology Tests ====================

    #[tokio::test]
    async fn switch_methodology_works() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.switch_methodology(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().methodology.is_active);
    }

    // ==================== get_* Methods Tests ====================

    #[tokio::test]
    async fn get_workflow_found() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_workflow(&id).await;
        assert!(result.is_ok());
        let workflow = result.unwrap();
        assert!(workflow.is_some());
        assert_eq!(workflow.unwrap().columns.len(), 3);
    }

    #[tokio::test]
    async fn get_workflow_not_found() {
        let (service, _) = create_service();

        let id = MethodologyId::new();
        let result = service.get_workflow(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_agent_profiles_found() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_agent_profiles(&id).await;
        assert!(result.is_ok());
        let profiles = result.unwrap();
        assert!(profiles.is_some());
        assert_eq!(profiles.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_skills_found() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_skills(&id).await;
        assert!(result.is_ok());
        let skills = result.unwrap();
        assert!(skills.is_some());
        assert_eq!(skills.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_phases_found() {
        let (service, repo) = create_service();

        let methodology = create_test_methodology();
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_phases(&id).await;
        assert!(result.is_ok());
        let phases = result.unwrap();
        assert!(phases.is_some());
        assert_eq!(phases.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_templates_found() {
        let (service, repo) = create_service();

        let mut methodology = create_test_methodology();
        methodology = methodology.with_template(
            crate::domain::entities::methodology::MethodologyTemplate::new("prd", "templates/prd.md"),
        );
        let id = methodology.id.clone();
        repo.add_methodology(methodology).await;

        let result = service.get_templates(&id).await;
        assert!(result.is_ok());
        let templates = result.unwrap();
        assert!(templates.is_some());
        assert_eq!(templates.unwrap().len(), 1);
    }

    // ==================== Built-in Methodology Tests ====================

    #[test]
    fn get_builtin_methodologies_returns_two() {
        let builtins = MethodologyService::<MockMethodologyRepository>::get_builtin_methodologies();
        assert_eq!(builtins.len(), 2);
    }

    #[test]
    fn get_bmad_returns_bmad() {
        let bmad = MethodologyService::<MockMethodologyRepository>::get_bmad();
        assert_eq!(bmad.id.as_str(), "bmad-method");
        assert_eq!(bmad.name, "BMAD Method");
        assert_eq!(bmad.agent_profiles.len(), 8);
    }

    #[test]
    fn get_gsd_returns_gsd() {
        let gsd = MethodologyService::<MockMethodologyRepository>::get_gsd();
        assert_eq!(gsd.id.as_str(), "gsd-method");
        assert_eq!(gsd.name, "GSD (Get Shit Done)");
        assert_eq!(gsd.agent_profiles.len(), 11);
    }

    // ==================== seed_builtins Tests ====================

    #[tokio::test]
    async fn seed_builtins_seeds_both() {
        let (service, _) = create_service();

        let result = service.seed_builtins().await;
        assert!(result.is_ok());
        let seeded = result.unwrap();
        assert_eq!(seeded.len(), 2);
    }

    #[tokio::test]
    async fn seed_builtins_skips_existing() {
        let (service, repo) = create_service();

        // Pre-add BMAD
        let bmad = MethodologyExtension::bmad();
        repo.add_methodology(bmad).await;

        let result = service.seed_builtins().await;
        assert!(result.is_ok());
        let seeded = result.unwrap();
        // Only GSD should be seeded
        assert_eq!(seeded.len(), 1);
        assert_eq!(seeded[0].name, "GSD (Get Shit Done)");
    }

    #[tokio::test]
    async fn seed_builtins_idempotent() {
        let (service, _) = create_service();

        // First seed
        let first = service.seed_builtins().await.unwrap();
        assert_eq!(first.len(), 2);

        // Second seed should seed nothing
        let second = service.seed_builtins().await.unwrap();
        assert!(second.is_empty());
    }

    // ==================== Integration Scenario Tests ====================

    #[tokio::test]
    async fn methodology_lifecycle_scenario() {
        let (service, repo) = create_service();

        // Seed builtins
        service.seed_builtins().await.unwrap();

        // Get all
        let all = service.get_all_methodologies().await.unwrap();
        assert_eq!(all.len(), 2);

        // Activate BMAD
        let bmad_id = MethodologyId::from_string("bmad-method");
        let activation = service.activate_methodology(&bmad_id).await.unwrap();
        assert!(activation.methodology.is_active);
        assert_eq!(activation.agent_profiles.len(), 8);

        // Verify BMAD is active
        let active = service.get_active().await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id.as_str(), "bmad-method");

        // Switch to GSD
        let gsd_id = MethodologyId::from_string("gsd-method");
        let switch = service.switch_methodology(&gsd_id).await.unwrap();
        assert_eq!(switch.previous_methodology, Some(bmad_id.clone()));
        assert_eq!(switch.agent_profiles.len(), 11);

        // Verify GSD is now active and BMAD is not
        let active = service.get_active().await.unwrap();
        assert_eq!(active.unwrap().id.as_str(), "gsd-method");

        let bmad_now = repo.get_by_id(&bmad_id).await.unwrap().unwrap();
        assert!(!bmad_now.is_active);

        // Deactivate GSD
        service.deactivate_methodology(&gsd_id).await.unwrap();

        // Verify no active methodology
        let active = service.get_active().await.unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn custom_methodology_scenario() {
        let (service, _) = create_service();

        // Create a custom methodology
        let custom = create_test_methodology();
        let id = custom.id.clone();

        service.create_methodology(custom).await.unwrap();

        // Activate it
        let activation = service.activate_methodology(&id).await.unwrap();
        assert!(activation.methodology.is_active);

        // Get its components
        let workflow = service.get_workflow(&id).await.unwrap().unwrap();
        assert_eq!(workflow.columns.len(), 3);

        let profiles = service.get_agent_profiles(&id).await.unwrap().unwrap();
        assert_eq!(profiles.len(), 2);

        let skills = service.get_skills(&id).await.unwrap().unwrap();
        assert_eq!(skills.len(), 2);

        // Deactivate and delete
        service.deactivate_methodology(&id).await.unwrap();
        service.delete_methodology(&id).await.unwrap();

        // Verify deleted
        let exists = service.methodology_exists(&id).await.unwrap();
        assert!(!exists);
    }
}
