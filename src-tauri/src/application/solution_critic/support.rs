use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

use crate::domain::entities::{
    AgentRun, Artifact, ArtifactContent, ChatMessage, ClaimReview, CompiledContext,
    ContextAssumption, ContextClaim, ContextQuestion, ContextSourceRef, ContextSourceType,
    IdeationSession, Project, RecommendationReview, RiskAssessment, SolutionCritique,
    TaskProposal, VerificationRequirement,
};
use crate::error::{AppError, AppResult};

use super::types::{
    ClaimReviewCandidate, CompiledContextCandidate, ContextAssumptionCandidate,
    ContextClaimCandidate, ContextQuestionCandidate, EvidenceRef, RawContextBundle,
    RecommendationReviewCandidate, RiskAssessmentCandidate, SolutionCritiqueCandidate,
    VerificationRequirementCandidate,
};

pub(super) const SOURCE_EXCERPT_LIMIT: usize = 4_000;
const TRUNCATION_MARKER: &str = "\n[truncated]";

pub(super) fn ensure_plan_target(
    session: &IdeationSession,
    target_artifact_id: &str,
) -> AppResult<()> {
    let matches_owned = session
        .plan_artifact_id
        .as_ref()
        .is_some_and(|id| id.as_str() == target_artifact_id);
    let matches_inherited = session
        .inherited_plan_artifact_id
        .as_ref()
        .is_some_and(|id| id.as_str() == target_artifact_id);
    if matches_owned || matches_inherited {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Phase 1 solution critique targets the session plan artifact only".to_string(),
        ))
    }
}

pub(super) fn inline_artifact_content(artifact: &Artifact) -> AppResult<String> {
    match &artifact.content {
        ArtifactContent::Inline { text } => Ok(text.clone()),
        ArtifactContent::File { .. } => Err(AppError::Validation(format!(
            "Artifact {} uses file content; solution critique Phase 1 does not read filesystem paths",
            artifact.id
        ))),
    }
}

pub(super) fn artifact_source(
    source_type: ContextSourceType,
    prefix: &str,
    artifact: &Artifact,
    content: Option<&str>,
) -> ContextSourceRef {
    ContextSourceRef {
        source_type,
        id: format!("{prefix}:{}", artifact.id.as_str()),
        label: artifact.name.clone(),
        excerpt: content.map(|text| truncate_text(text, SOURCE_EXCERPT_LIMIT)),
        created_at: Some(artifact.metadata.created_at),
    }
}

pub(super) fn chat_message_source(message: &ChatMessage) -> ContextSourceRef {
    ContextSourceRef {
        source_type: ContextSourceType::ChatMessage,
        id: format!("chat_message:{}", message.id.as_str()),
        label: format!("{} message", message.role),
        excerpt: Some(truncate_text(&message.content, SOURCE_EXCERPT_LIMIT)),
        created_at: Some(message.created_at),
    }
}

pub(super) fn task_proposal_source(proposal: &TaskProposal) -> ContextSourceRef {
    let excerpt = [
        Some(format!("Title: {}", proposal.title)),
        proposal
            .description
            .as_ref()
            .map(|value| format!("Description: {value}")),
        proposal
            .steps
            .as_ref()
            .map(|value| format!("Steps: {value}")),
        proposal
            .acceptance_criteria
            .as_ref()
            .map(|value| format!("Acceptance criteria: {value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n");

    ContextSourceRef {
        source_type: ContextSourceType::TaskProposal,
        id: format!("task_proposal:{}", proposal.id.as_str()),
        label: proposal.title.clone(),
        excerpt: Some(truncate_text(&excerpt, SOURCE_EXCERPT_LIMIT)),
        created_at: Some(proposal.created_at),
    }
}

pub(super) fn verification_status_source(session: &IdeationSession) -> ContextSourceRef {
    ContextSourceRef {
        source_type: ContextSourceType::VerificationStatus,
        id: format!(
            "verification_status:{}:{}",
            session.id.as_str(),
            session.verification_generation
        ),
        label: "Verification status".to_string(),
        excerpt: Some(truncate_text(
            &json!({
                "status": session.verification_status,
                "in_progress": session.verification_in_progress,
                "generation": session.verification_generation,
                "current_round": session.verification_current_round,
                "max_rounds": session.verification_max_rounds,
                "gap_count": session.verification_gap_count,
                "gap_score": session.verification_gap_score,
                "convergence_reason": session.verification_convergence_reason,
            })
            .to_string(),
            SOURCE_EXCERPT_LIMIT,
        )),
        created_at: Some(session.updated_at),
    }
}

pub(super) fn project_analysis_sources(project: &Project) -> Vec<ContextSourceRef> {
    let created_at = project
        .analyzed_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));
    let mut sources = Vec::new();
    if let Some(analysis) = project.detected_analysis.as_deref() {
        sources.push(ContextSourceRef {
            source_type: ContextSourceType::ProjectAnalysis,
            id: format!("project_analysis:{}:detected", project.id.as_str()),
            label: "Detected project analysis".to_string(),
            excerpt: Some(truncate_text(analysis, SOURCE_EXCERPT_LIMIT)),
            created_at,
        });
    }
    if let Some(analysis) = project.custom_analysis.as_deref() {
        sources.push(ContextSourceRef {
            source_type: ContextSourceType::ProjectAnalysis,
            id: format!("project_analysis:{}:custom", project.id.as_str()),
            label: "Custom project analysis".to_string(),
            excerpt: Some(truncate_text(analysis, SOURCE_EXCERPT_LIMIT)),
            created_at,
        });
    }
    sources
}

pub(super) fn agent_run_source(run: &AgentRun) -> ContextSourceRef {
    ContextSourceRef {
        source_type: ContextSourceType::AgentRun,
        id: format!("agent_run:{}", run.id),
        label: format!("Agent run {}", run.status),
        excerpt: Some(truncate_text(
            &json!({
                "status": run.status,
                "started_at": run.started_at,
                "completed_at": run.completed_at,
                "error_message": run.error_message,
                "harness": run.harness,
                "provider_session_id": run.provider_session_id,
                "logical_model": run.logical_model,
                "effective_model_id": run.effective_model_id,
                "input_tokens": run.input_tokens,
                "output_tokens": run.output_tokens,
                "estimated_usd": run.estimated_usd,
            })
            .to_string(),
            SOURCE_EXCERPT_LIMIT,
        )),
        created_at: Some(run.started_at),
    }
}

pub(super) fn build_compiled_context(
    candidate: CompiledContextCandidate,
    bundle: &RawContextBundle,
) -> AppResult<CompiledContext> {
    let source_map = source_map(&bundle.sources);
    Ok(CompiledContext {
        id: String::new(),
        target: bundle.target.clone(),
        sources: bundle.sources.clone(),
        claims: candidate
            .claims
            .into_iter()
            .map(|claim| build_context_claim(claim, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        open_questions: candidate
            .open_questions
            .into_iter()
            .map(|question| build_context_question(question, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        stale_assumptions: candidate
            .stale_assumptions
            .into_iter()
            .map(|assumption| build_context_assumption(assumption, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        generated_at: Utc::now(),
    })
}

pub(super) fn build_solution_critique(
    candidate: SolutionCritiqueCandidate,
    artifact_id: &str,
    context_artifact_id: &str,
    context: &CompiledContext,
) -> AppResult<SolutionCritique> {
    let source_map = source_map(&context.sources);
    Ok(SolutionCritique {
        id: String::new(),
        artifact_id: artifact_id.to_string(),
        context_artifact_id: context_artifact_id.to_string(),
        verdict: candidate.verdict,
        confidence: candidate.confidence,
        claims: candidate
            .claims
            .into_iter()
            .map(|claim| build_claim_review(claim, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        recommendations: candidate
            .recommendations
            .into_iter()
            .map(|recommendation| build_recommendation_review(recommendation, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        risks: candidate
            .risks
            .into_iter()
            .map(|risk| build_risk_assessment(risk, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        verification_plan: candidate
            .verification_plan
            .into_iter()
            .map(|requirement| build_verification_requirement(requirement, &source_map))
            .collect::<AppResult<Vec<_>>>()?,
        safe_next_action: candidate.safe_next_action,
        generated_at: Utc::now(),
    })
}

pub(super) fn parse_candidate<T: serde::de::DeserializeOwned>(json: &str) -> AppResult<T> {
    serde_json::from_str(json)
        .map_err(|error| AppError::Validation(format!("Invalid solution critique JSON: {error}")))
}

pub(super) fn parse_inline_artifact<T: serde::de::DeserializeOwned>(
    artifact: &Artifact,
) -> AppResult<T> {
    let text = inline_artifact_content(artifact)?;
    serde_json::from_str(&text).map_err(|error| {
        AppError::Validation(format!(
            "Failed to parse artifact {} as structured JSON: {error}",
            artifact.id
        ))
    })
}

pub(super) fn to_pretty_json<T: Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string_pretty(value)
        .map_err(|error| AppError::Validation(format!("Failed to serialize artifact JSON: {error}")))
}

pub(super) fn sort_sources(sources: &mut [ContextSourceRef]) {
    sources.sort_by(|left, right| {
        source_type_rank(left.source_type)
            .cmp(&source_type_rank(right.source_type))
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub(super) fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}

pub(super) fn truncate_text(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated = text.chars().take(limit).collect::<String>();
    truncated.push_str(TRUNCATION_MARKER);
    truncated
}

fn build_context_claim(
    candidate: ContextClaimCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<ContextClaim> {
    Ok(ContextClaim {
        id: candidate.id,
        text: candidate.text,
        classification: candidate.classification,
        confidence: candidate.confidence,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
    })
}

fn build_context_question(
    candidate: ContextQuestionCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<ContextQuestion> {
    Ok(ContextQuestion {
        id: candidate.id,
        question: candidate.question,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
    })
}

fn build_context_assumption(
    candidate: ContextAssumptionCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<ContextAssumption> {
    Ok(ContextAssumption {
        id: candidate.id,
        text: candidate.text,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
    })
}

fn build_claim_review(
    candidate: ClaimReviewCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<ClaimReview> {
    Ok(ClaimReview {
        id: candidate.id,
        claim: candidate.claim,
        status: candidate.status,
        confidence: candidate.confidence,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
        notes: candidate.notes,
    })
}

fn build_recommendation_review(
    candidate: RecommendationReviewCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<RecommendationReview> {
    Ok(RecommendationReview {
        id: candidate.id,
        recommendation: candidate.recommendation,
        status: candidate.status,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
        rationale: candidate.rationale,
    })
}

fn build_risk_assessment(
    candidate: RiskAssessmentCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<RiskAssessment> {
    Ok(RiskAssessment {
        id: candidate.id,
        risk: candidate.risk,
        severity: candidate.severity,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
        mitigation: candidate.mitigation,
    })
}

fn build_verification_requirement(
    candidate: VerificationRequirementCandidate,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<VerificationRequirement> {
    Ok(VerificationRequirement {
        id: candidate.id,
        requirement: candidate.requirement,
        priority: candidate.priority,
        evidence: canonical_evidence(candidate.evidence, source_map)?,
        suggested_test: candidate.suggested_test,
    })
}

fn source_map(sources: &[ContextSourceRef]) -> HashMap<String, ContextSourceRef> {
    sources
        .iter()
        .map(|source| (source.id.clone(), source.clone()))
        .collect()
}

fn canonical_evidence(
    evidence: Vec<EvidenceRef>,
    source_map: &HashMap<String, ContextSourceRef>,
) -> AppResult<Vec<ContextSourceRef>> {
    evidence
        .into_iter()
        .map(|reference| {
            source_map.get(&reference.id).cloned().ok_or_else(|| {
                AppError::Validation(format!(
                    "Model output referenced unknown source id '{}'",
                    reference.id
                ))
            })
        })
        .collect()
}

fn source_type_rank(source_type: ContextSourceType) -> u8 {
    match source_type {
        ContextSourceType::PlanArtifact => 0,
        ContextSourceType::ChatMessage => 1,
        ContextSourceType::TaskProposal => 2,
        ContextSourceType::VerificationStatus => 3,
        ContextSourceType::VerificationGap => 4,
        ContextSourceType::ProjectAnalysis => 5,
        ContextSourceType::Artifact => 6,
        ContextSourceType::AgentRun => 7,
    }
}
