use std::collections::BTreeSet;

use crate::domain::entities::{
    ClaimReview, ClaimReviewStatus, CritiqueSeverity, RiskAssessment, SolutionCritique,
    VerificationGap, VerificationRequirement,
};

use super::gap_fingerprint::gap_fingerprint;

pub fn project_solution_critique_gaps(critique: &SolutionCritique) -> Vec<VerificationGap> {
    let mut gaps = Vec::new();

    for claim in &critique.claims {
        if let Some(gap) = project_claim_gap(claim) {
            gaps.push(gap);
        }
    }
    for risk in &critique.risks {
        if let Some(gap) = project_risk_gap(risk) {
            gaps.push(gap);
        }
    }
    for requirement in &critique.verification_plan {
        if let Some(gap) = project_verification_gap(requirement) {
            gaps.push(gap);
        }
    }

    dedupe_and_sort_gaps(gaps)
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

#[cfg(test)]
#[path = "solution_critique_projection_tests.rs"]
mod tests;
