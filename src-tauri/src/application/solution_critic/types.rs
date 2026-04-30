use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    ClaimReviewStatus, ContextClaimKind, ContextSourceRef, ContextTargetRef, ContextTargetType,
    CritiqueConfidence, CritiqueSeverity, ProjectedCritiqueGap, RecommendationStatus,
    SolutionCritiqueGapAction, SolutionCritiqueGapActionKind, SolutionCritiqueVerdict,
    VerificationGap,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceLimits {
    #[serde(default)]
    pub chat_messages: Option<u32>,
    #[serde(default)]
    pub task_proposals: Option<u32>,
    #[serde(default)]
    pub related_artifacts: Option<u32>,
    #[serde(default)]
    pub agent_runs: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EffectiveSourceLimits {
    pub chat_messages: u32,
    pub task_proposals: u32,
    pub related_artifacts: u32,
    pub agent_runs: u32,
}

impl SourceLimits {
    pub(crate) fn effective(&self) -> EffectiveSourceLimits {
        EffectiveSourceLimits {
            chat_messages: clamp(self.chat_messages, 40, 100),
            task_proposals: clamp(self.task_proposals, 40, 100),
            related_artifacts: clamp(self.related_artifacts, 10, 25),
            agent_runs: clamp(self.agent_runs, 10, 25),
        }
    }
}

fn clamp(value: Option<u32>, default: u32, max: u32) -> u32 {
    value.unwrap_or(default).min(max)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileContextRequest {
    #[serde(default)]
    pub target_artifact_id: Option<String>,
    #[serde(default)]
    pub target_type: Option<ContextTargetType>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target: Option<ContextTargetRequest>,
    #[serde(default)]
    pub source_limits: SourceLimits,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileContextResult {
    pub artifact_id: String,
    pub compiled_context: crate::domain::entities::CompiledContext,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompiledContextReadResult {
    pub artifact_id: String,
    pub compiled_context: crate::domain::entities::CompiledContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueArtifactRequest {
    #[serde(default)]
    pub target_artifact_id: Option<String>,
    #[serde(default)]
    pub target_type: Option<ContextTargetType>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target: Option<ContextTargetRequest>,
    pub compiled_context_artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTargetRequest {
    pub target_type: ContextTargetType,
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
}

impl CompileContextRequest {
    pub fn for_target(target_type: ContextTargetType, id: impl Into<String>) -> Self {
        Self {
            target_artifact_id: None,
            target_type: Some(target_type),
            target_id: Some(id.into()),
            target: None,
            source_limits: SourceLimits::default(),
        }
    }

    pub fn for_plan_artifact(target_artifact_id: impl Into<String>) -> Self {
        Self {
            target_artifact_id: Some(target_artifact_id.into()),
            target_type: None,
            target_id: None,
            target: None,
            source_limits: SourceLimits::default(),
        }
    }

    pub(crate) fn target_request(&self) -> Option<ContextTargetRequest> {
        normalized_target_request(
            self.target.clone(),
            self.target_type,
            self.target_id.clone(),
            self.target_artifact_id.clone(),
        )
    }
}

impl CritiqueArtifactRequest {
    pub fn for_target(
        target_type: ContextTargetType,
        id: impl Into<String>,
        compiled_context_artifact_id: impl Into<String>,
    ) -> Self {
        Self {
            target_artifact_id: None,
            target_type: Some(target_type),
            target_id: Some(id.into()),
            target: None,
            compiled_context_artifact_id: compiled_context_artifact_id.into(),
        }
    }

    pub fn for_plan_artifact(
        target_artifact_id: impl Into<String>,
        compiled_context_artifact_id: impl Into<String>,
    ) -> Self {
        Self {
            target_artifact_id: Some(target_artifact_id.into()),
            target_type: None,
            target_id: None,
            target: None,
            compiled_context_artifact_id: compiled_context_artifact_id.into(),
        }
    }

    pub(crate) fn target_request(&self) -> Option<ContextTargetRequest> {
        normalized_target_request(
            self.target.clone(),
            self.target_type,
            self.target_id.clone(),
            self.target_artifact_id.clone(),
        )
    }
}

fn normalized_target_request(
    explicit: Option<ContextTargetRequest>,
    target_type: Option<ContextTargetType>,
    target_id: Option<String>,
    target_artifact_id: Option<String>,
) -> Option<ContextTargetRequest> {
    if let Some(target) = explicit {
        return Some(target);
    }
    if let (Some(target_type), Some(id)) = (target_type, target_id) {
        return Some(ContextTargetRequest {
            target_type,
            id,
            label: None,
        });
    }
    target_artifact_id.map(|id| ContextTargetRequest {
        target_type: ContextTargetType::PlanArtifact,
        id,
        label: None,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct CritiqueArtifactResult {
    pub artifact_id: String,
    pub solution_critique: crate::domain::entities::SolutionCritique,
    pub projected_gaps: Vec<VerificationGap>,
    pub projected_gap_items: Vec<ProjectedCritiqueGap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueReadResult {
    pub artifact_id: String,
    pub solution_critique: crate::domain::entities::SolutionCritique,
    pub projected_gaps: Vec<VerificationGap>,
    pub projected_gap_items: Vec<ProjectedCritiqueGap>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyProjectedGapActionRequest {
    pub action: SolutionCritiqueGapActionKind,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectedCritiqueGapActionResult {
    pub gap: ProjectedCritiqueGap,
    pub action: SolutionCritiqueGapAction,
    pub verification_updated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_generation: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueGapActionSummary {
    pub gap_id: String,
    pub gap_fingerprint: String,
    pub action: SolutionCritiqueGapActionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_generation: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueHistoryItem {
    pub artifact_id: String,
    pub context_artifact_id: String,
    pub target: ContextTargetRef,
    pub verdict: SolutionCritiqueVerdict,
    pub confidence: CritiqueConfidence,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub source_count: usize,
    pub claim_count: usize,
    pub risk_count: usize,
    pub projected_gap_count: usize,
    pub stale: bool,
    pub latest_gap_actions: Vec<SolutionCritiqueGapActionSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompiledContextHistoryItem {
    pub artifact_id: String,
    pub target: ContextTargetRef,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub source_count: usize,
    pub claim_count: usize,
    pub open_question_count: usize,
    pub stale_assumption_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueSessionRollup {
    pub session_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub target_count: usize,
    pub critique_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_verdict: Option<SolutionCritiqueVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highest_risk: Option<CritiqueSeverity>,
    pub stale_count: usize,
    pub promoted_gap_count: usize,
    pub deferred_gap_count: usize,
    pub covered_gap_count: usize,
    pub targets: Vec<SolutionCritiqueTargetRollupItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueTargetRollupItem {
    pub target: ContextTargetRef,
    pub artifact_id: String,
    pub context_artifact_id: String,
    pub verdict: SolutionCritiqueVerdict,
    pub confidence: CritiqueConfidence,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub stale: bool,
    pub risk_count: usize,
    pub projected_gap_count: usize,
    pub promoted_gap_count: usize,
    pub deferred_gap_count: usize,
    pub covered_gap_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawContextBundle {
    pub session_id: String,
    pub project_id: String,
    pub target: ContextTargetRef,
    pub target_content: String,
    pub sources: Vec<ContextSourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContextCandidate {
    #[serde(default)]
    pub claims: Vec<ContextClaimCandidate>,
    #[serde(default)]
    pub open_questions: Vec<ContextQuestionCandidate>,
    #[serde(default)]
    pub stale_assumptions: Vec<ContextAssumptionCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextClaimCandidate {
    pub id: String,
    pub text: String,
    pub classification: ContextClaimKind,
    pub confidence: CritiqueConfidence,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuestionCandidate {
    pub id: String,
    pub question: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssumptionCandidate {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionCritiqueCandidate {
    pub verdict: SolutionCritiqueVerdict,
    pub confidence: CritiqueConfidence,
    #[serde(default)]
    pub claims: Vec<ClaimReviewCandidate>,
    #[serde(default)]
    pub recommendations: Vec<RecommendationReviewCandidate>,
    #[serde(default)]
    pub risks: Vec<RiskAssessmentCandidate>,
    #[serde(default)]
    pub verification_plan: Vec<VerificationRequirementCandidate>,
    #[serde(default)]
    pub safe_next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimReviewCandidate {
    pub id: String,
    pub claim: String,
    pub status: ClaimReviewStatus,
    pub confidence: CritiqueConfidence,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationReviewCandidate {
    pub id: String,
    pub recommendation: String,
    pub status: RecommendationStatus,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentCandidate {
    pub id: String,
    pub risk: String,
    pub severity: CritiqueSeverity,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequirementCandidate {
    pub id: String,
    pub requirement: String,
    pub priority: CritiqueSeverity,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub suggested_test: Option<String>,
}
