use std::collections::HashSet;
use std::sync::Arc;

use crate::application::AppState;
use crate::domain::entities::{
    AgentRunId, Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactRelation,
    ArtifactType, ChatContextType, ChatConversationId, ChatMessage, ChatMessageId, CompiledContext,
    ContextSourceRef, ContextSourceType, ContextTargetRef, ContextTargetType, IdeationSession,
    Project, Review, ReviewId, SolutionCritique, Task, TaskId,
};
use crate::domain::repositories::{
    AgentRunRepository, ArtifactRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, ReviewRepository, TaskProposalRepository,
    TaskRepository,
};
use crate::domain::services::project_solution_critique_gaps;
use crate::error::{AppError, AppResult};

use super::generator::{AgentSolutionCritiqueGenerator, SolutionCritiqueGenerator};
use super::support::{
    agent_run_source, artifact_source, build_compiled_context, build_solution_critique,
    chat_message_source, ensure_plan_target, ensure_task_in_session_project,
    inline_artifact_content, parse_candidate, parse_inline_artifact, project_analysis_sources,
    review_issue_source, review_note_source, review_source, severity_rank, sort_sources,
    task_proposal_source, task_source, to_pretty_json, truncate_text, verification_status_source,
    SOURCE_EXCERPT_LIMIT,
};
use super::types::{
    CompileContextRequest, CompileContextResult, CompiledContextCandidate,
    CompiledContextReadResult, ContextTargetRequest, CritiqueArtifactRequest,
    CritiqueArtifactResult, EffectiveSourceLimits, RawContextBundle, SolutionCritiqueCandidate,
    SolutionCritiqueReadResult, SourceLimits,
};
use crate::infrastructure::sqlite::ReviewIssueRepository;

const CONTEXT_COMPILER_CREATED_BY: &str = "context_compiler";
const SOLUTION_CRITIC_CREATED_BY: &str = "solution_critic";
const WORK_CONTEXT_BUCKET: &str = "work-context";
const RESEARCH_OUTPUTS_BUCKET: &str = "research-outputs";

struct ResolvedCritiqueTarget {
    target: ContextTargetRef,
    target_content: String,
    artifact_id: Option<ArtifactId>,
    task_id: Option<TaskId>,
    review_id: Option<ReviewId>,
    agent_run_conversation_id: Option<ChatConversationId>,
}

pub struct SolutionCritiqueService {
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    task_proposal_repo: Arc<dyn TaskProposalRepository>,
    chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    task_repo: Arc<dyn TaskRepository>,
    review_repo: Arc<dyn ReviewRepository>,
    review_issue_repo: Arc<dyn ReviewIssueRepository>,
    generator: Arc<dyn SolutionCritiqueGenerator>,
}

impl SolutionCritiqueService {
    pub fn from_app_state(app_state: &AppState) -> Self {
        Self::from_app_state_with_generator(
            app_state,
            Arc::new(AgentSolutionCritiqueGenerator::new(Arc::clone(
                &app_state.agent_clients.default_client,
            ))),
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
            task_repo: Arc::clone(&app_state.task_repo),
            review_repo: Arc::clone(&app_state.review_repo),
            review_issue_repo: Arc::clone(&app_state.review_issue_repo),
            generator,
        }
    }

    pub async fn collect_raw_context(
        &self,
        session_id: &str,
        target_request: ContextTargetRequest,
        source_limits: &SourceLimits,
    ) -> AppResult<RawContextBundle> {
        let session = self.load_session(session_id).await?;
        let project = self.load_project(&session).await?;
        let target = self.resolve_target(&session, target_request).await?;
        let limits = source_limits.effective();
        let mut sources = Vec::new();

        sources.extend(self.target_sources(&target).await?);
        sources.extend(self.collect_chat_sources(&session, limits).await?);
        sources.extend(self.collect_proposal_sources(&session, limits).await?);
        sources.extend(self.collect_verification_sources(&session).await?);
        sources.extend(project_analysis_sources(&project));
        if let Some(artifact_id) = target.artifact_id.as_ref() {
            sources.extend(
                self.collect_related_artifact_sources(artifact_id, limits)
                    .await?,
            );
        }
        if let Some(task_id) = target.task_id.as_ref() {
            sources.extend(self.collect_task_context_sources(task_id, limits).await?);
        }
        if let Some(review_id) = target.review_id.as_ref() {
            sources.extend(self.collect_review_context_sources(review_id).await?);
        }
        sources.extend(
            self.collect_agent_run_sources(
                &session,
                target.agent_run_conversation_id.as_ref(),
                limits,
            )
            .await?,
        );
        dedupe_sort_sources(&mut sources);

        Ok(RawContextBundle {
            session_id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            target: target.target,
            target_content: target.target_content,
            sources,
        })
    }

    pub async fn compile_context(
        &self,
        session_id: &str,
        request: CompileContextRequest,
    ) -> AppResult<CompileContextResult> {
        let target_request = request
            .target_request()
            .ok_or_else(|| AppError::Validation("A critique target is required".to_string()))?;
        let bundle = self
            .collect_raw_context(session_id, target_request, &request.source_limits)
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
        if matches!(
            compiled_context.target.target_type,
            ContextTargetType::PlanArtifact | ContextTargetType::Artifact
        ) {
            let target_id = ArtifactId::from_string(&compiled_context.target.id);
            self.artifact_repo
                .add_relation(ArtifactRelation::related_to(artifact.id.clone(), target_id))
                .await?;
        }

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
        self.validate_context_target_scope(&session, &compiled_context.target)
            .await?;
        Ok(CompiledContextReadResult {
            artifact_id: artifact_id.to_string(),
            compiled_context,
        })
    }

    pub async fn get_latest_compiled_context(
        &self,
        session_id: &str,
    ) -> AppResult<Option<CompiledContextReadResult>> {
        let session = self.load_session(session_id).await?;
        let Some(target_artifact_id) = self.current_plan_artifact_id(&session).await? else {
            return Ok(None);
        };
        self.get_latest_compiled_context_for_target(
            session_id,
            ContextTargetRequest {
                target_type: ContextTargetType::PlanArtifact,
                id: target_artifact_id.as_str().to_string(),
                label: None,
            },
        )
        .await
    }

    pub async fn get_latest_compiled_context_for_target(
        &self,
        session_id: &str,
        target_request: ContextTargetRequest,
    ) -> AppResult<Option<CompiledContextReadResult>> {
        let session = self.load_session(session_id).await?;
        let target = self.resolve_target(&session, target_request).await?.target;
        let artifacts = self
            .artifact_repo
            .get_by_type(ArtifactType::Context)
            .await?;
        let mut matches = Vec::new();
        for artifact in artifacts {
            if artifact.archived_at.is_some() {
                continue;
            }
            let Ok(compiled_context) = parse_inline_artifact::<CompiledContext>(&artifact) else {
                continue;
            };
            if same_target(&compiled_context.target, &target) {
                matches.push((
                    artifact.metadata.created_at,
                    artifact.id.as_str().to_string(),
                    compiled_context,
                ));
            }
        }

        matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

        Ok(matches
            .into_iter()
            .next()
            .map(
                |(_, artifact_id, compiled_context)| CompiledContextReadResult {
                    artifact_id,
                    compiled_context,
                },
            ))
    }

    pub async fn critique_artifact(
        &self,
        session_id: &str,
        request: CritiqueArtifactRequest,
    ) -> AppResult<CritiqueArtifactResult> {
        let session = self.load_session(session_id).await?;
        let target_request = request
            .target_request()
            .ok_or_else(|| AppError::Validation("A critique target is required".to_string()))?;
        let target = self.resolve_target(&session, target_request).await?;
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
        if compiled_context.target.target_type != target.target.target_type
            || compiled_context.target.id != target.target.id
        {
            return Err(AppError::Validation(
                "Compiled context target does not match critique target".to_string(),
            ));
        }

        let bundle = RawContextBundle {
            session_id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            target: compiled_context.target.clone(),
            target_content: target.target_content,
            sources: compiled_context.sources.clone(),
        };
        let candidate_json = self
            .generator
            .critique_candidate(&bundle, &compiled_context)
            .await?;
        let candidate: SolutionCritiqueCandidate = parse_candidate(&candidate_json)?;
        let mut critique = build_solution_critique(
            candidate,
            target.target.id.as_str(),
            &request.compiled_context_artifact_id,
            &compiled_context,
        )?;

        let context_id = ArtifactId::from_string(request.compiled_context_artifact_id);
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
        if let Some(target_id) = target.artifact_id {
            self.artifact_repo
                .add_relation(ArtifactRelation::related_to(artifact.id.clone(), target_id))
                .await?;
        }

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
        self.validate_critique_scope(&session, &solution_critique)
            .await?;
        Ok(SolutionCritiqueReadResult {
            artifact_id: artifact_id.to_string(),
            projected_gaps: project_solution_critique_gaps(&solution_critique),
            solution_critique,
        })
    }

    pub async fn get_latest_solution_critique(
        &self,
        session_id: &str,
    ) -> AppResult<Option<SolutionCritiqueReadResult>> {
        let session = self.load_session(session_id).await?;
        let Some(target_artifact_id) = self.current_plan_artifact_id(&session).await? else {
            return Ok(None);
        };
        self.get_latest_solution_critique_for_target(
            session_id,
            ContextTargetRequest {
                target_type: ContextTargetType::PlanArtifact,
                id: target_artifact_id.as_str().to_string(),
                label: None,
            },
        )
        .await
    }

    pub async fn get_latest_solution_critique_for_target(
        &self,
        session_id: &str,
        target_request: ContextTargetRequest,
    ) -> AppResult<Option<SolutionCritiqueReadResult>> {
        let session = self.load_session(session_id).await?;
        let target = self.resolve_target(&session, target_request).await?.target;
        let artifacts = self
            .artifact_repo
            .get_by_type(ArtifactType::Findings)
            .await?;
        let mut matches = Vec::new();
        for artifact in artifacts {
            if artifact.archived_at.is_some() {
                continue;
            }
            let Ok(solution_critique) = parse_inline_artifact::<SolutionCritique>(&artifact) else {
                continue;
            };
            let Ok(context_artifact) = self
                .load_artifact(&solution_critique.context_artifact_id)
                .await
            else {
                continue;
            };
            let Ok(context) = parse_inline_artifact::<CompiledContext>(&context_artifact) else {
                continue;
            };
            if same_target(&context.target, &target) && solution_critique.artifact_id == target.id {
                matches.push((
                    artifact.metadata.created_at,
                    artifact.id.as_str().to_string(),
                    solution_critique,
                ));
            }
        }

        matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

        Ok(matches
            .into_iter()
            .next()
            .map(
                |(_, artifact_id, solution_critique)| SolutionCritiqueReadResult {
                    artifact_id,
                    projected_gaps: project_solution_critique_gaps(&solution_critique),
                    solution_critique,
                },
            ))
    }

    async fn resolve_target(
        &self,
        session: &IdeationSession,
        request: ContextTargetRequest,
    ) -> AppResult<ResolvedCritiqueTarget> {
        match request.target_type {
            ContextTargetType::PlanArtifact => {
                let artifact_id = self.resolve_latest_artifact_id(&request.id).await?;
                ensure_plan_target(session, artifact_id.as_str())?;
                let artifact = self.load_artifact(artifact_id.as_str()).await?;
                let target_content = inline_artifact_content(&artifact)?;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type: ContextTargetType::PlanArtifact,
                        id: artifact.id.as_str().to_string(),
                        label: request.label.unwrap_or_else(|| artifact.name.clone()),
                    },
                    target_content,
                    artifact_id: Some(artifact.id),
                    task_id: None,
                    review_id: None,
                    agent_run_conversation_id: None,
                })
            }
            ContextTargetType::Artifact => {
                let artifact_id = self.resolve_latest_artifact_id(&request.id).await?;
                let artifact = self.load_artifact(artifact_id.as_str()).await?;
                self.validate_artifact_scope(session, &artifact).await?;
                let target_content = inline_artifact_content(&artifact)?;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type: ContextTargetType::Artifact,
                        id: artifact.id.as_str().to_string(),
                        label: request.label.unwrap_or_else(|| artifact.name.clone()),
                    },
                    target_content,
                    artifact_id: Some(artifact.id),
                    task_id: artifact.metadata.task_id.clone(),
                    review_id: None,
                    agent_run_conversation_id: None,
                })
            }
            ContextTargetType::ChatMessage => {
                let message = self.load_chat_message(session, &request.id).await?;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type: ContextTargetType::ChatMessage,
                        id: message.id.as_str().to_string(),
                        label: request
                            .label
                            .unwrap_or_else(|| format!("{} message", message.role)),
                    },
                    target_content: message.content,
                    artifact_id: None,
                    task_id: message.task_id,
                    review_id: None,
                    agent_run_conversation_id: message.conversation_id,
                })
            }
            ContextTargetType::AgentRun => {
                let run_id = request.id.parse::<AgentRunId>().map_err(|error| {
                    AppError::Validation(format!("Invalid agent run id: {error}"))
                })?;
                let run = self
                    .agent_run_repo
                    .get_by_id(&run_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Agent run {} not found", request.id))
                    })?;
                self.validate_conversation_scope(session, &run.conversation_id)
                    .await?;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type: ContextTargetType::AgentRun,
                        id: run.id.to_string(),
                        label: request
                            .label
                            .unwrap_or_else(|| format!("Agent run {}", run.status)),
                    },
                    target_content: agent_run_source(&run).excerpt.unwrap_or_default(),
                    artifact_id: None,
                    task_id: None,
                    review_id: None,
                    agent_run_conversation_id: Some(run.conversation_id),
                })
            }
            ContextTargetType::Task | ContextTargetType::TaskExecution => {
                let task = self.load_task(session, &request.id).await?;
                let target_type = request.target_type;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type,
                        id: task.id.as_str().to_string(),
                        label: request.label.unwrap_or_else(|| task.title.clone()),
                    },
                    target_content: task_source(&task).excerpt.unwrap_or_default(),
                    artifact_id: task.plan_artifact_id.clone(),
                    task_id: Some(task.id),
                    review_id: None,
                    agent_run_conversation_id: None,
                })
            }
            ContextTargetType::ReviewReport => {
                let review = self.load_review(session, &request.id).await?;
                let task = self.load_task(session, review.task_id.as_str()).await?;
                Ok(ResolvedCritiqueTarget {
                    target: ContextTargetRef {
                        target_type: ContextTargetType::ReviewReport,
                        id: review.id.as_str().to_string(),
                        label: request
                            .label
                            .unwrap_or_else(|| format!("Review for {}", task.title)),
                    },
                    target_content: review_source(&review).excerpt.unwrap_or_default(),
                    artifact_id: task.plan_artifact_id,
                    task_id: Some(task.id),
                    review_id: Some(review.id),
                    agent_run_conversation_id: None,
                })
            }
        }
    }

    async fn target_sources(
        &self,
        target: &ResolvedCritiqueTarget,
    ) -> AppResult<Vec<ContextSourceRef>> {
        match target.target.target_type {
            ContextTargetType::PlanArtifact => {
                let Some(artifact_id) = target.artifact_id.as_ref() else {
                    return Ok(vec![]);
                };
                let artifact = self.load_artifact(artifact_id.as_str()).await?;
                Ok(vec![artifact_source(
                    ContextSourceType::PlanArtifact,
                    "plan_artifact",
                    &artifact,
                    Some(&target.target_content),
                )])
            }
            ContextTargetType::Artifact => {
                let Some(artifact_id) = target.artifact_id.as_ref() else {
                    return Ok(vec![]);
                };
                let artifact = self.load_artifact(artifact_id.as_str()).await?;
                Ok(vec![artifact_source(
                    ContextSourceType::Artifact,
                    "artifact",
                    &artifact,
                    Some(&target.target_content),
                )])
            }
            ContextTargetType::ChatMessage => Ok(vec![ContextSourceRef {
                source_type: ContextSourceType::ChatMessage,
                id: format!("chat_message:{}", target.target.id),
                label: target.target.label.clone(),
                excerpt: Some(truncate_text(&target.target_content, SOURCE_EXCERPT_LIMIT)),
                created_at: None,
            }]),
            ContextTargetType::AgentRun => Ok(vec![ContextSourceRef {
                source_type: ContextSourceType::AgentRun,
                id: format!("agent_run:{}", target.target.id),
                label: target.target.label.clone(),
                excerpt: Some(truncate_text(&target.target_content, SOURCE_EXCERPT_LIMIT)),
                created_at: None,
            }]),
            ContextTargetType::Task | ContextTargetType::TaskExecution => {
                let Some(task_id) = target.task_id.as_ref() else {
                    return Ok(vec![]);
                };
                let task = self
                    .task_repo
                    .get_by_id(task_id)
                    .await?
                    .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;
                Ok(vec![task_source(&task)])
            }
            ContextTargetType::ReviewReport => {
                let Some(review_id) = target.review_id.as_ref() else {
                    return Ok(vec![]);
                };
                let review = self
                    .review_repo
                    .get_by_id(review_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound(format!("Review {} not found", review_id)))?;
                Ok(vec![review_source(&review)])
            }
        }
    }

    async fn validate_critique_scope(
        &self,
        session: &IdeationSession,
        critique: &SolutionCritique,
    ) -> AppResult<()> {
        let context_artifact = self.load_artifact(&critique.context_artifact_id).await?;
        let context: CompiledContext = parse_inline_artifact(&context_artifact)?;
        if context.target.id != critique.artifact_id {
            return Err(AppError::Validation(
                "Solution critique target does not match its compiled context".to_string(),
            ));
        }
        self.validate_context_target_scope(session, &context.target)
            .await
    }

    async fn validate_context_target_scope(
        &self,
        session: &IdeationSession,
        target: &ContextTargetRef,
    ) -> AppResult<()> {
        match target.target_type {
            ContextTargetType::PlanArtifact => ensure_plan_target(session, &target.id),
            ContextTargetType::Artifact => {
                let artifact = self.load_artifact(&target.id).await?;
                self.validate_artifact_scope(session, &artifact).await
            }
            ContextTargetType::ChatMessage => self
                .load_chat_message(session, &target.id)
                .await
                .map(|_| ()),
            ContextTargetType::AgentRun => {
                let run_id = target.id.parse::<AgentRunId>().map_err(|error| {
                    AppError::Validation(format!("Invalid agent run id: {error}"))
                })?;
                let run = self
                    .agent_run_repo
                    .get_by_id(&run_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Agent run {} not found", target.id))
                    })?;
                self.validate_conversation_scope(session, &run.conversation_id)
                    .await
            }
            ContextTargetType::Task | ContextTargetType::TaskExecution => {
                self.load_task(session, &target.id).await.map(|_| ())
            }
            ContextTargetType::ReviewReport => {
                self.load_review(session, &target.id).await.map(|_| ())
            }
        }
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

    async fn load_chat_message(
        &self,
        session: &IdeationSession,
        message_id: &str,
    ) -> AppResult<ChatMessage> {
        let id = ChatMessageId::from_string(message_id);
        let message = self
            .chat_message_repo
            .get_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Chat message {message_id} not found")))?;
        if message
            .session_id
            .as_ref()
            .is_some_and(|id| id.as_str() == session.id.as_str())
            || message
                .project_id
                .as_ref()
                .is_some_and(|id| id.as_str() == session.project_id.as_str())
        {
            return Ok(message);
        }
        if let Some(task_id) = message.task_id.as_ref() {
            let task = self.load_task(session, task_id.as_str()).await?;
            ensure_task_in_session_project(session, &task)?;
            return Ok(message);
        }
        if let Some(conversation_id) = message.conversation_id.as_ref() {
            self.validate_conversation_scope(session, conversation_id)
                .await?;
            return Ok(message);
        }
        Err(AppError::Validation(
            "Chat message is outside the ideation session scope".to_string(),
        ))
    }

    async fn load_task(&self, session: &IdeationSession, task_id: &str) -> AppResult<Task> {
        let id = TaskId::from_string(task_id.to_string());
        let task = self
            .task_repo
            .get_by_id(&id)
            .await?
            .ok_or_else(|| AppError::TaskNotFound(task_id.to_string()))?;
        ensure_task_in_session_project(session, &task)?;
        Ok(task)
    }

    async fn load_review(&self, session: &IdeationSession, review_id: &str) -> AppResult<Review> {
        let id = ReviewId::from_string(review_id);
        let review = self
            .review_repo
            .get_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Review {review_id} not found")))?;
        let task = self.load_task(session, review.task_id.as_str()).await?;
        ensure_task_in_session_project(session, &task)?;
        Ok(review)
    }

    async fn validate_artifact_scope(
        &self,
        session: &IdeationSession,
        artifact: &Artifact,
    ) -> AppResult<()> {
        if artifact
            .metadata
            .team_metadata
            .as_ref()
            .and_then(|metadata| metadata.session_id.as_deref())
            .is_some_and(|session_id| session_id == session.id.as_str())
        {
            return Ok(());
        }
        if let Some(task_id) = artifact.metadata.task_id.as_ref() {
            let task = self
                .task_repo
                .get_by_id(task_id)
                .await?
                .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;
            ensure_task_in_session_project(session, &task)?;
            return Ok(());
        }
        if ensure_plan_target(session, artifact.id.as_str()).is_ok() {
            return Ok(());
        }
        if self
            .artifact_repo
            .get_related(&artifact.id)
            .await?
            .iter()
            .any(|related| ensure_plan_target(session, related.id.as_str()).is_ok())
        {
            return Ok(());
        }
        Err(AppError::Validation(
            "Artifact is outside the ideation session scope".to_string(),
        ))
    }

    async fn validate_conversation_scope(
        &self,
        session: &IdeationSession,
        conversation_id: &ChatConversationId,
    ) -> AppResult<()> {
        let conversation = self
            .chat_conversation_repo
            .get_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;
        match conversation.context_type {
            ChatContextType::Ideation | ChatContextType::Delegation => {
                if conversation.context_id == session.id.as_str() {
                    Ok(())
                } else {
                    Err(AppError::Validation(
                        "Conversation is outside the ideation session scope".to_string(),
                    ))
                }
            }
            ChatContextType::Project => {
                if conversation.context_id == session.project_id.as_str() {
                    Ok(())
                } else {
                    Err(AppError::Validation(
                        "Conversation is outside the ideation session project".to_string(),
                    ))
                }
            }
            ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
                self.load_task(session, &conversation.context_id)
                    .await
                    .map(|_| ())
            }
            ChatContextType::Merge => self
                .load_task(session, &conversation.context_id)
                .await
                .map(|_| ()),
        }
    }

    async fn resolve_latest_artifact_id(&self, artifact_id: &str) -> AppResult<ArtifactId> {
        self.artifact_repo
            .resolve_latest_artifact_id(&ArtifactId::from_string(artifact_id))
            .await
    }

    async fn current_plan_artifact_id(
        &self,
        session: &IdeationSession,
    ) -> AppResult<Option<ArtifactId>> {
        match &session.plan_artifact_id {
            Some(artifact_id) => Ok(Some(
                self.resolve_latest_artifact_id(artifact_id.as_str())
                    .await?,
            )),
            None => Ok(None),
        }
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

    async fn collect_task_context_sources(
        &self,
        task_id: &TaskId,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        let task = self
            .task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;
        let mut sources = vec![task_source(&task)];
        if let Some(plan_artifact_id) = task.plan_artifact_id.as_ref() {
            let plan_artifact_id = self
                .resolve_latest_artifact_id(plan_artifact_id.as_str())
                .await?;
            let plan_artifact = self.load_artifact(plan_artifact_id.as_str()).await?;
            let content = match &plan_artifact.content {
                ArtifactContent::Inline { text } => Some(text.as_str()),
                ArtifactContent::File { .. } => None,
            };
            sources.push(artifact_source(
                ContextSourceType::PlanArtifact,
                "plan_artifact",
                &plan_artifact,
                content,
            ));
        }
        sources.extend(self.collect_task_artifact_sources(task_id, limits).await?);
        sources.extend(self.collect_task_chat_sources(task_id, limits).await?);
        sources.extend(self.collect_review_note_sources(task_id).await?);
        sources.extend(self.collect_open_review_issue_sources(task_id).await?);
        sources.extend(
            self.collect_agent_run_sources_for_contexts(
                &[
                    (ChatContextType::Task, task_id.as_str().to_string()),
                    (ChatContextType::TaskExecution, task_id.as_str().to_string()),
                    (ChatContextType::Review, task_id.as_str().to_string()),
                    (ChatContextType::Merge, task_id.as_str().to_string()),
                ],
                None,
                limits,
            )
            .await?,
        );
        Ok(sources)
    }

    async fn collect_review_context_sources(
        &self,
        review_id: &ReviewId,
    ) -> AppResult<Vec<ContextSourceRef>> {
        let review = self
            .review_repo
            .get_by_id(review_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Review {} not found", review_id)))?;
        Ok(vec![review_source(&review)])
    }

    async fn collect_task_artifact_sources(
        &self,
        task_id: &TaskId,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.related_artifacts == 0 {
            return Ok(vec![]);
        }
        let mut artifacts = self.artifact_repo.get_by_task(task_id).await?;
        artifacts.retain(|artifact| artifact.archived_at.is_none());
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

    async fn collect_task_chat_sources(
        &self,
        task_id: &TaskId,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.chat_messages == 0 {
            return Ok(vec![]);
        }

        let mut messages = self.chat_message_repo.get_by_task(task_id).await?;
        messages.extend(
            self.collect_messages_from_contexts(&[
                (ChatContextType::Task, task_id.as_str().to_string()),
                (ChatContextType::TaskExecution, task_id.as_str().to_string()),
                (ChatContextType::Review, task_id.as_str().to_string()),
                (ChatContextType::Merge, task_id.as_str().to_string()),
            ])
            .await?,
        );
        dedupe_sort_messages(&mut messages);
        messages.truncate(limits.chat_messages as usize);
        Ok(messages.iter().map(chat_message_source).collect())
    }

    async fn collect_review_note_sources(
        &self,
        task_id: &TaskId,
    ) -> AppResult<Vec<ContextSourceRef>> {
        let mut notes = self.review_repo.get_notes_by_task_id(task_id).await?;
        notes.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(notes.iter().map(review_note_source).collect())
    }

    async fn collect_open_review_issue_sources(
        &self,
        task_id: &TaskId,
    ) -> AppResult<Vec<ContextSourceRef>> {
        let mut issues = self.review_issue_repo.get_open_by_task_id(task_id).await?;
        issues.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(issues.iter().map(review_issue_source).collect())
    }

    async fn collect_messages_from_contexts(
        &self,
        contexts: &[(ChatContextType, String)],
    ) -> AppResult<Vec<ChatMessage>> {
        let mut messages = Vec::new();
        for (context_type, context_id) in contexts {
            let conversations = self
                .chat_conversation_repo
                .get_by_context(*context_type, context_id)
                .await?;
            for conversation in conversations {
                messages.extend(
                    self.chat_message_repo
                        .get_by_conversation(&conversation.id)
                        .await?,
                );
            }
        }
        Ok(messages)
    }

    async fn collect_agent_run_sources(
        &self,
        session: &IdeationSession,
        target_conversation_id: Option<&ChatConversationId>,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        self.collect_agent_run_sources_for_contexts(
            &[(ChatContextType::Ideation, session.id.as_str().to_string())],
            target_conversation_id,
            limits,
        )
        .await
    }

    async fn collect_agent_run_sources_for_contexts(
        &self,
        contexts: &[(ChatContextType, String)],
        target_conversation_id: Option<&ChatConversationId>,
        limits: EffectiveSourceLimits,
    ) -> AppResult<Vec<ContextSourceRef>> {
        if limits.agent_runs == 0 {
            return Ok(vec![]);
        }
        let mut conversations = Vec::new();
        for (context_type, context_id) in contexts {
            conversations.extend(
                self.chat_conversation_repo
                    .get_by_context(*context_type, context_id)
                    .await?,
            );
        }
        if let Some(conversation_id) = target_conversation_id {
            if let Some(conversation) = self
                .chat_conversation_repo
                .get_by_id(conversation_id)
                .await?
            {
                conversations.push(conversation);
            }
        }
        conversations.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        conversations.dedup_by(|left, right| left.id == right.id);
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

fn dedupe_sort_messages(messages: &mut Vec<ChatMessage>) {
    let mut seen = HashSet::new();
    messages.retain(|message| seen.insert(message.id.as_str().to_string()));
    messages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
}

fn dedupe_sort_sources(sources: &mut Vec<ContextSourceRef>) {
    sort_sources(sources);
    let mut seen = HashSet::new();
    sources.retain(|source| seen.insert(source.id.clone()));
}

fn same_target(left: &ContextTargetRef, right: &ContextTargetRef) -> bool {
    left.target_type == right.target_type && left.id == right.id
}
