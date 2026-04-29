use std::sync::Arc;

use crate::application::AppState;
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactRelation, ArtifactType,
    ChatContextType, CompiledContext, ContextSourceRef, ContextSourceType, ContextTargetRef,
    ContextTargetType, IdeationSession, Project, SolutionCritique,
};
use crate::domain::repositories::{
    AgentRunRepository, ArtifactRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskProposalRepository,
};
use crate::domain::services::project_solution_critique_gaps;
use crate::error::{AppError, AppResult};

use super::generator::{DeterministicSolutionCritiqueGenerator, SolutionCritiqueGenerator};
use super::support::{
    agent_run_source, artifact_source, build_compiled_context, build_solution_critique,
    chat_message_source, ensure_plan_target, inline_artifact_content, parse_candidate,
    parse_inline_artifact, project_analysis_sources, severity_rank, sort_sources,
    task_proposal_source, to_pretty_json, truncate_text, verification_status_source,
    SOURCE_EXCERPT_LIMIT,
};
use super::types::{
    CompileContextRequest, CompileContextResult, CompiledContextCandidate,
    CompiledContextReadResult, CritiqueArtifactRequest, CritiqueArtifactResult,
    EffectiveSourceLimits, RawContextBundle, SolutionCritiqueCandidate, SolutionCritiqueReadResult,
    SourceLimits,
};

const CONTEXT_COMPILER_CREATED_BY: &str = "context_compiler";
const SOLUTION_CRITIC_CREATED_BY: &str = "solution_critic";
const WORK_CONTEXT_BUCKET: &str = "work-context";
const RESEARCH_OUTPUTS_BUCKET: &str = "research-outputs";

pub struct SolutionCritiqueService {
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    task_proposal_repo: Arc<dyn TaskProposalRepository>,
    chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    generator: Arc<dyn SolutionCritiqueGenerator>,
}

impl SolutionCritiqueService {
    pub fn from_app_state(app_state: &AppState) -> Self {
        Self::from_app_state_with_generator(
            app_state,
            Arc::new(DeterministicSolutionCritiqueGenerator),
        )
    }

    pub fn from_app_state_with_generator(
        app_state: &AppState,
        generator: Arc<dyn SolutionCritiqueGenerator>,
    ) -> Self {
        Self {
            ideation_session_repo: Arc::clone(&app_state.ideation_session_repo),
            project_repo: Arc::clone(&app_state.project_repo),
            artifact_repo: Arc::clone(&app_state.artifact_repo),
            chat_message_repo: Arc::clone(&app_state.chat_message_repo),
            task_proposal_repo: Arc::clone(&app_state.task_proposal_repo),
            chat_conversation_repo: Arc::clone(&app_state.chat_conversation_repo),
            agent_run_repo: Arc::clone(&app_state.agent_run_repo),
            generator,
        }
    }

    pub async fn collect_raw_context(
        &self,
        session_id: &str,
        target_artifact_id: &str,
        source_limits: &SourceLimits,
    ) -> AppResult<RawContextBundle> {
        let session = self.load_session(session_id).await?;
        let target_artifact_id = self.resolve_latest_artifact_id(target_artifact_id).await?;
        ensure_plan_target(&session, target_artifact_id.as_str())?;
        let project = self.load_project(&session).await?;
        let target_artifact = self.load_artifact(target_artifact_id.as_str()).await?;
        let target_content = inline_artifact_content(&target_artifact)?;
        let limits = source_limits.effective();
        let mut sources = Vec::new();

        sources.push(artifact_source(
            ContextSourceType::PlanArtifact,
            "plan_artifact",
            &target_artifact,
            Some(&target_content),
        ));
        sources.extend(self.collect_chat_sources(&session, limits).await?);
        sources.extend(self.collect_proposal_sources(&session, limits).await?);
        sources.extend(self.collect_verification_sources(&session).await?);
        sources.extend(project_analysis_sources(&project));
        sources.extend(
            self.collect_related_artifact_sources(&target_artifact.id, limits)
                .await?,
        );
        sources.extend(self.collect_agent_run_sources(&session, limits).await?);
        sort_sources(&mut sources);

        Ok(RawContextBundle {
            session_id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            target: ContextTargetRef {
                target_type: ContextTargetType::PlanArtifact,
                id: target_artifact.id.as_str().to_string(),
                label: target_artifact.name,
            },
            target_content,
            sources,
        })
    }

    pub async fn compile_context(
        &self,
        session_id: &str,
        request: CompileContextRequest,
    ) -> AppResult<CompileContextResult> {
        let bundle = self
            .collect_raw_context(
                session_id,
                &request.target_artifact_id,
                &request.source_limits,
            )
            .await?;
        let candidate_json = self.generator.compile_context_candidate(&bundle).await?;
        let candidate: CompiledContextCandidate = parse_candidate(&candidate_json)?;
        let mut compiled_context = build_compiled_context(candidate, &bundle)?;

        let mut artifact = Artifact::new_inline(
            "Compiled Context",
            ArtifactType::Context,
            "",
            CONTEXT_COMPILER_CREATED_BY,
        )
        .with_bucket(ArtifactBucketId::from_string(WORK_CONTEXT_BUCKET));
        compiled_context.id = artifact.id.as_str().to_string();
        artifact.content = ArtifactContent::inline(to_pretty_json(&compiled_context)?);
        let artifact = self.artifact_repo.create(artifact).await?;
        let target_id = ArtifactId::from_string(&compiled_context.target.id);
        self.artifact_repo
            .add_relation(ArtifactRelation::related_to(artifact.id.clone(), target_id))
            .await?;

        Ok(CompileContextResult {
            artifact_id: artifact.id.as_str().to_string(),
            compiled_context,
        })
    }

    pub async fn get_compiled_context(
        &self,
        session_id: &str,
        artifact_id: &str,
    ) -> AppResult<CompiledContextReadResult> {
        let session = self.load_session(session_id).await?;
        let artifact = self.load_artifact(artifact_id).await?;
        if artifact.artifact_type != ArtifactType::Context {
            return Err(AppError::Validation(format!(
                "Artifact {artifact_id} is not a compiled context"
            )));
        }
        let compiled_context: CompiledContext = parse_inline_artifact(&artifact)?;
        ensure_plan_target(&session, &compiled_context.target.id)?;
        Ok(CompiledContextReadResult {
            artifact_id: artifact_id.to_string(),
            compiled_context,
        })
    }

    pub async fn critique_artifact(
        &self,
        session_id: &str,
        request: CritiqueArtifactRequest,
    ) -> AppResult<CritiqueArtifactResult> {
        let session = self.load_session(session_id).await?;
        let target_artifact_id = self
            .resolve_latest_artifact_id(&request.target_artifact_id)
            .await?;
        ensure_plan_target(&session, target_artifact_id.as_str())?;
        let target_artifact = self.load_artifact(target_artifact_id.as_str()).await?;
        let target_content = inline_artifact_content(&target_artifact)?;
        let context_artifact = self
            .load_artifact(&request.compiled_context_artifact_id)
            .await?;
        if context_artifact.artifact_type != ArtifactType::Context {
            return Err(AppError::Validation(format!(
                "Artifact {} is not a compiled context",
                request.compiled_context_artifact_id
            )));
        }
        let compiled_context: CompiledContext = parse_inline_artifact(&context_artifact)?;
        if compiled_context.target.id != target_artifact_id.as_str() {
            return Err(AppError::Validation(
                "Compiled context target does not match critique target artifact".to_string(),
            ));
        }

        let bundle = RawContextBundle {
            session_id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            target: compiled_context.target.clone(),
            target_content,
            sources: compiled_context.sources.clone(),
        };
        let candidate_json = self
            .generator
            .critique_candidate(&bundle, &compiled_context)
            .await?;
        let candidate: SolutionCritiqueCandidate = parse_candidate(&candidate_json)?;
        let mut critique = build_solution_critique(
            candidate,
            target_artifact_id.as_str(),
            &request.compiled_context_artifact_id,
            &compiled_context,
        )?;

        let context_id = ArtifactId::from_string(request.compiled_context_artifact_id);
        let target_id = target_artifact_id;
        let mut artifact = Artifact::new_inline(
            "Solution Critique",
            ArtifactType::Findings,
            "",
            SOLUTION_CRITIC_CREATED_BY,
        )
        .with_bucket(ArtifactBucketId::from_string(RESEARCH_OUTPUTS_BUCKET))
        .derived_from_artifact(context_id.clone());
        critique.id = artifact.id.as_str().to_string();
        artifact.content = ArtifactContent::inline(to_pretty_json(&critique)?);
        let artifact = self.artifact_repo.create(artifact).await?;
        self.artifact_repo
            .add_relation(ArtifactRelation::derived_from(
                artifact.id.clone(),
                context_id,
            ))
            .await?;
        self.artifact_repo
            .add_relation(ArtifactRelation::related_to(artifact.id.clone(), target_id))
            .await?;

        Ok(CritiqueArtifactResult {
            artifact_id: artifact.id.as_str().to_string(),
            projected_gaps: project_solution_critique_gaps(&critique),
            solution_critique: critique,
        })
    }

    pub async fn get_solution_critique(
        &self,
        session_id: &str,
        artifact_id: &str,
    ) -> AppResult<SolutionCritiqueReadResult> {
        let session = self.load_session(session_id).await?;
        let artifact = self.load_artifact(artifact_id).await?;
        if artifact.artifact_type != ArtifactType::Findings {
            return Err(AppError::Validation(format!(
                "Artifact {artifact_id} is not a solution critique"
            )));
        }
        let solution_critique: SolutionCritique = parse_inline_artifact(&artifact)?;
        ensure_plan_target(&session, &solution_critique.artifact_id)?;
        Ok(SolutionCritiqueReadResult {
            artifact_id: artifact_id.to_string(),
            projected_gaps: project_solution_critique_gaps(&solution_critique),
            solution_critique,
        })
    }

    async fn load_session(&self, session_id: &str) -> AppResult<IdeationSession> {
        let id = crate::domain::entities::IdeationSessionId::from_string(session_id);
        self.ideation_session_repo
            .get_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Session {session_id} not found")))
    }

    async fn load_project(&self, session: &IdeationSession) -> AppResult<Project> {
        self.project_repo
            .get_by_id(&session.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Project {} not found", session.project_id)))
    }

    async fn load_artifact(&self, artifact_id: &str) -> AppResult<Artifact> {
        let id = ArtifactId::from_string(artifact_id);
        self.artifact_repo
            .get_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Artifact {artifact_id} not found")))
    }

    async fn resolve_latest_artifact_id(&self, artifact_id: &str) -> AppResult<ArtifactId> {
        self.artifact_repo
            .resolve_latest_artifact_id(&ArtifactId::from_string(artifact_id))
            .await
    }

    async fn collect_chat_sources(
        &self,
        session: &IdeationSession,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.chat_messages == 0 {
            return Ok(vec![]);
        }
        let mut messages = self
            .chat_message_repo
            .get_recent_by_session(&session.id, limits.chat_messages)
            .await?;
        messages.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(messages.iter().map(chat_message_source).collect())
    }

    async fn collect_proposal_sources(
        &self,
        session: &IdeationSession,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.task_proposals == 0 {
            return Ok(vec![]);
        }
        let mut proposals = self.task_proposal_repo.get_by_session(&session.id).await?;
        proposals.retain(|proposal| proposal.archived_at.is_none());
        proposals.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        proposals.truncate(limits.task_proposals as usize);
        Ok(proposals.iter().map(task_proposal_source).collect())
    }

    async fn collect_verification_sources(
        &self,
        session: &IdeationSession,
    ) -> AppResult<Vec<ContextSourceRef>> {
        let mut sources = vec![verification_status_source(session)];
        if let Some(snapshot) = self
            .ideation_session_repo
            .get_verification_run_snapshot(&session.id, session.verification_generation)
            .await?
        {
            let mut gaps = snapshot.current_gaps;
            gaps.sort_by(|left, right| {
                severity_rank(&left.severity)
                    .cmp(&severity_rank(&right.severity))
                    .then_with(|| left.category.cmp(&right.category))
                    .then_with(|| left.description.cmp(&right.description))
            });
            for (index, gap) in gaps.iter().enumerate() {
                sources.push(ContextSourceRef {
                    source_type: ContextSourceType::VerificationGap,
                    id: format!(
                        "verification_gap:{}:{}:{}",
                        session.id.as_str(),
                        session.verification_generation,
                        index + 1
                    ),
                    label: format!("{} verification gap", gap.severity),
                    excerpt: Some(truncate_text(
                        &format!(
                            "{} / {}: {}{}",
                            gap.severity,
                            gap.category,
                            gap.description,
                            gap.why_it_matters
                                .as_ref()
                                .map(|value| format!("\nWhy it matters: {value}"))
                                .unwrap_or_default()
                        ),
                        SOURCE_EXCERPT_LIMIT,
                    )),
                    created_at: None,
                });
            }
        }
        Ok(sources)
    }

    async fn collect_related_artifact_sources(
        &self,
        target_id: &ArtifactId,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.related_artifacts == 0 {
            return Ok(vec![]);
        }
        let mut artifacts = self.artifact_repo.get_related(target_id).await?;
        artifacts.retain(|artifact| artifact.id != *target_id && artifact.archived_at.is_none());
        artifacts.sort_by(|left, right| {
            left.metadata
                .created_at
                .cmp(&right.metadata.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        artifacts.truncate(limits.related_artifacts as usize);
        Ok(artifacts
            .iter()
            .map(|artifact| {
                let content = match &artifact.content {
                    ArtifactContent::Inline { text } => Some(text.as_str()),
                    ArtifactContent::File { .. } => None,
                };
                artifact_source(ContextSourceType::Artifact, "artifact", artifact, content)
            })
            .collect())
    }

    async fn collect_agent_run_sources(
        &self,
        session: &IdeationSession,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.agent_runs == 0 {
            return Ok(vec![]);
        }
        let mut conversations = self
            .chat_conversation_repo
            .get_by_context(ChatContextType::Ideation, session.id.as_str())
            .await?;
        conversations.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        let mut runs = Vec::new();
        for conversation in conversations {
            runs.extend(
                self.agent_run_repo
                    .get_by_conversation(&conversation.id)
                    .await?,
            );
        }
        runs.sort_by(|left, right| {
            right
                .started_at
                .cmp(&left.started_at)
                .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
        });
        runs.truncate(limits.agent_runs as usize);
        Ok(runs.iter().map(agent_run_source).collect())
    }
}
