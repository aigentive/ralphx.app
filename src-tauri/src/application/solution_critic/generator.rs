use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::domain::agents::{AgentHandle, AgentRole, AgenticClient, ClientType};
use crate::domain::entities::{
    ClaimReviewStatus, CompiledContext, ContextClaimKind, ContextSourceType, CritiqueConfidence,
    CritiqueSeverity, RecommendationStatus, SolutionCritiqueVerdict,
};
use crate::error::{AppError, AppResult};

use super::types::{
    ClaimReviewCandidate, CompiledContextCandidate, ContextAssumptionCandidate,
    ContextClaimCandidate, ContextQuestionCandidate, EvidenceRef, RawContextBundle,
    RecommendationReviewCandidate, RiskAssessmentCandidate, SolutionCritiqueCandidate,
    VerificationRequirementCandidate,
};

const MODEL_ROLE: &str = "solution-critic";
const PROMPT_TARGET_LIMIT: usize = 12_000;
const PROMPT_SOURCE_EXCERPT_LIMIT: usize = 1_500;

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

pub struct AgentSolutionCritiqueGenerator {
    client: Arc<dyn AgenticClient>,
}

impl AgentSolutionCritiqueGenerator {
    pub fn new(client: Arc<dyn AgenticClient>) -> Self {
        Self { client }
    }

    async fn run_json_prompt(&self, prompt: String) -> AppResult<String> {
        let role = AgentRole::Custom(MODEL_ROLE.to_string());
        let handle = AgentHandle::new(ClientType::Custom(MODEL_ROLE.to_string()), role);
        let response = self.client.send_prompt(&handle, &prompt).await?;
        extract_agent_json_object(&response.content)
    }

    async fn run_schema_validated_json_prompt<T, F>(
        &self,
        prompt: String,
        schema_name: &'static str,
        repair_prompt: F,
    ) -> AppResult<String>
    where
        T: serde::de::DeserializeOwned,
        F: FnOnce(&str, &str) -> AppResult<String>,
    {
        let json = self.run_json_prompt(prompt).await?;
        match serde_json::from_str::<T>(&json) {
            Ok(_) => Ok(json),
            Err(error) => {
                tracing::warn!(
                    schema = schema_name,
                    error = %error,
                    "Solution critic model returned invalid schema; attempting repair"
                );
                let repaired_json = self
                    .run_json_prompt(repair_prompt(&json, &error.to_string())?)
                    .await?;
                serde_json::from_str::<T>(&repaired_json).map_err(|repair_error| {
                    AppError::Validation(format!(
                        "Invalid solution critique JSON after {schema_name} repair: {repair_error}"
                    ))
                })?;
                Ok(repaired_json)
            }
        }
    }
}

#[async_trait]
impl SolutionCritiqueGenerator for AgentSolutionCritiqueGenerator {
    async fn compile_context_candidate(&self, bundle: &RawContextBundle) -> AppResult<String> {
        self.run_schema_validated_json_prompt::<CompiledContextCandidate, _>(
            build_context_compiler_prompt(bundle)?,
            "compiled_context",
            |invalid_json, validation_error| {
                build_context_compiler_repair_prompt(bundle, invalid_json, validation_error)
            },
        )
        .await
    }

    async fn critique_candidate(
        &self,
        bundle: &RawContextBundle,
        context: &CompiledContext,
    ) -> AppResult<String> {
        self.run_schema_validated_json_prompt::<SolutionCritiqueCandidate, _>(
            build_solution_critique_prompt(bundle, context)?,
            "solution_critique",
            |invalid_json, validation_error| {
                build_solution_critique_repair_prompt(
                    bundle,
                    context,
                    invalid_json,
                    validation_error,
                )
            },
        )
        .await
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string(value).map_err(|error| {
        AppError::Validation(format!("Failed to serialize candidate JSON: {error}"))
    })
}

fn build_context_compiler_prompt(bundle: &RawContextBundle) -> AppResult<String> {
    let payload = prompt_bundle_payload(bundle);
    let payload_json = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::Validation(format!(
            "Failed to serialize solution critic prompt: {error}"
        ))
    })?;

    Ok(format!(
        r#"You are RalphX's solution context compiler.

Task:
- Read the target and collected sources.
- Extract the implementation claims, open questions, and stale assumptions that a later critique must judge.
- Be evidence-bound: every evidence id must be copied exactly from the supplied source ids.
- Do not invent source ids or facts.
- Prefer specific claims about implementation, verification, risk, and target accuracy over generic observations.

Return strict JSON only. Do not wrap it in markdown.

Allowed enum values:
- classification: "fact", "inference", "assumption", "speculation"
- confidence: "low", "medium", "high"

Required JSON shape:
{{
  "claims": [
    {{
      "id": "short_stable_id",
      "text": "claim text",
      "classification": "fact|inference|assumption|speculation",
      "confidence": "low|medium|high",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ],
  "open_questions": [
    {{
      "id": "short_stable_id",
      "question": "question text",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ],
  "stale_assumptions": [
    {{
      "id": "short_stable_id",
      "text": "assumption text",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ]
}}

Input:
{payload_json}"#
    ))
}

fn build_context_compiler_repair_prompt(
    bundle: &RawContextBundle,
    invalid_json: &str,
    validation_error: &str,
) -> AppResult<String> {
    let payload = json!({
        "raw_context": prompt_bundle_payload(bundle),
        "validation_error": validation_error,
        "invalid_response": truncate_for_prompt(invalid_json, PROMPT_TARGET_LIMIT),
    });
    let payload_json = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::Validation(format!(
            "Failed to serialize solution critic repair prompt: {error}"
        ))
    })?;

    Ok(format!(
        r#"You are RalphX's solution context compiler.

Your previous response did not match the compiled context schema.

Repair task:
- Return one valid JSON object only.
- Use the raw context and source ids below.
- Copy evidence ids exactly from allowed_source_ids.
- Do not invent source ids or facts.
- Do not return a solution critique schema.

Required root fields:
- claims
- open_questions
- stale_assumptions

Allowed enum values:
- classification: "fact", "inference", "assumption", "speculation"
- confidence: "low", "medium", "high"

Required JSON shape:
{{
  "claims": [
    {{
      "id": "short_stable_id",
      "text": "claim text",
      "classification": "fact|inference|assumption|speculation",
      "confidence": "low|medium|high",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ],
  "open_questions": [
    {{
      "id": "short_stable_id",
      "question": "question text",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ],
  "stale_assumptions": [
    {{
      "id": "short_stable_id",
      "text": "assumption text",
      "evidence": [{{ "id": "source-id-from-input" }}]
    }}
  ]
}}

Input:
{payload_json}"#
    ))
}

fn build_solution_critique_prompt(
    bundle: &RawContextBundle,
    context: &CompiledContext,
) -> AppResult<String> {
    let payload = json!({
        "raw_context": prompt_bundle_payload(bundle),
        "compiled_context": prompt_compiled_context_payload(context),
    });
    let payload_json = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::Validation(format!(
            "Failed to serialize solution critic prompt: {error}"
        ))
    })?;

    Ok(format!(
        r#"You are RalphX's solution critic.

Task:
- Give an honest evidence-backed critique of the target.
- Judge the accuracy of the compiled claims against the collected sources.
- Identify unsupported claims, contradictions, missing verification, stale assumptions, and implementation risks.
- Focus on correctness, feasibility, merge safety, and verification value. Ignore style nits unless they create a concrete implementation risk.
- Be strict: if evidence is absent or weak, mark the claim unclear or unsupported.
- Be source-bound: every evidence id must be copied exactly from the supplied source ids.
- Do not invent source ids, tests that were already run, or proof that is not present.
- Choose a safe next action that a human developer can take immediately.

Return strict JSON only. Do not wrap it in markdown.

Allowed enum values:
- verdict: "accept", "revise", "investigate", "reject"
- confidence: "low", "medium", "high"
- claim status: "supported", "unsupported", "contradicted", "unclear"
- recommendation status: "accept", "revise", "investigate", "reject"
- risk severity / verification priority: "critical", "high", "medium", "low"

Required JSON shape:
{{
  "verdict": "accept|revise|investigate|reject",
  "confidence": "low|medium|high",
  "claims": [
    {{
      "id": "short_stable_id",
      "claim": "claim under review",
      "status": "supported|unsupported|contradicted|unclear",
      "confidence": "low|medium|high",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "notes": "why this status is correct"
    }}
  ],
  "recommendations": [
    {{
      "id": "short_stable_id",
      "recommendation": "recommended change or decision",
      "status": "accept|revise|investigate|reject",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "rationale": "reasoning"
    }}
  ],
  "risks": [
    {{
      "id": "short_stable_id",
      "risk": "risk text",
      "severity": "critical|high|medium|low",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "mitigation": "mitigation text"
    }}
  ],
  "verification_plan": [
    {{
      "id": "short_stable_id",
      "requirement": "verification requirement",
      "priority": "critical|high|medium|low",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "suggested_test": "specific test or inspection"
    }}
  ],
  "safe_next_action": "one concise action"
}}

Input:
{payload_json}"#
    ))
}

fn build_solution_critique_repair_prompt(
    bundle: &RawContextBundle,
    context: &CompiledContext,
    invalid_json: &str,
    validation_error: &str,
) -> AppResult<String> {
    let payload = json!({
        "raw_context": prompt_bundle_payload(bundle),
        "compiled_context": prompt_compiled_context_payload(context),
        "validation_error": validation_error,
        "invalid_response": truncate_for_prompt(invalid_json, PROMPT_TARGET_LIMIT),
    });
    let payload_json = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::Validation(format!(
            "Failed to serialize solution critic repair prompt: {error}"
        ))
    })?;

    Ok(format!(
        r#"You are RalphX's solution critic.

Your previous response did not match the solution critique schema.

Repair task:
- Return one valid JSON object only.
- Give an honest evidence-backed critique of the target.
- Use the compiled context and collected sources below.
- Copy evidence ids exactly from allowed_source_ids.
- Do not invent source ids, tests that were already run, or proof that is not present.
- Do not return the context compiler schema. If the invalid response has root fields like claims/open_questions/stale_assumptions, treat it only as context and produce a solution critique object now.

Required root fields:
- verdict
- confidence
- claims
- recommendations
- risks
- verification_plan
- safe_next_action

Allowed enum values:
- verdict: "accept", "revise", "investigate", "reject"
- confidence: "low", "medium", "high"
- claim status: "supported", "unsupported", "contradicted", "unclear"
- recommendation status: "accept", "revise", "investigate", "reject"
- risk severity / verification priority: "critical", "high", "medium", "low"

Required JSON shape:
{{
  "verdict": "accept|revise|investigate|reject",
  "confidence": "low|medium|high",
  "claims": [
    {{
      "id": "short_stable_id",
      "claim": "claim under review",
      "status": "supported|unsupported|contradicted|unclear",
      "confidence": "low|medium|high",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "notes": "why this status is correct"
    }}
  ],
  "recommendations": [
    {{
      "id": "short_stable_id",
      "recommendation": "recommended change or decision",
      "status": "accept|revise|investigate|reject",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "rationale": "reasoning"
    }}
  ],
  "risks": [
    {{
      "id": "short_stable_id",
      "risk": "risk text",
      "severity": "critical|high|medium|low",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "mitigation": "mitigation text"
    }}
  ],
  "verification_plan": [
    {{
      "id": "short_stable_id",
      "requirement": "verification requirement",
      "priority": "critical|high|medium|low",
      "evidence": [{{ "id": "source-id-from-input" }}],
      "suggested_test": "specific test or inspection"
    }}
  ],
  "safe_next_action": "one concise action"
}}

Input:
{payload_json}"#
    ))
}

fn prompt_bundle_payload(bundle: &RawContextBundle) -> serde_json::Value {
    let sources = bundle
        .sources
        .iter()
        .map(|source| {
            json!({
                "source_type": source.source_type,
                "id": source.id,
                "label": source.label,
                "excerpt": source
                    .excerpt
                    .as_deref()
                    .map(|value| truncate_for_prompt(value, PROMPT_SOURCE_EXCERPT_LIMIT)),
                "created_at": source.created_at,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "session_id": bundle.session_id,
        "project_id": bundle.project_id,
        "target": bundle.target,
        "target_content": truncate_for_prompt(&bundle.target_content, PROMPT_TARGET_LIMIT),
        "sources": sources,
        "allowed_source_ids": bundle
            .sources
            .iter()
            .map(|source| source.id.as_str())
            .collect::<Vec<_>>(),
    })
}

fn prompt_compiled_context_payload(context: &CompiledContext) -> serde_json::Value {
    json!({
        "id": context.id,
        "target": context.target,
        "claims": context
            .claims
            .iter()
            .map(|claim| {
                json!({
                    "id": claim.id,
                    "text": claim.text,
                    "classification": claim.classification,
                    "confidence": claim.confidence,
                    "evidence_ids": claim
                        .evidence
                        .iter()
                        .map(|source| source.id.as_str())
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
        "open_questions": context
            .open_questions
            .iter()
            .map(|question| {
                json!({
                    "id": question.id,
                    "question": question.question,
                    "evidence_ids": question
                        .evidence
                        .iter()
                        .map(|source| source.id.as_str())
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
        "stale_assumptions": context
            .stale_assumptions
            .iter()
            .map(|assumption| {
                json!({
                    "id": assumption.id,
                    "text": assumption.text,
                    "evidence_ids": assumption
                        .evidence
                        .iter()
                        .map(|source| source.id.as_str())
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
        "generated_at": context.generated_at,
    })
}

fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let mut truncated = value.chars().take(limit).collect::<String>();
    truncated.push_str("\n[truncated for solution critic prompt]");
    truncated
}

fn extract_json_object(response: &str) -> AppResult<String> {
    let trimmed = strip_code_fence(response.trim());
    if serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .is_some_and(|value| value.is_object())
    {
        return Ok(trimmed.to_string());
    }

    balanced_json_object(trimmed).ok_or_else(|| {
        AppError::Validation(
            "Solution critic model response did not contain a JSON object".to_string(),
        )
    })
}

fn extract_agent_json_object(response: &str) -> AppResult<String> {
    let response_text = extract_stream_json_response_text(response)
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| response.trim().to_string());
    extract_json_object(&response_text)
}

fn extract_stream_json_response_text(response: &str) -> Option<String> {
    let mut saw_stream_event = false;
    let mut assistant_text = Vec::new();
    let mut delta_text = Vec::new();
    let mut result_text = Vec::new();

    for line in response.lines() {
        let candidate = line
            .trim()
            .strip_prefix("data:")
            .map(str::trim)
            .unwrap_or_else(|| line.trim());
        if candidate.is_empty() || candidate == "[DONE]" {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) else {
            continue;
        };
        let Some(event_type) = stream_event_type(&value) else {
            continue;
        };
        saw_stream_event = true;

        match event_type {
            "assistant" => assistant_text.extend(extract_assistant_text(&value)),
            "content_block_delta" => {
                if let Some(text) = value
                    .get("delta")
                    .and_then(|delta| delta.get("text"))
                    .and_then(|text| text.as_str())
                {
                    delta_text.push(text.to_string());
                }
            }
            "result" => {
                if value.get("is_error").and_then(|flag| flag.as_bool()) != Some(true) {
                    if let Some(text) = value.get("result").and_then(|result| result.as_str()) {
                        result_text.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if !saw_stream_event {
        return None;
    }

    if !assistant_text.is_empty() {
        return Some(assistant_text.join("\n"));
    }
    if !delta_text.is_empty() {
        return Some(delta_text.join(""));
    }
    Some(result_text.join("\n"))
}

fn stream_event_type(value: &serde_json::Value) -> Option<&str> {
    value
        .get("type")
        .and_then(|event_type| event_type.as_str())
        .or_else(|| {
            value
                .get("message")
                .and_then(|message| message.get("type"))
                .and_then(|event_type| event_type.as_str())
        })
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("type"))
                .and_then(|event_type| event_type.as_str())
        })
        .or_else(|| {
            value
                .get("event")
                .and_then(|event| event.get("type"))
                .and_then(|event_type| event_type.as_str())
        })
}

fn extract_assistant_text(value: &serde_json::Value) -> Vec<String> {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_array())
        .map(|content| {
            content
                .iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|value| value.as_str()) == Some("text") {
                        block
                            .get("text")
                            .and_then(|text| text.as_str())
                            .map(ToString::to_string)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn strip_code_fence(value: &str) -> &str {
    let Some(rest) = value.strip_prefix("```") else {
        return value;
    };
    let rest = rest.trim_start();
    let rest = rest
        .find('\n')
        .map(|line_end| &rest[line_end + 1..])
        .unwrap_or(rest);
    rest.rfind("```")
        .map(|end| rest[..end].trim())
        .unwrap_or(rest.trim())
}

fn balanced_json_object(value: &str) -> Option<String> {
    let mut start = None;
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in value.char_indices() {
        if start.is_none() {
            if character == '{' {
                start = Some(index);
                depth = 1;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let object_start = start?;
                    let object_end = index + character.len_utf8();
                    return Some(value[object_start..object_end].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::{
        AgentConfig, AgentOutput, AgentResponse, AgentResult, ClientCapabilities, ResponseChunk,
    };
    use futures::stream;
    use std::pin::Pin;
    use tokio::sync::Mutex;

    struct RecordingAgentClient {
        responses: Mutex<Vec<String>>,
        prompts: Mutex<Vec<String>>,
        capabilities: ClientCapabilities,
    }

    impl RecordingAgentClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Mutex::new(responses),
                prompts: Mutex::new(Vec::new()),
                capabilities: ClientCapabilities::mock(),
            }
        }

        async fn prompts(&self) -> Vec<String> {
            self.prompts.lock().await.clone()
        }
    }

    #[async_trait]
    impl AgenticClient for RecordingAgentClient {
        async fn spawn_agent(&self, _config: AgentConfig) -> AgentResult<AgentHandle> {
            unreachable!("solution critic generator uses send_prompt")
        }

        async fn stop_agent(&self, _handle: &AgentHandle) -> AgentResult<()> {
            Ok(())
        }

        async fn wait_for_completion(&self, _handle: &AgentHandle) -> AgentResult<AgentOutput> {
            Ok(AgentOutput::success(""))
        }

        async fn send_prompt(
            &self,
            _handle: &AgentHandle,
            prompt: &str,
        ) -> AgentResult<AgentResponse> {
            self.prompts.lock().await.push(prompt.to_string());
            let response = self.responses.lock().await.remove(0);
            Ok(AgentResponse::new(response))
        }

        fn stream_response(
            &self,
            _handle: &AgentHandle,
            _prompt: &str,
        ) -> Pin<Box<dyn futures::Stream<Item = AgentResult<ResponseChunk>> + Send>> {
            Box::pin(stream::empty())
        }

        fn capabilities(&self) -> &ClientCapabilities {
            &self.capabilities
        }

        async fn is_available(&self) -> AgentResult<bool> {
            Ok(true)
        }
    }

    fn raw_bundle() -> RawContextBundle {
        RawContextBundle {
            session_id: "session-1".to_string(),
            project_id: "project-1".to_string(),
            target: crate::domain::entities::ContextTargetRef {
                target_type: crate::domain::entities::ContextTargetType::PlanArtifact,
                id: "plan-1".to_string(),
                label: "Plan".to_string(),
            },
            target_content: "Implement the migration and prove it with a test.".to_string(),
            sources: vec![crate::domain::entities::ContextSourceRef {
                source_type: crate::domain::entities::ContextSourceType::PlanArtifact,
                id: "plan_artifact:plan-1".to_string(),
                label: "Plan".to_string(),
                excerpt: Some("Implement the migration.".to_string()),
                created_at: None,
            }],
        }
    }

    #[tokio::test]
    async fn agent_generator_extracts_json_from_model_response() {
        let response = r#"```json
{
  "claims": [
    {
      "id": "claim_migration",
      "text": "The plan requires a migration.",
      "classification": "fact",
      "confidence": "high",
      "evidence": [{ "id": "plan_artifact:plan-1" }]
    }
  ],
  "open_questions": [],
  "stale_assumptions": []
}
```"#;
        let client = Arc::new(RecordingAgentClient::new(vec![response.to_string()]));
        let generator = AgentSolutionCritiqueGenerator::new(client.clone());

        let json = generator
            .compile_context_candidate(&raw_bundle())
            .await
            .unwrap();

        assert!(json.starts_with('{'));
        assert!(json.contains("claim_migration"));
        let prompts = client.prompts().await;
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("Return strict JSON only"));
        assert!(prompts[0].contains("plan_artifact:plan-1"));
    }

    #[tokio::test]
    async fn agent_generator_extracts_json_from_claude_stream_json_response() {
        let compiler_json = r#"{
  "claims": [
    {
      "id": "claim_migration",
      "text": "The plan requires a migration.",
      "classification": "fact",
      "confidence": "high",
      "evidence": [{ "id": "plan_artifact:plan-1" }]
    }
  ],
  "open_questions": [],
  "stale_assumptions": []
}"#;
        let response = [
            json!({
                "type": "system",
                "subtype": "hook_started",
                "hook_id": "hook-1",
                "hook_name": "SessionStart:startup",
            })
            .to_string(),
            json!({
                "type": "assistant",
                "message": {
                    "content": [
                        {
                            "type": "text",
                            "text": compiler_json,
                        }
                    ],
                    "stop_reason": "end_turn",
                },
                "session_id": "claude-session-1",
            })
            .to_string(),
            json!({
                "type": "result",
                "result": compiler_json,
                "session_id": "claude-session-1",
                "is_error": false,
            })
            .to_string(),
        ]
        .join("\n");
        let client = Arc::new(RecordingAgentClient::new(vec![response]));
        let generator = AgentSolutionCritiqueGenerator::new(client);

        let json = generator
            .compile_context_candidate(&raw_bundle())
            .await
            .unwrap();

        assert!(json.contains("claim_migration"));
        assert!(!json.contains("hook_started"));
        serde_json::from_str::<CompiledContextCandidate>(&json).unwrap();
    }

    #[test]
    fn extract_agent_json_object_uses_stream_result_when_assistant_text_is_absent() {
        let critique_json = r#"{
  "verdict": "investigate",
  "confidence": "medium",
  "claims": [],
  "recommendations": [],
  "risks": [],
  "verification_plan": [],
  "safe_next_action": "Inspect the evidence."
}"#;
        let response = [
            json!({
                "type": "system",
                "subtype": "hook_started",
            })
            .to_string(),
            json!({
                "type": "result",
                "result": critique_json,
                "is_error": false,
            })
            .to_string(),
        ]
        .join("\n");

        let json = extract_agent_json_object(&response).unwrap();

        assert!(json.contains("\"verdict\""));
        assert!(!json.contains("hook_started"));
        serde_json::from_str::<SolutionCritiqueCandidate>(&json).unwrap();
    }

    #[test]
    fn extract_json_object_ignores_trailing_second_object() {
        let response = r#"{
  "claims": [],
  "open_questions": [],
  "stale_assumptions": []
}
{"extra": "trailing object"}"#;

        let json = extract_json_object(response).unwrap();

        assert!(json.contains("\"claims\""));
        assert!(!json.contains("trailing object"));
        serde_json::from_str::<CompiledContextCandidate>(&json).unwrap();
    }
}
