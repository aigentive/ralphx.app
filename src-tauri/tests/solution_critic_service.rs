use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ralphx_lib::application::solution_critic::{
    CompileContextRequest, CritiqueArtifactRequest, RawContextBundle, SolutionCritiqueGenerator,
    SolutionCritiqueService, SourceLimits,
};
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactRelationType, ArtifactType, ChatMessage, ChatMessageId,
    CompiledContext, ContextSourceType, IdeationSession, IdeationSessionId, Project, ProjectId,
    ProposalCategory, TaskProposal, TaskProposalId, VerificationStatus,
};
use ralphx_lib::{AppResult, AppState};

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

#[tokio::test]
async fn collector_respects_limits_and_truncates_sources() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let now = Utc::now();

    for index in 0..3 {
        let mut message = ChatMessage::user_in_session(
            session_id.clone(),
            format!("message-{index}"),
        );
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
    assert_eq!(bundle.sources[0].source_type, ContextSourceType::PlanArtifact);
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
    assert_eq!(result.solution_critique.context_artifact_id, context.artifact_id);

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
