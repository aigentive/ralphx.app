use chrono::{TimeZone, Utc};
use serde_json::json;

use super::*;

fn source_ref(id: &str) -> ContextSourceRef {
    ContextSourceRef {
        source_type: ContextSourceType::PlanArtifact,
        id: id.to_string(),
        label: "Plan artifact".to_string(),
        excerpt: Some("Plan excerpt".to_string()),
        created_at: Some(Utc.with_ymd_and_hms(2026, 4, 29, 12, 0, 0).unwrap()),
    }
}

#[test]
fn compiled_context_serializes_with_snake_case_enums() {
    let source = source_ref("plan:artifact-1");
    let context = CompiledContext {
        id: "ctx-1".to_string(),
        target: ContextTargetRef {
            target_type: ContextTargetType::PlanArtifact,
            id: "artifact-1".to_string(),
            label: "Implementation plan".to_string(),
        },
        sources: vec![source.clone()],
        claims: vec![ContextClaim {
            id: "claim-1".to_string(),
            text: "The plan targets ideation verification.".to_string(),
            classification: ContextClaimKind::Fact,
            confidence: CritiqueConfidence::High,
            evidence: vec![source],
        }],
        open_questions: vec![],
        stale_assumptions: vec![],
        generated_at: Utc.with_ymd_and_hms(2026, 4, 29, 12, 1, 0).unwrap(),
    };

    let value = serde_json::to_value(&context).unwrap();

    assert_eq!(value["target"]["target_type"], "plan_artifact");
    assert_eq!(value["sources"][0]["source_type"], "plan_artifact");
    assert_eq!(value["claims"][0]["classification"], "fact");
    assert_eq!(value["claims"][0]["confidence"], "high");
}

#[test]
fn solution_critique_serializes_with_snake_case_enums() {
    let source = source_ref("plan:artifact-1");
    let critique = SolutionCritique {
        id: "critique-1".to_string(),
        artifact_id: "artifact-1".to_string(),
        context_artifact_id: "context-artifact-1".to_string(),
        verdict: SolutionCritiqueVerdict::Investigate,
        confidence: CritiqueConfidence::Medium,
        claims: vec![ClaimReview {
            id: "review-1".to_string(),
            claim: "The plan is fully backed by current state.".to_string(),
            status: ClaimReviewStatus::Unsupported,
            confidence: CritiqueConfidence::Low,
            evidence: vec![source.clone()],
            notes: None,
        }],
        recommendations: vec![RecommendationReview {
            id: "rec-1".to_string(),
            recommendation: "Add a focused handler test.".to_string(),
            status: RecommendationStatus::Accept,
            evidence: vec![source.clone()],
            rationale: None,
        }],
        risks: vec![RiskAssessment {
            id: "risk-1".to_string(),
            risk: "Unvalidated source ids could invent evidence.".to_string(),
            severity: CritiqueSeverity::High,
            evidence: vec![source.clone()],
            mitigation: None,
        }],
        verification_plan: vec![VerificationRequirement {
            id: "verify-1".to_string(),
            requirement: "Assert handler returns persisted payload.".to_string(),
            priority: CritiqueSeverity::Medium,
            evidence: vec![source],
            suggested_test: Some("HTTP handler integration test".to_string()),
        }],
        safe_next_action: Some("Run targeted handler tests.".to_string()),
        generated_at: Utc.with_ymd_and_hms(2026, 4, 29, 12, 2, 0).unwrap(),
    };

    let value = serde_json::to_value(&critique).unwrap();

    assert_eq!(value["verdict"], "investigate");
    assert_eq!(value["confidence"], "medium");
    assert_eq!(value["claims"][0]["status"], "unsupported");
    assert_eq!(value["recommendations"][0]["status"], "accept");
    assert_eq!(value["risks"][0]["severity"], "high");
    assert_eq!(value["verification_plan"][0]["priority"], "medium");
}

#[test]
fn source_id_helpers_return_deterministic_sets() {
    let source = source_ref("plan:artifact-1");
    let context = CompiledContext {
        id: "ctx-1".to_string(),
        target: ContextTargetRef {
            target_type: ContextTargetType::PlanArtifact,
            id: "artifact-1".to_string(),
            label: "Implementation plan".to_string(),
        },
        sources: vec![source.clone()],
        claims: vec![],
        open_questions: vec![],
        stale_assumptions: vec![],
        generated_at: Utc::now(),
    };
    let critique = SolutionCritique {
        id: "critique-1".to_string(),
        artifact_id: "artifact-1".to_string(),
        context_artifact_id: "context-artifact-1".to_string(),
        verdict: SolutionCritiqueVerdict::Revise,
        confidence: CritiqueConfidence::High,
        claims: vec![ClaimReview {
            id: "review-1".to_string(),
            claim: "Needs evidence.".to_string(),
            status: ClaimReviewStatus::Unclear,
            confidence: CritiqueConfidence::Medium,
            evidence: vec![source],
            notes: None,
        }],
        recommendations: vec![],
        risks: vec![],
        verification_plan: vec![],
        safe_next_action: None,
        generated_at: Utc::now(),
    };

    assert_eq!(context.source_ids().into_iter().collect::<Vec<_>>(), vec!["plan:artifact-1"]);
    assert_eq!(
        critique.referenced_source_ids().into_iter().collect::<Vec<_>>(),
        vec!["plan:artifact-1"]
    );
}

#[test]
fn deserializes_expected_json_shape() {
    let value = json!({
        "id": "ctx-1",
        "target": {
            "target_type": "plan_artifact",
            "id": "artifact-1",
            "label": "Implementation plan"
        },
        "sources": [{
            "source_type": "verification_gap",
            "id": "gap:g1",
            "label": "Critical gap",
            "excerpt": "Missing tests",
            "created_at": "2026-04-29T12:00:00Z"
        }],
        "claims": [],
        "open_questions": [],
        "stale_assumptions": [],
        "generated_at": "2026-04-29T12:01:00Z"
    });

    let parsed: CompiledContext = serde_json::from_value(value).unwrap();

    assert_eq!(parsed.sources[0].source_type, ContextSourceType::VerificationGap);
}
