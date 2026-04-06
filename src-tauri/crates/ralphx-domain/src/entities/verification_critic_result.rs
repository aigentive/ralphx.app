use serde::{Deserialize, Serialize};

/// Identifies which type of critic produced a verification result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticKind {
    Completeness,
    Feasibility,
    Ux,
    CodeQuality,
    Intent,
    PromptQuality,
    PipelineSafety,
    StateMachine,
}

impl CriticKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CriticKind::Completeness => "completeness",
            CriticKind::Feasibility => "feasibility",
            CriticKind::Ux => "ux",
            CriticKind::CodeQuality => "code_quality",
            CriticKind::Intent => "intent",
            CriticKind::PromptQuality => "prompt_quality",
            CriticKind::PipelineSafety => "pipeline_safety",
            CriticKind::StateMachine => "state_machine",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "completeness" => Some(CriticKind::Completeness),
            "feasibility" => Some(CriticKind::Feasibility),
            "ux" => Some(CriticKind::Ux),
            "code_quality" => Some(CriticKind::CodeQuality),
            "intent" => Some(CriticKind::Intent),
            "prompt_quality" => Some(CriticKind::PromptQuality),
            "pipeline_safety" => Some(CriticKind::PipelineSafety),
            "state_machine" => Some(CriticKind::StateMachine),
            _ => None,
        }
    }
}

/// A stored verification critic result, linking a critic run to its artifact output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCriticResult {
    pub id: String,
    pub parent_session_id: String,
    /// The child verification session that ran this critic.
    pub verification_session_id: String,
    pub verification_generation: i32,
    pub round: i32,
    pub critic_kind: CriticKind,
    pub artifact_id: String,
    /// "complete" | "partial" | "error"
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
