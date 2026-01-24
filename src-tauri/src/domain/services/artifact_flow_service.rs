// ArtifactFlowService - domain service for artifact flow automation
//
// Provides business logic for:
// - Evaluating artifact flow triggers on events
// - Executing flow steps (copy, spawn process)
// - Managing flow registration and state

use std::sync::Arc;

use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactFlow, ArtifactFlowContext, ArtifactFlowEngine,
    ArtifactFlowEvaluation, ArtifactFlowId, ArtifactFlowStep,
};
use crate::domain::repositories::ArtifactFlowRepository;
use crate::error::AppResult;

/// Result of executing a flow step
#[derive(Debug, Clone)]
pub enum StepExecutionResult {
    /// Copy step executed successfully
    Copied {
        artifact_id: String,
        target_bucket: ArtifactBucketId,
    },
    /// Spawn process step queued (process creation handled externally)
    ProcessSpawned {
        process_type: String,
        agent_profile: String,
    },
}

/// Result of executing a complete flow
#[derive(Debug, Clone)]
pub struct FlowExecutionResult {
    /// The flow that was executed
    pub flow_id: ArtifactFlowId,
    /// The flow name
    pub flow_name: String,
    /// Results of each step
    pub step_results: Vec<StepExecutionResult>,
}

/// Service for artifact flow automation
pub struct ArtifactFlowService<R: ArtifactFlowRepository> {
    flow_repo: Arc<R>,
    engine: ArtifactFlowEngine,
}

impl<R: ArtifactFlowRepository> ArtifactFlowService<R> {
    /// Create a new ArtifactFlowService with the given repository
    pub fn new(flow_repo: Arc<R>) -> Self {
        Self {
            flow_repo,
            engine: ArtifactFlowEngine::new(),
        }
    }

    /// Load all active flows from the repository into the engine
    pub async fn load_active_flows(&mut self) -> AppResult<usize> {
        let flows = self.flow_repo.get_active().await?;
        let count = flows.len();
        self.engine = ArtifactFlowEngine::new();
        self.engine.register_flows(flows);
        Ok(count)
    }

    /// Register a flow with the engine (does not persist)
    pub fn register_flow(&mut self, flow: ArtifactFlow) {
        self.engine.register_flow(flow);
    }

    /// Get the current flow count in the engine
    pub fn flow_count(&self) -> usize {
        self.engine.flow_count()
    }

    /// Evaluate flows when an artifact is created
    pub fn on_artifact_created(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_artifact_created(artifact)
    }

    /// Evaluate flows when a task is completed
    pub fn on_task_completed(
        &self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_task_completed(task_id, artifact)
    }

    /// Evaluate flows when a process is completed
    pub fn on_process_completed(
        &self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_process_completed(process_id, artifact)
    }

    /// Evaluate flows for a given context
    pub fn evaluate_flows(&self, context: &ArtifactFlowContext) -> Vec<ArtifactFlowEvaluation> {
        self.engine.evaluate_triggers(context)
    }

    /// Execute the steps of a flow evaluation.
    /// Returns the results of each step execution.
    /// Note: The actual artifact copy and process spawn are handled by the caller,
    /// as this service does not have direct access to artifact/process repositories.
    pub fn execute_steps(
        &self,
        evaluation: &ArtifactFlowEvaluation,
        artifact: &Artifact,
    ) -> Vec<StepExecutionResult> {
        evaluation
            .steps
            .iter()
            .map(|step| match step {
                ArtifactFlowStep::Copy { to_bucket } => StepExecutionResult::Copied {
                    artifact_id: artifact.id.as_str().to_string(),
                    target_bucket: to_bucket.clone(),
                },
                ArtifactFlowStep::SpawnProcess {
                    process_type,
                    agent_profile,
                } => StepExecutionResult::ProcessSpawned {
                    process_type: process_type.clone(),
                    agent_profile: agent_profile.clone(),
                },
            })
            .collect()
    }

    /// Execute all steps for all matching flow evaluations.
    /// Returns execution results for each flow.
    pub fn execute_all_flows(
        &self,
        evaluations: &[ArtifactFlowEvaluation],
        artifact: &Artifact,
    ) -> Vec<FlowExecutionResult> {
        evaluations
            .iter()
            .map(|eval| FlowExecutionResult {
                flow_id: eval.flow_id.clone(),
                flow_name: eval.flow_name.clone(),
                step_results: self.execute_steps(eval, artifact),
            })
            .collect()
    }

    /// Get a flow from the repository by ID
    pub async fn get_flow(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
        self.flow_repo.get_by_id(id).await
    }

    /// Get all flows from the repository
    pub async fn get_all_flows(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.flow_repo.get_all().await
    }

    /// Get all active flows from the repository
    pub async fn get_active_flows(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.flow_repo.get_active().await
    }

    /// Create a new flow in the repository
    pub async fn create_flow(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
        self.flow_repo.create(flow).await
    }

    /// Update a flow in the repository
    pub async fn update_flow(&self, flow: &ArtifactFlow) -> AppResult<()> {
        self.flow_repo.update(flow).await
    }

    /// Delete a flow from the repository
    pub async fn delete_flow(&self, id: &ArtifactFlowId) -> AppResult<()> {
        self.flow_repo.delete(id).await
    }

    /// Set the active state of a flow
    pub async fn set_flow_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
        self.flow_repo.set_active(id, is_active).await
    }

    /// Check if a flow exists in the repository
    pub async fn flow_exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
        self.flow_repo.exists(id).await
    }

    /// Process an artifact_created event: evaluate triggers and return execution plan
    pub async fn process_artifact_created(
        &mut self,
        artifact: &Artifact,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_artifact_created(artifact);

        // Execute all matching flows
        Ok(self.execute_all_flows(&evaluations, artifact))
    }

    /// Process a task_completed event: evaluate triggers and return execution plan
    pub async fn process_task_completed(
        &mut self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_task_completed(task_id, artifact);

        // Execute all matching flows (if artifact provided)
        if let Some(artifact) = artifact {
            Ok(self.execute_all_flows(&evaluations, artifact))
        } else {
            Ok(evaluations
                .into_iter()
                .map(|eval| FlowExecutionResult {
                    flow_id: eval.flow_id,
                    flow_name: eval.flow_name,
                    step_results: vec![],
                })
                .collect())
        }
    }

    /// Process a process_completed event: evaluate triggers and return execution plan
    pub async fn process_process_completed(
        &mut self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_process_completed(process_id, artifact);

        // Execute all matching flows (if artifact provided)
        if let Some(artifact) = artifact {
            Ok(self.execute_all_flows(&evaluations, artifact))
        } else {
            Ok(evaluations
                .into_iter()
                .map(|eval| FlowExecutionResult {
                    flow_id: eval.flow_id,
                    flow_name: eval.flow_name,
                    step_results: vec![],
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        ArtifactBucketId, ArtifactContent, ArtifactFlowFilter, ArtifactId, ArtifactMetadata,
        ArtifactType,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // ==================== Mock Flow Repository ====================

    struct MockFlowRepository {
        flows: Mutex<HashMap<String, ArtifactFlow>>,
    }

    impl MockFlowRepository {
        fn new() -> Self {
            Self {
                flows: Mutex::new(HashMap::new()),
            }
        }

        async fn add_flow(&self, flow: ArtifactFlow) {
            let mut flows = self.flows.lock().await;
            flows.insert(flow.id.as_str().to_string(), flow);
        }
    }

    #[async_trait]
    impl ArtifactFlowRepository for MockFlowRepository {
        async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
            self.add_flow(flow.clone()).await;
            Ok(flow)
        }

        async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
            let flows = self.flows.lock().await;
            Ok(flows.get(id.as_str()).cloned())
        }

        async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>> {
            let flows = self.flows.lock().await;
            Ok(flows.values().cloned().collect())
        }

        async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>> {
            let flows = self.flows.lock().await;
            Ok(flows.values().filter(|f| f.is_active).cloned().collect())
        }

        async fn update(&self, flow: &ArtifactFlow) -> AppResult<()> {
            let mut flows = self.flows.lock().await;
            flows.insert(flow.id.as_str().to_string(), flow.clone());
            Ok(())
        }

        async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()> {
            let mut flows = self.flows.lock().await;
            flows.remove(id.as_str());
            Ok(())
        }

        async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
            let mut flows = self.flows.lock().await;
            if let Some(flow) = flows.get_mut(id.as_str()) {
                flow.is_active = is_active;
            }
            Ok(())
        }

        async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
            let flows = self.flows.lock().await;
            Ok(flows.contains_key(id.as_str()))
        }
    }

    // ==================== Test Helpers ====================

    fn create_service() -> (ArtifactFlowService<MockFlowRepository>, Arc<MockFlowRepository>) {
        let flow_repo = Arc::new(MockFlowRepository::new());
        let service = ArtifactFlowService::new(flow_repo.clone());
        (service, flow_repo)
    }

    fn create_test_artifact(artifact_type: ArtifactType, bucket_id: Option<&str>) -> Artifact {
        Artifact {
            id: ArtifactId::new(),
            artifact_type,
            name: "Test Artifact".to_string(),
            content: ArtifactContent::inline("Test content"),
            metadata: ArtifactMetadata::new("user"),
            derived_from: vec![],
            bucket_id: bucket_id.map(ArtifactBucketId::from_string),
        }
    }

    fn create_basic_flow() -> ArtifactFlow {
        use crate::domain::entities::ArtifactFlowTrigger;
        ArtifactFlow::new("Basic Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "target-bucket",
            )))
    }

    fn create_filtered_flow() -> ArtifactFlow {
        use crate::domain::entities::ArtifactFlowTrigger;
        ArtifactFlow::new(
            "Filtered Flow",
            ArtifactFlowTrigger::on_artifact_created().with_filter(
                ArtifactFlowFilter::new()
                    .with_artifact_types(vec![ArtifactType::Recommendations])
                    .with_source_bucket(ArtifactBucketId::from_string("research-outputs")),
            ),
        )
        .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
            "prd-library",
        )))
        .with_step(ArtifactFlowStep::spawn_process(
            "task_decomposition",
            "orchestrator",
        ))
    }

    fn create_task_completed_flow() -> ArtifactFlow {
        use crate::domain::entities::ArtifactFlowTrigger;
        ArtifactFlow::new("Task Flow", ArtifactFlowTrigger::on_task_completed())
            .with_step(ArtifactFlowStep::spawn_process("archive", "system"))
    }

    fn create_process_completed_flow() -> ArtifactFlow {
        use crate::domain::entities::ArtifactFlowTrigger;
        ArtifactFlow::new("Process Flow", ArtifactFlowTrigger::on_process_completed())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "archive-bucket",
            )))
    }

    // ==================== Service Creation Tests ====================

    #[test]
    fn service_new_creates_empty_engine() {
        let (service, _) = create_service();
        assert_eq!(service.flow_count(), 0);
    }

    #[test]
    fn service_register_flow_increases_count() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow());
        assert_eq!(service.flow_count(), 1);
    }

    #[test]
    fn service_register_multiple_flows() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow());
        service.register_flow(create_filtered_flow());
        service.register_flow(create_task_completed_flow());
        assert_eq!(service.flow_count(), 3);
    }

    // ==================== load_active_flows Tests ====================

    #[tokio::test]
    async fn load_active_flows_from_empty_repo() {
        let (mut service, _) = create_service();
        let count = service.load_active_flows().await.unwrap();
        assert_eq!(count, 0);
        assert_eq!(service.flow_count(), 0);
    }

    #[tokio::test]
    async fn load_active_flows_loads_all_active() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_basic_flow()).await;
        flow_repo.add_flow(create_filtered_flow()).await;

        let count = service.load_active_flows().await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(service.flow_count(), 2);
    }

    #[tokio::test]
    async fn load_active_flows_skips_inactive() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_basic_flow()).await;
        flow_repo
            .add_flow(create_filtered_flow().set_active(false))
            .await;

        let count = service.load_active_flows().await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(service.flow_count(), 1);
    }

    #[tokio::test]
    async fn load_active_flows_replaces_existing() {
        let (mut service, flow_repo) = create_service();

        // First load
        flow_repo.add_flow(create_basic_flow()).await;
        service.load_active_flows().await.unwrap();
        assert_eq!(service.flow_count(), 1);

        // Add more and reload
        flow_repo.add_flow(create_filtered_flow()).await;
        service.load_active_flows().await.unwrap();
        assert_eq!(service.flow_count(), 2);
    }

    // ==================== on_artifact_created Tests ====================

    #[test]
    fn on_artifact_created_no_flows_returns_empty() {
        let (service, _) = create_service();
        let artifact = create_test_artifact(ArtifactType::Prd, None);

        let evaluations = service.on_artifact_created(&artifact);
        assert!(evaluations.is_empty());
    }

    #[test]
    fn on_artifact_created_basic_flow_matches() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow());

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluations = service.on_artifact_created(&artifact);

        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].flow_name, "Basic Flow");
    }

    #[test]
    fn on_artifact_created_filtered_flow_matches() {
        let (mut service, _) = create_service();
        service.register_flow(create_filtered_flow());

        let artifact = create_test_artifact(ArtifactType::Recommendations, Some("research-outputs"));
        let evaluations = service.on_artifact_created(&artifact);

        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].flow_name, "Filtered Flow");
        assert_eq!(evaluations[0].steps.len(), 2);
    }

    #[test]
    fn on_artifact_created_filtered_flow_no_match_wrong_type() {
        let (mut service, _) = create_service();
        service.register_flow(create_filtered_flow());

        let artifact = create_test_artifact(ArtifactType::Findings, Some("research-outputs"));
        let evaluations = service.on_artifact_created(&artifact);

        assert!(evaluations.is_empty());
    }

    #[test]
    fn on_artifact_created_filtered_flow_no_match_wrong_bucket() {
        let (mut service, _) = create_service();
        service.register_flow(create_filtered_flow());

        let artifact = create_test_artifact(ArtifactType::Recommendations, Some("other-bucket"));
        let evaluations = service.on_artifact_created(&artifact);

        assert!(evaluations.is_empty());
    }

    #[test]
    fn on_artifact_created_multiple_flows_match() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow());

        // Create another basic flow
        let mut flow2 = create_basic_flow();
        flow2.id = ArtifactFlowId::new();
        flow2.name = "Another Flow".to_string();
        service.register_flow(flow2);

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluations = service.on_artifact_created(&artifact);

        assert_eq!(evaluations.len(), 2);
    }

    // ==================== on_task_completed Tests ====================

    #[test]
    fn on_task_completed_matches_task_flow() {
        let (mut service, _) = create_service();
        service.register_flow(create_task_completed_flow());

        let artifact = create_test_artifact(ArtifactType::CodeChange, None);
        let evaluations = service.on_task_completed("task-1", Some(&artifact));

        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].flow_name, "Task Flow");
    }

    #[test]
    fn on_task_completed_no_match_for_artifact_flow() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow()); // artifact_created trigger

        let artifact = create_test_artifact(ArtifactType::CodeChange, None);
        let evaluations = service.on_task_completed("task-1", Some(&artifact));

        assert!(evaluations.is_empty());
    }

    #[test]
    fn on_task_completed_without_artifact() {
        let (mut service, _) = create_service();
        service.register_flow(create_task_completed_flow());

        let evaluations = service.on_task_completed("task-1", None);
        assert_eq!(evaluations.len(), 1);
    }

    // ==================== on_process_completed Tests ====================

    #[test]
    fn on_process_completed_matches_process_flow() {
        let (mut service, _) = create_service();
        service.register_flow(create_process_completed_flow());

        let artifact = create_test_artifact(ArtifactType::Findings, None);
        let evaluations = service.on_process_completed("process-1", Some(&artifact));

        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].flow_name, "Process Flow");
    }

    #[test]
    fn on_process_completed_no_match_for_task_flow() {
        let (mut service, _) = create_service();
        service.register_flow(create_task_completed_flow());

        let artifact = create_test_artifact(ArtifactType::Findings, None);
        let evaluations = service.on_process_completed("process-1", Some(&artifact));

        assert!(evaluations.is_empty());
    }

    // ==================== evaluate_flows Tests ====================

    #[test]
    fn evaluate_flows_with_context() {
        let (mut service, _) = create_service();
        service.register_flow(create_basic_flow());

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let context = ArtifactFlowContext::artifact_created(artifact);

        let evaluations = service.evaluate_flows(&context);
        assert_eq!(evaluations.len(), 1);
    }

    #[test]
    fn evaluate_flows_task_context() {
        let (mut service, _) = create_service();
        service.register_flow(create_task_completed_flow());

        let artifact = create_test_artifact(ArtifactType::CodeChange, None);
        let context = ArtifactFlowContext::task_completed("task-1", Some(artifact));

        let evaluations = service.evaluate_flows(&context);
        assert_eq!(evaluations.len(), 1);
    }

    // ==================== execute_steps Tests ====================

    #[test]
    fn execute_steps_copy_step() {
        let (service, _) = create_service();

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluation = ArtifactFlowEvaluation {
            flow_id: ArtifactFlowId::new(),
            flow_name: "Test".to_string(),
            steps: vec![ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "target",
            ))],
        };

        let results = service.execute_steps(&evaluation, &artifact);

        assert_eq!(results.len(), 1);
        match &results[0] {
            StepExecutionResult::Copied {
                artifact_id,
                target_bucket,
            } => {
                assert_eq!(artifact_id, artifact.id.as_str());
                assert_eq!(target_bucket.as_str(), "target");
            }
            _ => panic!("Expected Copied result"),
        }
    }

    #[test]
    fn execute_steps_spawn_process_step() {
        let (service, _) = create_service();

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluation = ArtifactFlowEvaluation {
            flow_id: ArtifactFlowId::new(),
            flow_name: "Test".to_string(),
            steps: vec![ArtifactFlowStep::spawn_process(
                "task_decomposition",
                "orchestrator",
            )],
        };

        let results = service.execute_steps(&evaluation, &artifact);

        assert_eq!(results.len(), 1);
        match &results[0] {
            StepExecutionResult::ProcessSpawned {
                process_type,
                agent_profile,
            } => {
                assert_eq!(process_type, "task_decomposition");
                assert_eq!(agent_profile, "orchestrator");
            }
            _ => panic!("Expected ProcessSpawned result"),
        }
    }

    #[test]
    fn execute_steps_multiple_steps() {
        let (service, _) = create_service();

        let artifact = create_test_artifact(ArtifactType::Recommendations, Some("research-outputs"));
        let evaluation = ArtifactFlowEvaluation {
            flow_id: ArtifactFlowId::new(),
            flow_name: "Research to Dev".to_string(),
            steps: vec![
                ArtifactFlowStep::copy(ArtifactBucketId::from_string("prd-library")),
                ArtifactFlowStep::spawn_process("task_decomposition", "orchestrator"),
            ],
        };

        let results = service.execute_steps(&evaluation, &artifact);

        assert_eq!(results.len(), 2);
        assert!(matches!(results[0], StepExecutionResult::Copied { .. }));
        assert!(matches!(
            results[1],
            StepExecutionResult::ProcessSpawned { .. }
        ));
    }

    // ==================== execute_all_flows Tests ====================

    #[test]
    fn execute_all_flows_empty_evaluations() {
        let (service, _) = create_service();
        let artifact = create_test_artifact(ArtifactType::Prd, None);

        let results = service.execute_all_flows(&[], &artifact);
        assert!(results.is_empty());
    }

    #[test]
    fn execute_all_flows_single_evaluation() {
        let (service, _) = create_service();

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluations = vec![ArtifactFlowEvaluation {
            flow_id: ArtifactFlowId::new(),
            flow_name: "Test".to_string(),
            steps: vec![ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "target",
            ))],
        }];

        let results = service.execute_all_flows(&evaluations, &artifact);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].flow_name, "Test");
        assert_eq!(results[0].step_results.len(), 1);
    }

    #[test]
    fn execute_all_flows_multiple_evaluations() {
        let (service, _) = create_service();

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let evaluations = vec![
            ArtifactFlowEvaluation {
                flow_id: ArtifactFlowId::new(),
                flow_name: "Flow A".to_string(),
                steps: vec![ArtifactFlowStep::copy(ArtifactBucketId::from_string("a"))],
            },
            ArtifactFlowEvaluation {
                flow_id: ArtifactFlowId::new(),
                flow_name: "Flow B".to_string(),
                steps: vec![ArtifactFlowStep::spawn_process("test", "agent")],
            },
        ];

        let results = service.execute_all_flows(&evaluations, &artifact);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].flow_name, "Flow A");
        assert_eq!(results[1].flow_name, "Flow B");
    }

    // ==================== Repository Method Tests ====================

    #[tokio::test]
    async fn get_flow_found() {
        let (service, flow_repo) = create_service();

        let flow = create_basic_flow();
        let id = flow.id.clone();
        flow_repo.add_flow(flow).await;

        let result = service.get_flow(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn get_flow_not_found() {
        let (service, _) = create_service();

        let id = ArtifactFlowId::new();
        let result = service.get_flow(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_all_flows_empty() {
        let (service, _) = create_service();

        let result = service.get_all_flows().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_all_flows_returns_all() {
        let (service, flow_repo) = create_service();

        flow_repo.add_flow(create_basic_flow()).await;
        flow_repo.add_flow(create_filtered_flow()).await;

        let result = service.get_all_flows().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_active_flows_filters_inactive() {
        let (service, flow_repo) = create_service();

        flow_repo.add_flow(create_basic_flow()).await;
        flow_repo
            .add_flow(create_filtered_flow().set_active(false))
            .await;

        let result = service.get_active_flows().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_flow_persists() {
        let (service, flow_repo) = create_service();

        let flow = create_basic_flow();
        let id = flow.id.clone();

        let result = service.create_flow(flow).await;
        assert!(result.is_ok());

        let found = flow_repo.get_by_id(&id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn update_flow_modifies() {
        let (service, flow_repo) = create_service();

        let mut flow = create_basic_flow();
        let id = flow.id.clone();
        flow_repo.add_flow(flow.clone()).await;

        flow.name = "Updated Name".to_string();
        service.update_flow(&flow).await.unwrap();

        let found = flow_repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Name");
    }

    #[tokio::test]
    async fn delete_flow_removes() {
        let (service, flow_repo) = create_service();

        let flow = create_basic_flow();
        let id = flow.id.clone();
        flow_repo.add_flow(flow).await;

        service.delete_flow(&id).await.unwrap();

        let found = flow_repo.get_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn set_flow_active_updates() {
        let (service, flow_repo) = create_service();

        let flow = create_basic_flow();
        let id = flow.id.clone();
        flow_repo.add_flow(flow).await;

        service.set_flow_active(&id, false).await.unwrap();

        let found = flow_repo.get_by_id(&id).await.unwrap().unwrap();
        assert!(!found.is_active);
    }

    #[tokio::test]
    async fn flow_exists_true() {
        let (service, flow_repo) = create_service();

        let flow = create_basic_flow();
        let id = flow.id.clone();
        flow_repo.add_flow(flow).await;

        let result = service.flow_exists(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn flow_exists_false() {
        let (service, _) = create_service();

        let id = ArtifactFlowId::new();
        let result = service.flow_exists(&id).await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==================== process_* Event Handler Tests ====================

    #[tokio::test]
    async fn process_artifact_created_loads_flows_and_executes() {
        let (mut service, flow_repo) = create_service();

        // Add a flow to the repo
        flow_repo.add_flow(create_basic_flow()).await;

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let results = service.process_artifact_created(&artifact).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].flow_name, "Basic Flow");
        assert_eq!(results[0].step_results.len(), 1);
    }

    #[tokio::test]
    async fn process_artifact_created_no_matching_flows() {
        let (mut service, flow_repo) = create_service();

        // Add a filtered flow that won't match
        flow_repo.add_flow(create_filtered_flow()).await;

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let results = service.process_artifact_created(&artifact).await.unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn process_task_completed_with_artifact() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_task_completed_flow()).await;

        let artifact = create_test_artifact(ArtifactType::CodeChange, None);
        let results = service
            .process_task_completed("task-1", Some(&artifact))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].flow_name, "Task Flow");
    }

    #[tokio::test]
    async fn process_task_completed_without_artifact() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_task_completed_flow()).await;

        let results = service.process_task_completed("task-1", None).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].step_results.is_empty()); // No artifact means no step execution
    }

    #[tokio::test]
    async fn process_process_completed_with_artifact() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_process_completed_flow()).await;

        let artifact = create_test_artifact(ArtifactType::Findings, None);
        let results = service
            .process_process_completed("process-1", Some(&artifact))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].flow_name, "Process Flow");
        assert_eq!(results[0].step_results.len(), 1);
    }

    #[tokio::test]
    async fn process_process_completed_without_artifact() {
        let (mut service, flow_repo) = create_service();

        flow_repo.add_flow(create_process_completed_flow()).await;

        let results = service
            .process_process_completed("process-1", None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].step_results.is_empty());
    }

    // ==================== Integration Scenario Tests ====================

    #[tokio::test]
    async fn research_to_dev_flow_scenario() {
        let (mut service, flow_repo) = create_service();

        // Register the research-to-dev flow
        flow_repo.add_flow(create_filtered_flow()).await;

        // Create a recommendations artifact in research-outputs bucket
        let artifact =
            create_test_artifact(ArtifactType::Recommendations, Some("research-outputs"));

        let results = service.process_artifact_created(&artifact).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].flow_name, "Filtered Flow");
        assert_eq!(results[0].step_results.len(), 2);

        // First step: copy to prd-library
        match &results[0].step_results[0] {
            StepExecutionResult::Copied { target_bucket, .. } => {
                assert_eq!(target_bucket.as_str(), "prd-library");
            }
            _ => panic!("Expected copy step"),
        }

        // Second step: spawn task_decomposition process
        match &results[0].step_results[1] {
            StepExecutionResult::ProcessSpawned {
                process_type,
                agent_profile,
            } => {
                assert_eq!(process_type, "task_decomposition");
                assert_eq!(agent_profile, "orchestrator");
            }
            _ => panic!("Expected spawn_process step"),
        }
    }

    #[tokio::test]
    async fn multiple_flows_triggered_scenario() {
        let (mut service, flow_repo) = create_service();

        // Add two flows that trigger on artifact_created
        flow_repo.add_flow(create_basic_flow()).await;

        let mut second_flow = create_basic_flow();
        second_flow.id = ArtifactFlowId::new();
        second_flow.name = "Second Flow".to_string();
        flow_repo.add_flow(second_flow).await;

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let results = service.process_artifact_created(&artifact).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn inactive_flows_not_triggered() {
        let (mut service, flow_repo) = create_service();

        // Add an inactive flow
        flow_repo
            .add_flow(create_basic_flow().set_active(false))
            .await;

        let artifact = create_test_artifact(ArtifactType::Prd, None);
        let results = service.process_artifact_created(&artifact).await.unwrap();

        assert!(results.is_empty());
    }
}
