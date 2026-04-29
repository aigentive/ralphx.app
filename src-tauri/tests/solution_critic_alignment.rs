use std::sync::Arc;

use async_trait::async_trait;
use ralphx_lib::application::solution_critic::{
    CompileContextRequest, CritiqueArtifactRequest, RawContextBundle, SolutionCritiqueGenerator,
    SolutionCritiqueService, SourceLimits,
};
use ralphx_lib::domain::entities::{
    Artifact, ArtifactId, ArtifactRelationType, ArtifactType, CompiledContext, IdeationSession,
    IdeationSessionId, Project, ProjectId, VerificationStatus,
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
                "evidence": [{{"id": "plan_artifact:{plan_id}"}}]
            }}],
            "recommendations": [],
            "risks": [],
            "verification_plan": [],
            "safe_next_action": "Inspect the compiled context."
        }}"#
    )
}

async fn setup_fixture() -> Fixture {
    let state = AppState::new_test();
    let project_id = ProjectId::from_string("project-solution-critic-alignment".to_string());
    let session_id = IdeationSessionId::from_string("session-solution-critic-alignment");
    let plan_artifact_id = ArtifactId::from_string("plan-alignment");

    let mut project = Project::new(
        "Solution Critic Alignment".to_string(),
        "/tmp/ralphx".to_string(),
    );
    project.id = project_id.clone();
    state.project_repo.create(project).await.unwrap();

    let mut plan = Artifact::new_inline(
        "Implementation Plan",
        ArtifactType::Specification,
        "Build the backend context compiler.",
        "orchestrator",
    );
    plan.id = plan_artifact_id.clone();
    state.artifact_repo.create(plan).await.unwrap();

    let mut session = IdeationSession::builder()
        .id(session_id.clone())
        .project_id(project_id)
        .plan_artifact_id(plan_artifact_id.clone())
        .verification_status(VerificationStatus::Reviewing)
        .verification_generation(3)
        .build();
    session.verification_in_progress = true;
    session.verification_current_round = Some(2);
    session.verification_max_rounds = Some(4);
    session.verification_gap_count = 5;
    session.verification_gap_score = Some(80);
    session.verification_convergence_reason = Some("initial-review".to_string());
    state.ideation_session_repo.create(session).await.unwrap();

    Fixture {
        state,
        session_id,
        plan_artifact_id,
    }
}

#[tokio::test]
async fn compile_context_resolves_stale_target_artifact_to_latest_version() {
    let state = AppState::new_sqlite_test();
    let project_id = ProjectId::from_string("project-solution-critic-latest".to_string());
    let session_id = IdeationSessionId::from_string("session-solution-critic-latest");
    let original_plan_id = ArtifactId::from_string("plan-latest-v1");
    let latest_plan_id = ArtifactId::from_string("plan-latest-v2");

    let mut project = Project::new("Latest Plan".to_string(), "/tmp/ralphx".to_string());
    project.id = project_id.clone();
    state.project_repo.create(project).await.unwrap();

    let mut original_plan = Artifact::new_inline(
        "Implementation Plan",
        ArtifactType::Specification,
        "Old plan content.",
        "orchestrator",
    );
    original_plan.id = original_plan_id.clone();
    state.artifact_repo.create(original_plan).await.unwrap();

    let mut latest_plan = Artifact::new_inline(
        "Implementation Plan",
        ArtifactType::Specification,
        "Latest plan content.",
        "orchestrator",
    );
    latest_plan.id = latest_plan_id.clone();
    latest_plan.metadata.version = 2;
    state
        .artifact_repo
        .create_with_previous_version(latest_plan, original_plan_id.clone())
        .await
        .unwrap();

    let session = IdeationSession::builder()
        .id(session_id.clone())
        .project_id(project_id)
        .plan_artifact_id(latest_plan_id.clone())
        .verification_status(VerificationStatus::Unverified)
        .build();
    state.ideation_session_repo.create(session).await.unwrap();

    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            compile_json(&latest_plan_id),
            critique_json(&latest_plan_id),
        ),
    );
    let context = service
        .compile_context(
            session_id.as_str(),
            CompileContextRequest {
                target_artifact_id: original_plan_id.as_str().to_string(),
                source_limits: SourceLimits::default(),
            },
        )
        .await
        .unwrap();
    assert_eq!(context.compiled_context.target.id, latest_plan_id.as_str());

    let context_relations = state
        .artifact_repo
        .get_relations(&ArtifactId::from_string(&context.artifact_id))
        .await
        .unwrap();
    assert!(context_relations.iter().any(|relation| {
        relation.relation_type == ArtifactRelationType::RelatedTo
            && relation.to_artifact_id == latest_plan_id
    }));

    let critique = service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest {
                target_artifact_id: original_plan_id.as_str().to_string(),
                compiled_context_artifact_id: context.artifact_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        critique.solution_critique.artifact_id,
        latest_plan_id.as_str()
    );
}

#[tokio::test]
async fn unknown_model_evidence_source_is_rejected_before_persistence() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let unknown_source_json = r#"{
        "claims": [{
            "id": "claim-unknown-source",
            "text": "The plan has unsupported evidence.",
            "classification": "fact",
            "confidence": "high",
            "evidence": [{"id": "chat_message:not-collected"}]
        }],
        "open_questions": [],
        "stale_assumptions": []
    }"#;
    let service = SolutionCritiqueService::from_app_state_with_generator(
        &state,
        generator(
            unknown_source_json.to_string(),
            critique_json(&plan_artifact_id),
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

    assert!(error.to_string().contains("unknown source id"));
    let contexts = state
        .artifact_repo
        .get_by_type(ArtifactType::Context)
        .await
        .unwrap();
    assert!(contexts.is_empty());
}

#[tokio::test]
async fn compile_and_critique_do_not_mutate_session_verification_state() {
    let Fixture {
        state,
        session_id,
        plan_artifact_id,
    } = setup_fixture().await;
    let before = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .unwrap()
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
    service
        .critique_artifact(
            session_id.as_str(),
            CritiqueArtifactRequest {
                target_artifact_id: plan_artifact_id.as_str().to_string(),
                compiled_context_artifact_id: context.artifact_id,
            },
        )
        .await
        .unwrap();

    let after = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.plan_artifact_id, before.plan_artifact_id);
    assert_eq!(after.verification_status, before.verification_status);
    assert_eq!(
        after.verification_in_progress,
        before.verification_in_progress
    );
    assert_eq!(
        after.verification_generation,
        before.verification_generation
    );
    assert_eq!(
        after.verification_current_round,
        before.verification_current_round
    );
    assert_eq!(
        after.verification_max_rounds,
        before.verification_max_rounds
    );
    assert_eq!(after.verification_gap_count, before.verification_gap_count);
    assert_eq!(after.verification_gap_score, before.verification_gap_score);
    assert_eq!(
        after.verification_convergence_reason,
        before.verification_convergence_reason
    );
}
