use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    ClaimReviewStatus, ContextClaimKind, ContextSourceRef, ContextTargetRef, CritiqueConfidence,
    CritiqueSeverity, RecommendationStatus, SolutionCritiqueVerdict, VerificationGap,
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
    pub target_artifact_id: String,
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
    pub target_artifact_id: String,
    pub compiled_context_artifact_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CritiqueArtifactResult {
    pub artifact_id: String,
    pub solution_critique: crate::domain::entities::SolutionCritique,
    pub projected_gaps: Vec<VerificationGap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolutionCritiqueReadResult {
    pub artifact_id: String,
    pub solution_critique: crate::domain::entities::SolutionCritique,
    pub projected_gaps: Vec<VerificationGap>,
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
