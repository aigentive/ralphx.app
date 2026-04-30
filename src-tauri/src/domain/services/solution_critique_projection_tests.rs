use chrono::Utc;

use super::{project_solution_critique_gap_items, project_solution_critique_gaps};
use crate::domain::entities::{
    ClaimReview, ClaimReviewStatus, CritiqueConfidence, CritiqueSeverity,
    ProjectedCritiqueGapStatus, RecommendationReview, RecommendationStatus, RiskAssessment,
    SolutionCritique, SolutionCritiqueGapAction, SolutionCritiqueGapActionKind,
    SolutionCritiqueVerdict, VerificationRequirement,
};

fn critique(
    claims: Vec<ClaimReview>,
    risks: Vec<RiskAssessment>,
    verification_plan: Vec<VerificationRequirement>,
    recommendations: Vec<RecommendationReview>,
) -> SolutionCritique {
    SolutionCritique {
        id: "critique-1".to_string(),
        artifact_id: "plan-1".to_string(),
        context_artifact_id: "context-1".to_string(),
        verdict: SolutionCritiqueVerdict::Investigate,
        confidence: CritiqueConfidence::Medium,
        claims,
        recommendations,
        risks,
        verification_plan,
        safe_next_action: None,
        generated_at: Utc::now(),
    }
}

fn claim(status: ClaimReviewStatus, text: &str) -> ClaimReview {
    ClaimReview {
        id: format!("claim-{status:?}"),
        claim: text.to_string(),
        status,
        confidence: CritiqueConfidence::Medium,
        evidence: vec![],
        notes: None,
    }
}

fn risk(severity: CritiqueSeverity, text: &str) -> RiskAssessment {
    RiskAssessment {
        id: format!("risk-{severity:?}"),
        risk: text.to_string(),
        severity,
        evidence: vec![],
        mitigation: Some("Mitigate before trusting the plan.".to_string()),
    }
}

fn requirement(priority: CritiqueSeverity, text: &str) -> VerificationRequirement {
    VerificationRequirement {
        id: format!("requirement-{priority:?}"),
        requirement: text.to_string(),
        priority,
        evidence: vec![],
        suggested_test: Some("Run focused regression coverage.".to_string()),
    }
}

fn recommendation(status: RecommendationStatus, text: &str) -> RecommendationReview {
    RecommendationReview {
        id: format!("recommendation-{status:?}"),
        recommendation: text.to_string(),
        status,
        evidence: vec![],
        rationale: Some("Style-only note.".to_string()),
    }
}

#[test]
fn projects_high_signal_claim_reviews_to_gaps() {
    let critique = critique(
        vec![
            claim(
                ClaimReviewStatus::Supported,
                "The plan names the target module.",
            ),
            claim(
                ClaimReviewStatus::Unsupported,
                "The plan assumes a migration already exists.",
            ),
            claim(
                ClaimReviewStatus::Contradicted,
                "The plan says the runtime is single-harness.",
            ),
            claim(
                ClaimReviewStatus::Unclear,
                "The plan depends on an unstated handoff contract.",
            ),
        ],
        vec![],
        vec![],
        vec![],
    );

    let gaps = project_solution_critique_gaps(&critique);

    assert_eq!(gaps.len(), 3);
    assert_eq!(gaps[0].severity, "high");
    assert!(gaps[0].description.contains("Contradicted plan claim"));
    assert_eq!(gaps[1].severity, "high");
    assert!(gaps[1].description.contains("Unsupported plan claim"));
    assert_eq!(gaps[2].severity, "medium");
    assert!(gaps[2].description.contains("Unclear plan claim"));
    assert!(gaps
        .iter()
        .all(|gap| gap.category == "solution_critique_claim"));
}

#[test]
fn projects_risks_and_required_verification_without_low_signal_items() {
    let critique = critique(
        vec![],
        vec![
            risk(
                CritiqueSeverity::Critical,
                "The plan can delete user data without a rollback path.",
            ),
            risk(
                CritiqueSeverity::Low,
                "The implementation summary could use clearer prose.",
            ),
        ],
        vec![
            requirement(
                CritiqueSeverity::High,
                "Prove the first writer and reader both use the new context artifact.",
            ),
            requirement(
                CritiqueSeverity::Medium,
                "Add a regression test for missing proof obligations.",
            ),
            requirement(
                CritiqueSeverity::Low,
                "Consider renaming a local helper for readability.",
            ),
        ],
        vec![recommendation(
            RecommendationStatus::Revise,
            "Make the headings more readable.",
        )],
    );

    let gaps = project_solution_critique_gaps(&critique);

    assert_eq!(gaps.len(), 3);
    assert_eq!(gaps[0].severity, "critical");
    assert_eq!(gaps[0].category, "solution_critique_risk");
    assert_eq!(gaps[1].severity, "high");
    assert_eq!(gaps[1].category, "solution_critique_verification");
    assert_eq!(gaps[2].severity, "medium");
    assert_eq!(gaps[2].category, "solution_critique_verification");
    assert!(gaps
        .iter()
        .all(|gap| !gap.description.contains("clearer prose")));
    assert!(gaps
        .iter()
        .all(|gap| !gap.description.contains("headings more readable")));
}

#[test]
fn deduplicates_projected_gaps_by_fingerprint() {
    let critique = critique(
        vec![
            claim(
                ClaimReviewStatus::Unsupported,
                "The plan lacks a first writer contract.",
            ),
            claim(
                ClaimReviewStatus::Unsupported,
                "plan lacks first writer contract",
            ),
        ],
        vec![],
        vec![],
        vec![],
    );

    let gaps = project_solution_critique_gaps(&critique);

    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].description.contains("first writer contract"));
}

#[test]
fn projects_stable_gap_items_with_source_and_latest_action_status() {
    let critique = critique(
        vec![claim(
            ClaimReviewStatus::Unsupported,
            "The plan lacks a first writer contract.",
        )],
        vec![],
        vec![],
        vec![],
    );

    let first = project_solution_critique_gap_items(&critique, "critique-artifact-1", &[]);
    let second = project_solution_critique_gap_items(&critique, "critique-artifact-1", &[]);

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id, second[0].id);
    assert_eq!(first[0].status, ProjectedCritiqueGapStatus::Open);
    assert_eq!(first[0].origin.item_id, "claim-Unsupported");
    let expected_source = format!("solution_critique:critique-artifact-1:{}", first[0].id);
    assert_eq!(
        first[0].verification_gap.source.as_deref(),
        Some(expected_source.as_str())
    );

    let action = SolutionCritiqueGapAction {
        id: "action-1".to_string(),
        session_id: "session-1".to_string(),
        project_id: "project-1".to_string(),
        target_type: crate::domain::entities::ContextTargetType::PlanArtifact,
        target_id: "plan-1".to_string(),
        critique_artifact_id: "critique-artifact-1".to_string(),
        context_artifact_id: "context-1".to_string(),
        gap_id: first[0].id.clone(),
        gap_fingerprint: first[0].fingerprint.clone(),
        action: SolutionCritiqueGapActionKind::Deferred,
        note: None,
        actor_kind: "human".to_string(),
        verification_generation: None,
        promoted_round: None,
        created_at: Utc::now(),
    };
    let with_action =
        project_solution_critique_gap_items(&critique, "critique-artifact-1", &[action]);
    assert_eq!(with_action[0].id, first[0].id);
    assert_eq!(with_action[0].status, ProjectedCritiqueGapStatus::Deferred);
}
