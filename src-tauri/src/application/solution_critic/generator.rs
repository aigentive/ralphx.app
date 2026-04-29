use async_trait::async_trait;

use crate::domain::entities::{
    ClaimReviewStatus, ContextClaimKind, ContextSourceType, CritiqueConfidence, CritiqueSeverity,
    RecommendationStatus, SolutionCritiqueVerdict,
};
use crate::error::{AppError, AppResult};

use super::types::{
    ClaimReviewCandidate, CompiledContextCandidate, ContextAssumptionCandidate,
    ContextClaimCandidate, ContextQuestionCandidate, EvidenceRef, RawContextBundle,
    RecommendationReviewCandidate, RiskAssessmentCandidate, SolutionCritiqueCandidate,
    VerificationRequirementCandidate,
};

#[async_trait]
pub trait SolutionCritiqueGenerator: Send + Sync {
    async fn compile_context_candidate(&self, bundle: &RawContextBundle) -> AppResult<String>;

    async fn critique_candidate(
        &self,
        bundle: &RawContextBundle,
        context: &crate::domain::entities::CompiledContext,
    ) -> AppResult<String>;
}

#[derive(Debug, Default)]
pub struct DeterministicSolutionCritiqueGenerator;

#[async_trait]
impl SolutionCritiqueGenerator for DeterministicSolutionCritiqueGenerator {
    async fn compile_context_candidate(&self, bundle: &RawContextBundle) -> AppResult<String> {
        let target_ref = EvidenceRef {
            id: format!("plan_artifact:{}", bundle.target.id),
        };
        let mut claims = vec![ContextClaimCandidate {
            id: "claim_target_plan".to_string(),
            text: format!("The selected target is {}.", bundle.target.label),
            classification: ContextClaimKind::Fact,
            confidence: CritiqueConfidence::High,
            evidence: vec![target_ref.clone()],
        }];

        if bundle
            .sources
            .iter()
            .any(|source| source.source_type == ContextSourceType::VerificationGap)
        {
            claims.push(ContextClaimCandidate {
                id: "claim_verification_gaps_present".to_string(),
                text: "Current verification state includes unresolved gaps.".to_string(),
                classification: ContextClaimKind::Fact,
                confidence: CritiqueConfidence::Medium,
                evidence: bundle
                    .sources
                    .iter()
                    .filter(|source| source.source_type == ContextSourceType::VerificationGap)
                    .map(|source| EvidenceRef {
                        id: source.id.clone(),
                    })
                    .collect(),
            });
        }

        let candidate = CompiledContextCandidate {
            claims,
            open_questions: vec![ContextQuestionCandidate {
                id: "question_evidence_sufficiency".to_string(),
                question: "Is each implementation claim in the target backed by collected evidence?"
                    .to_string(),
                evidence: vec![target_ref.clone()],
            }],
            stale_assumptions: vec![ContextAssumptionCandidate {
                id: "assumption_current_state".to_string(),
                text: "Collected chat, proposal, artifact, and verification sources reflect the current plan state."
                    .to_string(),
                evidence: vec![target_ref],
            }],
        };

        to_json(&candidate)
    }

    async fn critique_candidate(
        &self,
        bundle: &RawContextBundle,
        context: &crate::domain::entities::CompiledContext,
    ) -> AppResult<String> {
        let target_ref = EvidenceRef {
            id: format!("plan_artifact:{}", bundle.target.id),
        };
        let has_open_questions = !context.open_questions.is_empty();
        let has_gaps = context
            .sources
            .iter()
            .any(|source| source.source_type == ContextSourceType::VerificationGap);
        let verdict = if has_gaps || has_open_questions {
            SolutionCritiqueVerdict::Investigate
        } else {
            SolutionCritiqueVerdict::Revise
        };

        let candidate = SolutionCritiqueCandidate {
            verdict,
            confidence: CritiqueConfidence::Medium,
            claims: vec![ClaimReviewCandidate {
                id: "claim_review_target_supported".to_string(),
                claim: "The target artifact should be trusted only where claims map to collected sources."
                    .to_string(),
                status: ClaimReviewStatus::Unclear,
                confidence: CritiqueConfidence::Medium,
                evidence: vec![target_ref.clone()],
                notes: Some("Deterministic review requires a follow-up model pass for full semantic scoring.".to_string()),
            }],
            recommendations: vec![RecommendationReviewCandidate {
                id: "recommendation_verify_evidence".to_string(),
                recommendation: "Verify unsupported or unclear plan claims before implementation.".to_string(),
                status: RecommendationStatus::Accept,
                evidence: vec![target_ref.clone()],
                rationale: Some("Phase 1 stores critique artifacts without mutating verification state.".to_string()),
            }],
            risks: vec![RiskAssessmentCandidate {
                id: "risk_unsupported_claims".to_string(),
                risk: "Unsupported plan claims may lead to incorrect implementation work.".to_string(),
                severity: CritiqueSeverity::Medium,
                evidence: vec![target_ref.clone()],
                mitigation: Some("Run focused verification against the listed requirements.".to_string()),
            }],
            verification_plan: vec![VerificationRequirementCandidate {
                id: "verify_claim_evidence".to_string(),
                requirement: "Check that every major target claim has at least one concrete source."
                    .to_string(),
                priority: CritiqueSeverity::Medium,
                evidence: vec![target_ref],
                suggested_test: None,
            }],
            safe_next_action: Some("Inspect the persisted critique and verify unclear claims.".to_string()),
        };

        to_json(&candidate)
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string(value)
        .map_err(|error| AppError::Validation(format!("Failed to serialize candidate JSON: {error}")))
}
