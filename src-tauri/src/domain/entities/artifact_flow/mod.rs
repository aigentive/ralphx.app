// Artifact flow entities for the extensibility system
// Artifact flows automate artifact routing between processes

#[cfg(test)]
mod tests;

mod types;

pub use types::*;

use super::artifact::{Artifact, ArtifactBucketId, ArtifactType};

/// Engine for evaluating and executing artifact flows
#[derive(Debug, Default)]
pub struct ArtifactFlowEngine {
    /// Registered flows
    flows: Vec<ArtifactFlow>,
}

impl ArtifactFlowEngine {
    /// Creates a new empty engine
    pub fn new() -> Self {
        Self { flows: vec![] }
    }

    /// Registers a flow with the engine
    pub fn register_flow(&mut self, flow: ArtifactFlow) {
        self.flows.push(flow);
    }

    /// Registers multiple flows with the engine
    pub fn register_flows(&mut self, flows: impl IntoIterator<Item = ArtifactFlow>) {
        self.flows.extend(flows);
    }

    /// Removes a flow by ID
    pub fn unregister_flow(&mut self, flow_id: &ArtifactFlowId) -> Option<ArtifactFlow> {
        if let Some(pos) = self.flows.iter().position(|f| &f.id == flow_id) {
            Some(self.flows.remove(pos))
        } else {
            None
        }
    }

    /// Returns all registered flows
    pub fn flows(&self) -> &[ArtifactFlow] {
        &self.flows
    }

    /// Returns the number of registered flows
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Evaluates triggers for a given context and returns matching flows
    pub fn evaluate_triggers(&self, context: &ArtifactFlowContext) -> Vec<ArtifactFlowEvaluation> {
        let mut evaluations = vec![];

        for flow in &self.flows {
            if !flow.is_active {
                continue;
            }

            if flow.trigger.event != context.event {
                continue;
            }

            // Check filter if artifact is present
            let matches = match (&context.artifact, &flow.trigger.filter) {
                (Some(artifact), Some(filter)) => filter.matches(artifact),
                (Some(_artifact), None) => true,
                (None, Some(_filter)) => false, // Can't match filter without artifact
                (None, None) => true,           // No filter, no artifact, match
            };

            if matches {
                evaluations.push(ArtifactFlowEvaluation {
                    flow_id: flow.id.clone(),
                    flow_name: flow.name.clone(),
                    steps: flow.steps.clone(),
                });
            }
        }

        evaluations
    }

    /// Convenience method to evaluate triggers for artifact_created event
    pub fn on_artifact_created(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::artifact_created(artifact.clone());
        self.evaluate_triggers(&context)
    }

    /// Convenience method to evaluate triggers for artifact_updated event
    pub fn on_artifact_updated(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::artifact_updated(artifact.clone());
        self.evaluate_triggers(&context)
    }

    /// Convenience method to evaluate triggers for task_completed event
    pub fn on_task_completed(
        &self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::task_completed(task_id, artifact.cloned());
        self.evaluate_triggers(&context)
    }

    /// Convenience method to evaluate triggers for process_completed event
    pub fn on_process_completed(
        &self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::process_completed(process_id, artifact.cloned());
        self.evaluate_triggers(&context)
    }
}

/// Creates the example "research-to-dev" flow from the PRD
pub fn create_research_to_dev_flow() -> ArtifactFlow {
    ArtifactFlow::new(
        "Research to Development",
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

/// Creates the "plan_updated_sync" flow for proactive sync of ideation plans.
///
/// This flow triggers when a Specification artifact (implementation plan) in the
/// prd-library bucket is updated. It:
/// 1. Finds all proposals linked to the plan artifact
/// 2. Emits a `plan:proposals_may_need_update` event to notify the UI
///
/// The UI can then show a notification like:
/// "Plan updated. N proposals may need revision. [Review]"
pub fn create_plan_updated_sync_flow() -> ArtifactFlow {
    ArtifactFlow::new(
        "Plan Updated Sync",
        ArtifactFlowTrigger::on_artifact_updated().with_filter(
            ArtifactFlowFilter::new()
                .with_artifact_types(vec![ArtifactType::Specification])
                .with_source_bucket(ArtifactBucketId::from_string("prd-library")),
        ),
    )
    .with_step(ArtifactFlowStep::find_linked_proposals())
    .with_step(ArtifactFlowStep::emit_event("plan:proposals_may_need_update"))
}
