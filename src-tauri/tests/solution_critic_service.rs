use std::{collections::HashSet, pin::Pin, sync::Arc};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use futures::stream;
use ralphx_lib::application::solution_critic::{
    CompileContextRequest, ContextTargetRequest, CritiqueArtifactRequest, RawContextBundle,
    SolutionCritiqueGenerator, SolutionCritiqueService, SourceLimits,
};
use ralphx_lib::domain::agents::{
    AgentConfig, AgentHandle, AgentOutput, AgentResponse, AgentResult, AgenticClient,
    ClientCapabilities, ResponseChunk,
};
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactRelationType, ArtifactType, ChatMessage, ChatMessageId,
    CompiledContext, ContextSourceType, ContextTargetType, IdeationSession, IdeationSessionId,
    IssueSeverity, Project, ProjectId, ProposalCategory, Review, ReviewIssueEntity, ReviewNote,
    ReviewOutcome, ReviewerType, Task, TaskId, TaskProposal, TaskProposalId, TeamArtifactMetadata,
    VerificationStatus,
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

fn compile_for_source_json(source_id: &str, claim_text: &str) -> String {
    format!(
        r#"{{
            "claims": [{{
                "id": "claim-target",
                "text": "{claim_text}",
                "classification": "fact",
                "confidence": "high",
                "evidence": [{{"id": "{source_id}"}}]
            }}],
            "open_questions": [],
            "stale_assumptions": []
        }}"#
    )
}

fn critique_for_source_json(source_id: &str, claim_text: &str) -> String {
    format!(
        r#"{{
            "verdict": "investigate",
            "confidence": "medium",
            "claims": [{{
                "id": "claim-target-review",
                "claim": "{claim_text}",
                "status": "unclear",
                "confidence": "medium",
                "evidence": [{{"id": "{source_id}"}}],
                "notes": "The target needs evidence review."
            }}],
            "recommendations": [],
            "risks": [],
            "verification_plan": [],
            "safe_next_action": "Inspect the target context."
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

fn claude_stream_response(text: String) -> String {
    [
        serde_json::json!({
            "type": "system",
            "subtype": "hook_started",
            "hook_id": "hook-1",
            "hook_name": "SessionStart:startup",
        })
        .to_string(),
        serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "text",
                        "text": text,
                    }
                ],
                "stop_reason": "end_turn",
            },
            "session_id": "claude-session-1",
        })
        .to_string(),
        serde_json::json!({
            "type": "result",
            "result": "done",
            "session_id": "claude-session-1",
            "is_error": false,
        })
        .to_string(),
    ]
    .join("\n")
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
            ContextTargetRequest {
                target_type: ContextTargetType::PlanArtifact,
                id: plan_artifact_id.as_str().to_string(),
                label: None,
            },
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
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
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
async fn compile_context_supports_chat_message_targets_without_artifact_relation() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id: _,
    } = setup_fixture().await;
    let message_id = ChatMessageId::from_string("assistant-message-1");
    let mut message =
        ChatMessage::orchestrator_in_session(session_id.clone(), "The implementation is complete.");
    message.id = message_id.clone();
    state.chat_message_repo.create(message).await.unwrap();

    let source_id = format!("chat_message:{}", message_id.as_str());
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_for_source_json(
                &source_id,
                "The assistant message makes an implementation claim.",
            ),
            "{}".to_string(),
        ),
    );

    let result = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest::for_target(ContextTargetType::ChatMessage, message_id.as_str()),
        )
        .await
        .unwrap();

    assert_eq!(
        result.compiled_context.target.target_type,
        ContextTargetType::ChatMessage
    );
    assert_eq!(result.compiled_context.target.id, message_id.as_str());
    assert!(result
        .compiled_context
        .sources
        .iter()
        .any(|source| source.id == source_id));

    let relations = state
        .artifact_repo
        .get_relations(&ArtifactId::from_string(&result.artifact_id))
        .await
        .unwrap();
    assert!(!relations
        .iter()
        .any(|relation| relation.relation_type == ArtifactRelationType::RelatedTo));

    let read = service
        .get_compiled_context(session_id.as_str(), &result.artifact_id)
        .await
        .unwrap();
    assert_eq!(read.compiled_context.target.id, message_id.as_str());

    let latest = service
        .get_latest_compiled_context_for_target(
            session_id.as_str(),
            ContextTargetRequest {
                target_type: ContextTargetType::ChatMessage,
                id: message_id.as_str().to_string(),
                label: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.artifact_id, result.artifact_id);
    assert_eq!(
        latest.compiled_context.target.target_type,
        ContextTargetType::ChatMessage
    );
}

#[tokio::test]
async fn compile_context_accepts_team_artifacts_scoped_to_session() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id: _,
    } = setup_fixture().await;
    let artifact_id = ArtifactId::from_string("team-artifact-1");
    let mut artifact = Artifact::new_inline(
        "Backend research",
        ArtifactType::TeamResearch,
        "The specialist found the backend path is implemented.",
        "backend-specialist",
    );
    artifact.id = artifact_id.clone();
    artifact.metadata = artifact.metadata.with_team_metadata(TeamArtifactMetadata {
        team_name: "research-team".to_string(),
        author_teammate: "backend-specialist".to_string(),
        session_id: Some(session_id.as_str().to_string()),
        team_phase: Some("research".to_string()),
        verification_finding: None,
    });
    state.artifact_repo.create(artifact).await.unwrap();

    let source_id = format!("artifact:{}", artifact_id.as_str());
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_for_source_json(&source_id, "The team artifact is valid critique context."),
            "{}".to_string(),
        ),
    );

    let result = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest::for_target(ContextTargetType::Artifact, artifact_id.as_str()),
        )
        .await
        .unwrap();

    assert_eq!(
        result.compiled_context.target.target_type,
        ContextTargetType::Artifact
    );
    assert_eq!(result.compiled_context.target.id, artifact_id.as_str());
    assert!(result
        .compiled_context
        .sources
        .iter()
        .any(|source| source.id == source_id));

    let read = service
        .get_compiled_context(session_id.as_str(), &result.artifact_id)
        .await
        .unwrap();
    assert_eq!(read.compiled_context.target.id, artifact_id.as_str());
}

#[tokio::test]
async fn task_execution_targets_collect_diff_transcript_and_review_context() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .unwrap()
        .unwrap();
    let task_id = TaskId::from_string("task-execution-1".to_string());
    let mut task = Task::new(
        session.project_id.clone(),
        "Implement solution critic UI".to_string(),
    );
    task.id = task_id.clone();
    task.description =
        Some("Wire critique targets across execution and review surfaces.".to_string());
    task.ideation_session_id = Some(session_id.clone());
    task.plan_artifact_id = Some(plan_artifact_id);
    state.task_repo.create(task).await.unwrap();

    let diff_id = ArtifactId::from_string("task-diff-1");
    let mut diff = Artifact::new_inline(
        "Worker Diff",
        ArtifactType::Diff,
        "diff --git a/frontend/src/components/Ideation/SolutionCritiqueSummary.tsx",
        "worker",
    )
    .with_task(task_id.clone());
    diff.id = diff_id.clone();
    state.artifact_repo.create(diff).await.unwrap();

    let mut transcript =
        ChatMessage::user_about_task(task_id.clone(), "Worker claims all critique UI is done.");
    transcript.id = ChatMessageId::from_string("task-message-1");
    state.chat_message_repo.create(transcript).await.unwrap();

    let mut review = Review::new(session.project_id, task_id.clone(), ReviewerType::Ai);
    review.request_changes("Missing tests for arbitrary critique targets.".to_string());
    state.review_repo.create(&review).await.unwrap();

    let note = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Ai,
        ReviewOutcome::ChangesRequested,
        "The worker claim is unsupported without frontend tests.".to_string(),
    );
    let note_id = note.id.clone();
    state.review_repo.add_note(&note).await.unwrap();
    let mut issue = ReviewIssueEntity::new(
        note_id.clone(),
        task_id.clone(),
        "Open reviewer issue".to_string(),
        IssueSeverity::Major,
    );
    issue.description = Some("The latest worker claim still needs proof.".to_string());
    let issue_id = issue.id.clone();
    state.review_issue_repo.create(issue).await.unwrap();

    let task_source_id = format!("task:{}", task_id.as_str());
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_for_source_json(&task_source_id, "The task execution target is reviewable."),
            critique_for_source_json(&task_source_id, "The task execution target is reviewable."),
        ),
    );

    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest::for_target(ContextTargetType::TaskExecution, task_id.as_str()),
        )
        .await
        .unwrap();

    assert_eq!(
        context.compiled_context.target.target_type,
        ContextTargetType::TaskExecution
    );
    let source_ids = context
        .compiled_context
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect::<Vec<_>>();
    let unique_source_ids = source_ids.iter().collect::<HashSet<_>>();
    assert_eq!(unique_source_ids.len(), source_ids.len());
    let diff_source_id = format!("artifact:{}", diff_id.as_str());
    let note_source_id = format!("review_note:{}", note_id.as_str());
    let issue_source_id = format!("review_issue:{}", issue_id.as_str());
    assert!(source_ids.contains(&task_source_id.as_str()));
    assert!(source_ids.contains(&diff_source_id.as_str()));
    assert!(source_ids.contains(&note_source_id.as_str()));
    assert!(source_ids.contains(&issue_source_id.as_str()));
    assert!(source_ids.contains(&"chat_message:task-message-1"));

    let context_artifact_id = context.artifact_id.clone();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_target(
                ContextTargetType::TaskExecution,
                task_id.as_str(),
                context_artifact_id.clone(),
            ),
        )
        .await
        .unwrap();

    assert_eq!(critique.solution_critique.artifact_id, task_id.as_str());
    assert_eq!(
        critique.solution_critique.context_artifact_id,
        context_artifact_id
    );
    let read = service
        .get_solution_critique(session_id.as_str(), &critique.artifact_id)
        .await
        .unwrap();
    assert_eq!(read.solution_critique.artifact_id, task_id.as_str());

    let latest = service
        .get_latest_solution_critique_for_target(
            session_id.as_str(),
            ContextTargetRequest {
                target_type: ContextTargetType::TaskExecution,
                id: task_id.as_str().to_string(),
                label: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.artifact_id, critique.artifact_id);
    assert_eq!(latest.solution_critique.artifact_id, task_id.as_str());
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
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
        )
        .await
        .unwrap();

    let result = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_plan_artifact(
                plan_artifact_id.as_str(),
                context.artifact_id.clone(),
            ),
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
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_plan_artifact(
                plan_artifact_id.as_str(),
                context.artifact_id.clone(),
            ),
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
async fn default_service_extracts_critique_from_claude_stream_json() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let agent_client = Arc::new(RecordingAgentClient::new(vec![
        claude_stream_response(model_compile_response(&plan_artifact_id)),
        claude_stream_response(model_critique_response(&plan_artifact_id)),
    ]));
    let state = state.with_agent_client(agent_client.clone());
    let service = SolutionCritiqueService::from_app_state(&state);

    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_plan_artifact(
                plan_artifact_id.as_str(),
                context.artifact_id.clone(),
            ),
        )
        .await
        .unwrap();

    assert_eq!(context.compiled_context.claims.len(), 1);
    assert_eq!(
        critique.solution_critique.safe_next_action.as_deref(),
        Some("Run the targeted solution critic service test before trusting the plan.")
    );
    assert_eq!(
        state
            .artifact_repo
            .get_by_type(ArtifactType::Findings)
            .await
            .unwrap()
            .len(),
        1
    );

    let prompts = agent_client.prompts().await;
    assert_eq!(prompts.len(), 2);
}

#[tokio::test]
async fn default_service_repairs_critique_schema_before_persisting() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let agent_client = Arc::new(RecordingAgentClient::new(vec![
        model_compile_response(&plan_artifact_id),
        model_compile_response(&plan_artifact_id),
        model_critique_response(&plan_artifact_id),
    ]));
    let state = state.with_agent_client(agent_client.clone());
    let service = SolutionCritiqueService::from_app_state(&state);

    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_plan_artifact(
                plan_artifact_id.as_str(),
                context.artifact_id.clone(),
            ),
        )
        .await
        .unwrap();

    assert_eq!(
        critique.solution_critique.safe_next_action.as_deref(),
        Some("Run the targeted solution critic service test before trusting the plan.")
    );
    assert_eq!(
        state
            .artifact_repo
            .get_by_type(ArtifactType::Findings)
            .await
            .unwrap()
            .len(),
        1
    );

    let prompts = agent_client.prompts().await;
    assert_eq!(prompts.len(), 3);
    assert!(prompts[1].contains("solution critic"));
    assert!(prompts[2].contains("did not match the solution critique schema"));
    assert!(prompts[2].contains("Do not return the context compiler schema"));
    assert!(prompts[2].contains("missing field"));
    assert!(prompts[2].contains("verdict"));
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
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
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
            CompileContextRequest::for_plan_artifact(plan_artifact_id.as_str()),
        )
        .await
        .unwrap();
    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest::for_plan_artifact(
                plan_artifact_id.as_str(),
                context.artifact_id.clone(),
            ),
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
