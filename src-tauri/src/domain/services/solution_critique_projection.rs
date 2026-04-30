use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::domain::entities::{
    ClaimReview, ClaimReviewStatus, CritiqueSeverity, ProjectedCritiqueGap,
    ProjectedCritiqueGapOrigin, ProjectedCritiqueGapOriginKind, ProjectedCritiqueGapStatus,
    RiskAssessment, SolutionCritique, SolutionCritiqueGapAction, SolutionCritiqueGapActionKind,
    VerificationGap, VerificationRequirement,
};

use super::gap_fingerprint::gap_fingerprint;

pub fn project_solution_critique_gaps(critique: &SolutionCritique) -> Vec<VerificationGap> {
    let gaps = gap_candidates(critique)
        .into_iter()
        .map(|candidate| candidate.gap)
        .collect::<Vec<_>>();

    dedupe_and_sort_gaps(gaps)
}

pub fn project_solution_critique_gap_items(
    critique: &SolutionCritique,
    critique_artifact_id: &str,
    latest_actions: &[SolutionCritiqueGapAction],
) -> Vec<ProjectedCritiqueGap> {
    let mut items = gap_candidates(critique)
        .into_iter()
        .map(|candidate| {
            let fingerprint = gap_fingerprint(&candidate.gap.description);
            let id = projected_gap_id(
                critique_artifact_id,
                candidate.origin.kind,
                &candidate.origin.item_id,
                &fingerprint,
            );
            let latest_action = latest_action_for_gap(latest_actions, &id);
            let status = latest_action
                .as_ref()
                .map(action_status)
                .unwrap_or(ProjectedCritiqueGapStatus::Open);
            let mut verification_gap = candidate.gap;
            verification_gap.source =
                Some(format!("solution_critique:{critique_artifact_id}:{id}"));

            ProjectedCritiqueGap {
                id,
                critique_artifact_id: critique_artifact_id.to_string(),
                context_artifact_id: critique.context_artifact_id.clone(),
                origin: candidate.origin,
                fingerprint,
                status,
                verification_gap,
                latest_action,
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        severity_rank(&left.verification_gap.severity)
            .cmp(&severity_rank(&right.verification_gap.severity))
            .then_with(|| {
                left.verification_gap
                    .category
                    .cmp(&right.verification_gap.category)
            })
            .then_with(|| {
                left.verification_gap
                    .description
                    .cmp(&right.verification_gap.description)
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    items
}

fn gap_candidates(critique: &SolutionCritique) -> Vec<GapCandidate> {
    let mut candidates = Vec::new();

    for claim in &critique.claims {
        if let Some(gap) = project_claim_gap(claim) {
            candidates.push(GapCandidate {
                origin: ProjectedCritiqueGapOrigin {
                    kind: ProjectedCritiqueGapOriginKind::Claim,
                    item_id: claim.id.clone(),
                },
                gap,
            });
        }
    }
    for risk in &critique.risks {
        if let Some(gap) = project_risk_gap(risk) {
            candidates.push(GapCandidate {
                origin: ProjectedCritiqueGapOrigin {
                    kind: ProjectedCritiqueGapOriginKind::Risk,
                    item_id: risk.id.clone(),
                },
                gap,
            });
        }
    }
    for requirement in &critique.verification_plan {
        if let Some(gap) = project_verification_gap(requirement) {
            candidates.push(GapCandidate {
                origin: ProjectedCritiqueGapOrigin {
                    kind: ProjectedCritiqueGapOriginKind::Verification,
                    item_id: requirement.id.clone(),
                },
                gap,
            });
        }
    }

    candidates
}

struct GapCandidate {
    origin: ProjectedCritiqueGapOrigin,
    gap: VerificationGap,
}

fn project_claim_gap(claim: &ClaimReview) -> Option<VerificationGap> {
    let (severity, label, why_it_matters) = match claim.status {
        ClaimReviewStatus::Supported => return None,
        ClaimReviewStatus::Unsupported => (
            "high",
            "Unsupported plan claim",
            "A plan claim lacks supporting evidence in the compiled context.",
        ),
        ClaimReviewStatus::Contradicted => (
            "high",
            "Contradicted plan claim",
            "A plan claim conflicts with evidence in the compiled context.",
        ),
        ClaimReviewStatus::Unclear => (
            "medium",
            "Unclear plan claim",
            "A plan claim needs more evidence before it should be trusted.",
        ),
    };

    Some(VerificationGap {
        severity: severity.to_string(),
        category: "solution_critique_claim".to_string(),
        description: format!("{label}: {}", claim.claim),
        why_it_matters: Some(
            claim
                .notes
                .clone()
                .unwrap_or_else(|| why_it_matters.to_string()),
        ),
        source: None,
    })
}

fn project_risk_gap(risk: &RiskAssessment) -> Option<VerificationGap> {
    let severity = severity_to_gap(&risk.severity)?;
    Some(VerificationGap {
        severity,
        category: "solution_critique_risk".to_string(),
        description: format!("Risk requires resolution: {}", risk.risk),
        why_it_matters: risk.mitigation.clone(),
        source: None,
    })
}

fn project_verification_gap(requirement: &VerificationRequirement) -> Option<VerificationGap> {
    let severity = severity_to_gap(&requirement.priority)?;
    Some(VerificationGap {
        severity,
        category: "solution_critique_verification".to_string(),
        description: format!("Required verification: {}", requirement.requirement),
        why_it_matters: requirement
            .suggested_test
            .as_ref()
            .map(|test| format!("Suggested test: {test}")),
        source: None,
    })
}

fn severity_to_gap(severity: &CritiqueSeverity) -> Option<String> {
    match severity {
        CritiqueSeverity::Critical => Some("critical".to_string()),
        CritiqueSeverity::High => Some("high".to_string()),
        CritiqueSeverity::Medium => Some("medium".to_string()),
        CritiqueSeverity::Low => None,
    }
}

fn dedupe_and_sort_gaps(gaps: Vec<VerificationGap>) -> Vec<VerificationGap> {
    let mut seen = BTreeSet::new();
    let mut unique = gaps
        .into_iter()
        .filter(|gap| seen.insert(gap_fingerprint(&gap.description)))
        .collect::<Vec<_>>();
    unique.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.category.cmp(&right.category))
            .then_with(|| left.description.cmp(&right.description))
    });
    unique
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    }
}

fn projected_gap_id(
    critique_artifact_id: &str,
    origin_kind: ProjectedCritiqueGapOriginKind,
    item_id: &str,
    fingerprint: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(critique_artifact_id.as_bytes());
    hasher.update(b":");
    hasher.update(origin_kind_key(origin_kind).as_bytes());
    hasher.update(b":");
    hasher.update(item_id.as_bytes());
    hasher.update(b":");
    hasher.update(fingerprint.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn origin_kind_key(kind: ProjectedCritiqueGapOriginKind) -> &'static str {
    match kind {
        ProjectedCritiqueGapOriginKind::Claim => "claim",
        ProjectedCritiqueGapOriginKind::Risk => "risk",
        ProjectedCritiqueGapOriginKind::Verification => "verification",
    }
}

fn latest_action_for_gap(
    actions: &[SolutionCritiqueGapAction],
    gap_id: &str,
) -> Option<SolutionCritiqueGapAction> {
    actions
        .iter()
        .filter(|action| action.gap_id == gap_id)
        .max_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        })
        .cloned()
}

fn action_status(action: &SolutionCritiqueGapAction) -> ProjectedCritiqueGapStatus {
    match action.action {
        SolutionCritiqueGapActionKind::Promoted => ProjectedCritiqueGapStatus::Promoted,
        SolutionCritiqueGapActionKind::Deferred => ProjectedCritiqueGapStatus::Deferred,
        SolutionCritiqueGapActionKind::Covered => ProjectedCritiqueGapStatus::Covered,
        SolutionCritiqueGapActionKind::Reopened => ProjectedCritiqueGapStatus::Open,
    }
}

#[cfg(test)]
#[path = "solution_critique_projection_tests.rs"]
mod tests;
