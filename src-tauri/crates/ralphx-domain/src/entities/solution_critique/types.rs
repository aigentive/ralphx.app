use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTargetRef {
    pub target_type: ContextTargetType,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTargetType {
    PlanArtifact,
    Artifact,
    ChatMessage,
    AgentRun,
    Task,
    TaskExecution,
    ReviewReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextSourceRef {
    pub source_type: ContextSourceType,
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceType {
    PlanArtifact,
    Task,
    ChatMessage,
    TaskProposal,
    VerificationStatus,
    VerificationGap,
    ProjectAnalysis,
    Artifact,
    AgentRun,
    Review,
    ReviewNote,
    ReviewIssue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledContext {
    pub id: String,
    pub target: ContextTargetRef,
    pub sources: Vec<ContextSourceRef>,
    pub claims: Vec<ContextClaim>,
    pub open_questions: Vec<ContextQuestion>,
    pub stale_assumptions: Vec<ContextAssumption>,
    pub generated_at: DateTime<Utc>,
}

impl CompiledContext {
    pub fn source_ids(&self) -> BTreeSet<&str> {
        self.sources
            .iter()
            .map(|source| source.id.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextClaim {
    pub id: String,
    pub text: String,
    pub classification: ContextClaimKind,
    pub confidence: CritiqueConfidence,
    pub evidence: Vec<ContextSourceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextClaimKind {
    Fact,
    Inference,
    Assumption,
    Speculation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextQuestion {
    pub id: String,
    pub question: String,
    pub evidence: Vec<ContextSourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextAssumption {
    pub id: String,
    pub text: String,
    pub evidence: Vec<ContextSourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolutionCritique {
    pub id: String,
    pub artifact_id: String,
    pub context_artifact_id: String,
    pub verdict: SolutionCritiqueVerdict,
    pub confidence: CritiqueConfidence,
    pub claims: Vec<ClaimReview>,
    pub recommendations: Vec<RecommendationReview>,
    pub risks: Vec<RiskAssessment>,
    pub verification_plan: Vec<VerificationRequirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_next_action: Option<String>,
    pub generated_at: DateTime<Utc>,
}

impl SolutionCritique {
    pub fn referenced_source_ids(&self) -> BTreeSet<&str> {
        let mut ids = BTreeSet::new();
        for claim in &self.claims {
            collect_source_ids(&claim.evidence, &mut ids);
        }
        for recommendation in &self.recommendations {
            collect_source_ids(&recommendation.evidence, &mut ids);
        }
        for risk in &self.risks {
            collect_source_ids(&risk.evidence, &mut ids);
        }
        for requirement in &self.verification_plan {
            collect_source_ids(&requirement.evidence, &mut ids);
        }
        ids
    }
}

fn collect_source_ids<'a>(sources: &'a [ContextSourceRef], ids: &mut BTreeSet<&'a str>) {
    for source in sources {
        ids.insert(source.id.as_str());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolutionCritiqueVerdict {
    Accept,
    Revise,
    Investigate,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritiqueConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimReview {
    pub id: String,
    pub claim: String,
    pub status: ClaimReviewStatus,
    pub confidence: CritiqueConfidence,
    pub evidence: Vec<ContextSourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReviewStatus {
    Supported,
    Unsupported,
    Contradicted,
    Unclear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendationReview {
    pub id: String,
    pub recommendation: String,
    pub status: RecommendationStatus,
    pub evidence: Vec<ContextSourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationStatus {
    Accept,
    Revise,
    Investigate,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub id: String,
    pub risk: String,
    pub severity: CritiqueSeverity,
    pub evidence: Vec<ContextSourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritiqueSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRequirement {
    pub id: String,
    pub requirement: String,
    pub priority: CritiqueSeverity,
    pub evidence: Vec<ContextSourceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_test: Option<String>,
}
