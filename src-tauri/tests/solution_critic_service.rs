use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use futures::stream;
use ralphx_lib::application::solution_critic::{
    CompileContextRequest, CritiqueArtifactRequest, RawContextBundle, SolutionCritiqueGenerator,
    SolutionCritiqueService, SourceLimits,
};
use ralphx_lib::domain::agents::{
    AgentConfig, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ResponseChunk,
};
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactRelationType, ArtifactType, ChatMessage, ChatMessageId,
    CompiledContext, ContextSourceType, IdeationSession, IdeationSessionId, Project, ProjectId,
    ProposalCategory, TaskProposal, TaskProposalId, VerificationStatus,
};
use ralphx_lib::{AppResult, AppState};
use tokio::sync::Mutex;

struct StaticGenerator {
    compile_json: String,
    critique_json: String,
}

#[async_trait]
impl SolutionCritiqueGenerator for StaticGenerator {
    async fn compile_context_candidate(&self, _bundle: &RawContextBundle) -> AppResult<String> {
        Ok(self.compile_json.clone())
    }

    async fn critique_candidate(
        &self,
        _bundle: &RawContextBundle,
        _context: &CompiledContext,
    ) -> AppResult<String> {
        Ok(self.critique_json.clone())
    }
}

struct Fixture {
    state: AppState,
    session_id: IdeationSessionId,
    plan_artifact_id: ArtifactId,
}

struct RecordingAgentClient {
    responses: Mutex<Vec<String>>,
    prompts: Mutex<Vec<String>>,
    capabilities: ClientCapabilities,
}

impl RecordingAgentClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(responses),
            prompts: Mutex::new(Vec::new()),
            capabilities: ClientCapabilities::mock(),
        }
    }

    async fn prompts(&self) -> Vec<String> {
        self.prompts.lock().await.clone()
    }
}

#[async_trait]
impl AgenticClient for RecordingAgentClient {
    async fn spawn_agent(&self, _config: AgentConfig) -> AgentResult<AgentHandle> {
        unreachable!("solution critic default path should call send_prompt")
    }

    async fn stop_agent(&self, _handle: &AgentHandle) -> AgentResult<()> {
        Ok(())
    }

    async fn wait_for_completion(&self, _handle: &AgentHandle) -> AgentResult<AgentOutput> {
        Ok(AgentOutput::success(""))
    }

    async fn send_prompt(&self, _handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
        self.prompts.lock().await.push(prompt.to_string());
        let response = self.responses.lock().await.remove(0);
        Ok(AgentResponse::new(response))
    }

    fn stream_response(
        &self,
        _handle: &AgentHandle,
        _prompt: &str,
    ) -> Pin<Box<dyn futures::Stream<Item = AgentResult<ResponseChunk>> + Send>> {
        Box::pin(stream::empty())
    }

    fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> AgentResult<bool> {
        Ok(true)
    }
}

async fn setup_fixture() -> Fixture {
    let state = AppState::new_test();
    let project_id = ProjectId::from_string("project-solution-critic".to_string());
    let session_id = IdeationSessionId::from_string("session-solution-critic");
    let plan_artifact_id = ArtifactId::from_string("plan-artifact-1");

    let mut project = Project::new("Solution Critic".to_string(), "/tmp/ralphx".to_string());
    project.id = project_id.clone();
    project.detected_analysis = Some(r#"[{"path":"src-tauri","label":"Rust"}]"#.to_string());
    state.project_repo.create(project).await.unwrap();

    let mut plan = Artifact::new_inline(
        "Implementation Plan",
        ArtifactType::Specification,
        "Build the backend context compiler.",
        "orchestrator",
    );
    plan.id = plan_artifact_id.clone();
    state.artifact_repo.create(plan).await.unwrap();

    let session = IdeationSession::builder()
        .id(session_id.clone())
        .project_id(project_id)
        .plan_artifact_id(plan_artifact_id.clone())
        .verification_status(VerificationStatus::Unverified)
        .build();
    state.ideation_session_repo.create(session).await.unwrap();

    Fixture {
        state,
        session_id,
        plan_artifact_id,
    }
}

fn generator(compile_json: String, critique_json: String) -> Arc<dyn SolutionCritiqueGenerator> {
    Arc::new(StaticGenerator {
        compile_json,
        critique_json,
    })
}

fn compile_json(plan_id: &ArtifactId) -> String {
    format!(
        r#"{{
            "claims": [{{
                "id": "claim-plan",
                "text": "The plan exists.",
                "classification": "fact",
                "confidence": "high",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}]
            }}],
            "open_questions": [],
            "stale_assumptions": []
        }}"#
    )
}

fn critique_json(plan_id: &ArtifactId) -> String {
    format!(
        r#"{{
            "verdict": "investigate",
            "confidence": "medium",
            "claims": [{{
                "id": "claim-review",
                "claim": "The plan needs evidence review.",
                "status": "unclear",
                "confidence": "medium",
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}],
                "notes": "Needs manual inspection."
            }}],
            "recommendations": [],
            "risks": [],
            "verification_plan": [],
            "safe_next_action": "Inspect the compiled context."
        }}"#
    )
}

fn model_compile_response(plan_id: &ArtifactId) -> String {
    format!(
        r#"```json
{{
  "claims": [
    {{
      "id": "claim_backend_context_compiler",
      "text": "The plan promises a backend context compiler.",
      "classification": "fact",
      "confidence": "high",
      "evidence": [{{ "id": "plan_artifact:{plan_id}" }}]
    }}
  ],
  "open_questions": [
    {{
      "id": "question_targeted_test",
      "question": "Which targeted test proves the context compiler behavior?",
      "evidence": [{{ "id": "plan_artifact:{plan_id}" }}]
    }}
  ],
  "stale_assumptions": []
}}
```"#
    )
}

fn model_critique_response(plan_id: &ArtifactId) -> String {
    format!(
        r#"{{
  "verdict": "investigate",
  "confidence": "high",
  "claims": [
    {{
      "id": "claim_backend_context_compiler_accuracy",
      "claim": "The plan promises a backend context compiler.",
      "status": "unsupported",
      "confidence": "high",
      "evidence": [{{ "id": "plan_artifact:{plan_id}" }}],
      "notes": "The target states the promise, but collected sources do not prove the compiler is implemented."
    }}
  ],
  "recommendations": [],
  "risks": [
    {{
      "id": "risk_unproven_context_compiler",
      "risk": "Proceeding without proof could miss a broken context compiler path.",
      "severity": "high",
      "evidence": [{{ "id": "plan_artifact:{plan_id}" }}],
      "mitigation": "Run the focused solution critic service test before trusting the plan."
    }}
  ],
  "verification_plan": [
    {{
      "id": "verify_context_compiler",
      "requirement": "Prove the context compiler persists source-bound claims.",
      "priority": "high",
      "evidence": [{{ "id": "plan_artifact:{plan_id}" }}],
      "suggested_test": "cargo test --test solution_critic_service"
    }}
  ],
  "safe_next_action": "Run the targeted solution critic service test before trusting the plan."
}}"#
    )
}

#[tokio::test]
async fn collector_respects_limits_and_truncates_sources() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let now = Utc::now();

    for index in 0..3 {
        let mut message =
            ChatMessage::user_in_session(session_id.clone(), format!("message-{index}"));
        message.id = ChatMessageId::from_string(format!("message-{index}"));
        message.created_at = now + Duration::seconds(index);
        state.chat_message_repo.create(message).await.unwrap();
    }

    let mut proposal = TaskProposal::new(
        session_id.clone(),
        "Long proposal",
        ProposalCategory::Feature,
        ralphx_lib::domain::entities::Priority::Medium,
    );
    proposal.id = TaskProposalId::from_string("proposal-1");
    proposal.description = Some("x".repeat(4_100));
    state.task_proposal_repo.create(proposal).await.unwrap();

    let service = SolutionCritiqueService::from_app_state(&state);
    let bundle = service
        .collect_raw_context(
            session_id.as_str(),
            plan_artifact_id.as_str(),
            &SourceLimits {
                chat_messages: Some(2),
                task_proposals: Some(1),
                related_artifacts: Some(0),
                agent_runs: Some(0),
            },
        )
        .await
        .unwrap();

    let chat_sources: Vec<_> = bundle
        .sources
        .iter()
        .filter(|source| source.source_type == ContextSourceType::ChatMessage)
        .collect();
    assert_eq!(chat_sources.len(), 2);
    assert_eq!(chat_sources[0].id, "chat_message:message-1");
    assert_eq!(chat_sources[1].id, "chat_message:message-2");
    assert_eq!(
        bundle.sources[0].source_type,
        ContextSourceType::PlanArtifact
    );
    assert!(bundle
        .sources
        .iter()
        .find(|source| source.id == "task_proposal:proposal-1")
        .and_then(|source| source.excerpt.as_deref())
        .unwrap()
        .contains("[truncated]"));
}

#[tokio::test]
async fn compile_context_persists_context_artifact_and_relation() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(compile_json(&plan_artifact_id), "{}".to_string()),
    );

    let result = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap();

    let artifact = state
        .artifact_repo
        .get_by_id(&ArtifactId::from_string(&result.artifact_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(artifact.artifact_type, ArtifactType::Context);
    assert_eq!(artifact.bucket_id.unwrap().as_str(), "work-context");
    assert_eq!(artifact.metadata.created_by, "context_compiler");
    assert_eq!(result.compiled_context.id, result.artifact_id);

    let relations = state
        .artifact_repo
        .get_relations(&ArtifactId::from_string(&result.artifact_id))
        .await
        .unwrap();
    assert!(relations.iter().any(|relation| {
        relation.relation_type == ArtifactRelationType::RelatedTo
            && relation.to_artifact_id == plan_artifact_id
    }));

    let latest = service
        .get_latest_compiled_context(session_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.artifact_id, result.artifact_id);
    assert_eq!(latest.compiled_context.target.id, plan_artifact_id.as_str());
}

#[tokio::test]
async fn critique_artifact_persists_findings_artifact_and_relations() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_json(&plan_artifact_id),
            critique_json(&plan_artifact_id),
        ),
    );
    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap();

    let result = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                compiled_context_artifact_id: context.artifact_id.clone(),
            },
        )
        .await
        .unwrap();

    let artifact = state
        .artifact_repo
        .get_by_id(&ArtifactId::from_string(&result.artifact_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(artifact.artifact_type, ArtifactType::Findings);
    assert_eq!(artifact.bucket_id.unwrap().as_str(), "research-outputs");
    assert_eq!(artifact.metadata.created_by, "solution_critic");
    assert_eq!(artifact.derived_from[0].as_str(), context.artifact_id);
    assert_eq!(
        result.solution_critique.context_artifact_id,
        context.artifact_id
    );
    assert_eq!(result.projected_gaps.len(), 1);
    assert_eq!(result.projected_gaps[0].severity, "medium");
    assert_eq!(result.projected_gaps[0].category, "solution_critique_claim");

    let relations = state
        .artifact_repo
        .get_relations(&ArtifactId::from_string(&result.artifact_id))
        .await
        .unwrap();
    assert!(relations.iter().any(|relation| {
        relation.relation_type == ArtifactRelationType::DerivedFrom
            && relation.to_artifact_id.as_str() == context.artifact_id
    }));
    assert!(relations.iter().any(|relation| {
        relation.relation_type == ArtifactRelationType::RelatedTo
            && relation.to_artifact_id == plan_artifact_id
    }));

    let read = service
        .get_solution_critique(session_id.as_str(), &result.artifact_id)
        .await
        .unwrap();
    assert_eq!(read.projected_gaps.len(), 1);
    assert_eq!(read.projected_gaps[0].severity, "medium");

    let latest = service
        .get_latest_solution_critique(session_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.artifact_id, result.artifact_id);
    assert_eq!(latest.projected_gaps.len(), 1);
}

#[tokio::test]
async fn default_service_uses_agent_client_for_context_and_critique() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let agent_client = Arc::new(RecordingAgentClient::new(vec![
        model_compile_response(&plan_artifact_id),
        model_critique_response(&plan_artifact_id),
    ]));
    let state = state.with_agent_client(agent_client.clone());
    let service = SolutionCritiqueService::from_app_state(&state);

    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                compiled_context_artifact_id: context.artifact_id.clone(),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        context.compiled_context.claims[0].text,
        "The plan promises a backend context compiler."
    );
    assert_eq!(
        critique.solution_critique.claims[0].notes.as_deref(),
        Some("The target states the promise, but collected sources do not prove the compiler is implemented.")
    );
    assert_eq!(
        critique.solution_critique.safe_next_action.as_deref(),
        Some("Run the targeted solution critic service test before trusting the plan.")
    );
    assert!(critique
        .projected_gaps
        .iter()
        .any(|gap| gap.category == "solution_critique_risk" && gap.severity == "high"));

    let prompts = agent_client.prompts().await;
    assert_eq!(prompts.len(), 2);
    assert!(prompts[0].contains("solution context compiler"));
    assert!(prompts[1].contains("solution critic"));
    assert!(prompts[1].contains("Be strict"));
    assert!(!prompts[1].contains("Deterministic review requires"));
}

#[tokio::test]
async fn latest_reads_return_none_before_context_or_critique_exists() {
    let Fixture {
        state, session_id, ..
    } = setup_fixture().await;
    let service = SolutionCritiqueService::from_app_state(&state);

    assert!(service
        .get_latest_compiled_context(session_id.as_str())
        .await
        .unwrap()
        .is_none());
    assert!(service
        .get_latest_solution_critique(session_id.as_str())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn invalid_model_json_persists_no_partial_artifacts() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            r#"{"claims":[{"id":"bad","text":"Bad","classification":"fact","confidence":"certain","evidence":[]}]}"#
                .to_string(),
            "{}".to_string(),
        ),
    );

    let error = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("Invalid solution critique JSON"));
    let contexts = state
        .artifact_repo
        .get_by_type(ArtifactType::Context)
        .await
        .unwrap();
    assert!(contexts.is_empty());
}

#[tokio::test]
async fn read_methods_reject_artifacts_from_another_session_plan() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let other_plan_id = ArtifactId::from_string("plan-artifact-2");
    let mut other_plan = Artifact::new_inline(
        "Other Plan",
        ArtifactType::Specification,
        "Build a different backend change.",
        "orchestrator",
    );
    other_plan.id = other_plan_id.clone();
    state.artifact_repo.create(other_plan).await.unwrap();

    let other_session_id = IdeationSessionId::from_string("session-solution-critic-other");
    let other_session = IdeationSession::builder()
        .id(other_session_id.clone())
        .project_id(ProjectId::from_string(
            "project-solution-critic".to_string(),
        ))
        .plan_artifact_id(other_plan_id)
        .verification_status(VerificationStatus::Unverified)
        .build();
    state
        .ideation_session_repo
        .create(other_session)
        .await
        .unwrap();

    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_json(&plan_artifact_id),
            critique_json(&plan_artifact_id),
        ),
    );
    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                compiled_context_artifact_id: context.artifact_id.clone(),
            },
        )
        .await
        .unwrap();

    let context_error = service
        .get_compiled_context(other_session_id.as_str(), &context.artifact_id)
        .await
        .unwrap_err();
    assert!(context_error
        .to_string()
        .contains("targets the session plan artifact only"));

    let critique_error = service
        .get_solution_critique(other_session_id.as_str(), &critique.artifact_id)
        .await
        .unwrap_err();
    assert!(critique_error
        .to_string()
        .contains("targets the session plan artifact only"));
}
