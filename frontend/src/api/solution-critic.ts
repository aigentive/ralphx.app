import { z } from "zod";
import { ApiVerificationGapSchema } from "./ideation.schemas";

const API_BASE = "http://localhost:3847/api";

const nullableString = z.string().nullable().optional();

export const ContextSourceRefSchema = z.object({
  source_type: z.string(),
  id: z.string(),
  label: z.string(),
  excerpt: nullableString,
  created_at: nullableString,
}).transform((value) => ({
  sourceType: value.source_type,
  id: value.id,
  label: value.label,
  ...(value.excerpt ? { excerpt: value.excerpt } : {}),
  ...(value.created_at ? { createdAt: value.created_at } : {}),
}));

export const ContextTargetRefSchema = z.object({
  target_type: z.string(),
  id: z.string(),
  label: z.string(),
}).transform((value) => ({
  targetType: value.target_type,
  id: value.id,
  label: value.label,
}));

const ContextClaimSchema = z.object({
  id: z.string(),
  text: z.string(),
  classification: z.string(),
  confidence: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
});

const ContextQuestionSchema = z.object({
  id: z.string(),
  question: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
});

const ContextAssumptionSchema = z.object({
  id: z.string(),
  text: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
});

export const CompiledContextSchema = z.object({
  id: z.string(),
  target: ContextTargetRefSchema,
  sources: z.array(ContextSourceRefSchema).optional().default([]),
  claims: z.array(ContextClaimSchema).optional().default([]),
  open_questions: z.array(ContextQuestionSchema).optional().default([]),
  stale_assumptions: z.array(ContextAssumptionSchema).optional().default([]),
  generated_at: z.string(),
}).transform((value) => ({
  id: value.id,
  target: value.target,
  sources: value.sources,
  claims: value.claims,
  openQuestions: value.open_questions,
  staleAssumptions: value.stale_assumptions,
  generatedAt: value.generated_at,
}));

const ClaimReviewSchema = z.object({
  id: z.string(),
  claim: z.string(),
  status: z.string(),
  confidence: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
  notes: nullableString,
});

const RecommendationReviewSchema = z.object({
  id: z.string(),
  recommendation: z.string(),
  status: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
  rationale: nullableString,
});

const RiskAssessmentSchema = z.object({
  id: z.string(),
  risk: z.string(),
  severity: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
  mitigation: nullableString,
});

const VerificationRequirementSchema = z.object({
  id: z.string(),
  requirement: z.string(),
  priority: z.string(),
  evidence: z.array(ContextSourceRefSchema).optional().default([]),
  suggested_test: nullableString,
});

export const SolutionCritiqueSchema = z.object({
  id: z.string(),
  artifact_id: z.string(),
  context_artifact_id: z.string(),
  verdict: z.string(),
  confidence: z.string(),
  claims: z.array(ClaimReviewSchema).optional().default([]),
  recommendations: z.array(RecommendationReviewSchema).optional().default([]),
  risks: z.array(RiskAssessmentSchema).optional().default([]),
  verification_plan: z.array(VerificationRequirementSchema).optional().default([]),
  safe_next_action: nullableString,
  generated_at: z.string(),
}).transform((value) => ({
  id: value.id,
  artifactId: value.artifact_id,
  contextArtifactId: value.context_artifact_id,
  verdict: value.verdict,
  confidence: value.confidence,
  claims: value.claims.map((claim) => ({
    ...claim,
    ...(claim.notes ? { notes: claim.notes } : {}),
  })),
  recommendations: value.recommendations.map((recommendation) => ({
    ...recommendation,
    ...(recommendation.rationale ? { rationale: recommendation.rationale } : {}),
  })),
  risks: value.risks.map((risk) => ({
    ...risk,
    ...(risk.mitigation ? { mitigation: risk.mitigation } : {}),
  })),
  verificationPlan: value.verification_plan.map((requirement) => ({
    ...requirement,
    ...(requirement.suggested_test ? { suggestedTest: requirement.suggested_test } : {}),
  })),
  ...(value.safe_next_action ? { safeNextAction: value.safe_next_action } : {}),
  generatedAt: value.generated_at,
}));

export const CompiledContextReadResponseSchema = z.object({
  artifact_id: z.string(),
  compiled_context: CompiledContextSchema,
}).transform((value) => ({
  artifactId: value.artifact_id,
  compiledContext: value.compiled_context,
}));

export const SolutionCritiqueReadResponseSchema = z.object({
  artifact_id: z.string(),
  solution_critique: SolutionCritiqueSchema,
  projected_gaps: z.array(ApiVerificationGapSchema).optional().default([]),
}).transform((value) => ({
  artifactId: value.artifact_id,
  solutionCritique: value.solution_critique,
  projectedGaps: value.projected_gaps,
}));

export type CompiledContextReadResponse = z.infer<typeof CompiledContextReadResponseSchema>;
export type SolutionCritiqueReadResponse = z.infer<typeof SolutionCritiqueReadResponseSchema>;

export type SolutionCritiqueTargetType =
  | "plan_artifact"
  | "artifact"
  | "chat_message"
  | "agent_run"
  | "task"
  | "task_execution"
  | "review_report";

export interface SolutionCritiqueTargetInput {
  targetType: SolutionCritiqueTargetType;
  id: string;
  label?: string;
}

export interface SourceLimitsInput {
  chatMessages?: number;
  taskProposals?: number;
  relatedArtifacts?: number;
  agentRuns?: number;
}

async function solutionCriticFetch<T>(
  url: string,
  init: RequestInit,
  schema: z.ZodType<T>,
  label: string
): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as Record<string, unknown>;
    throw new Error((body as { error?: string }).error ?? `${label}: ${response.status}`);
  }
  return schema.parse(await response.json());
}

function sourceLimitsToApi(sourceLimits?: SourceLimitsInput): Record<string, number> {
  if (!sourceLimits) return {};
  return {
    ...(sourceLimits.chatMessages !== undefined && { chat_messages: sourceLimits.chatMessages }),
    ...(sourceLimits.taskProposals !== undefined && { task_proposals: sourceLimits.taskProposals }),
    ...(sourceLimits.relatedArtifacts !== undefined && { related_artifacts: sourceLimits.relatedArtifacts }),
    ...(sourceLimits.agentRuns !== undefined && { agent_runs: sourceLimits.agentRuns }),
  };
}

function targetToApi(target: SolutionCritiqueTargetInput): Record<string, unknown> {
  return {
    target: {
      target_type: target.targetType,
      id: target.id,
      ...(target.label ? { label: target.label } : {}),
    },
  };
}

function targetPath(target: SolutionCritiqueTargetInput): string {
  return `${encodeURIComponent(target.targetType)}/${encodeURIComponent(target.id)}`;
}

export const solutionCriticApi = {
  getLatestCompiledContext: (sessionId: string): Promise<CompiledContextReadResponse | null> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/compiled-context`,
      {},
      CompiledContextReadResponseSchema.nullable(),
      "Failed to get latest compiled context"
    ),

  getLatestTargetCompiledContext: (
    sessionId: string,
    target: SolutionCritiqueTargetInput
  ): Promise<CompiledContextReadResponse | null> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/compiled-context/target/${targetPath(target)}`,
      {},
      CompiledContextReadResponseSchema.nullable(),
      "Failed to get latest target compiled context"
    ),

  getCompiledContext: (
    sessionId: string,
    artifactId: string
  ): Promise<CompiledContextReadResponse> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/compiled-context/${encodeURIComponent(artifactId)}`,
      {},
      CompiledContextReadResponseSchema,
      "Failed to get compiled context"
    ),

  compileContext: (
    sessionId: string,
    targetArtifactId: string,
    sourceLimits?: SourceLimitsInput
  ): Promise<CompiledContextReadResponse> =>
    solutionCriticApi.compileTargetContext(
      sessionId,
      { targetType: "plan_artifact", id: targetArtifactId },
      sourceLimits
    ),

  compileTargetContext: (
    sessionId: string,
    target: SolutionCritiqueTargetInput,
    sourceLimits?: SourceLimitsInput
  ): Promise<CompiledContextReadResponse> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/compiled-context`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...targetToApi(target),
          source_limits: sourceLimitsToApi(sourceLimits),
        }),
      },
      CompiledContextReadResponseSchema,
      "Failed to compile context"
    ),

  getLatestSolutionCritique: (sessionId: string): Promise<SolutionCritiqueReadResponse | null> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique`,
      {},
      SolutionCritiqueReadResponseSchema.nullable(),
      "Failed to get latest solution critique"
    ),

  getLatestTargetSolutionCritique: (
    sessionId: string,
    target: SolutionCritiqueTargetInput
  ): Promise<SolutionCritiqueReadResponse | null> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/target/${targetPath(target)}`,
      {},
      SolutionCritiqueReadResponseSchema.nullable(),
      "Failed to get latest target solution critique"
    ),

  getSolutionCritique: (
    sessionId: string,
    artifactId: string
  ): Promise<SolutionCritiqueReadResponse> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/${encodeURIComponent(artifactId)}`,
      {},
      SolutionCritiqueReadResponseSchema,
      "Failed to get solution critique"
    ),

  critiqueArtifact: (
    sessionId: string,
    targetArtifactId: string,
    compiledContextArtifactId: string
  ): Promise<SolutionCritiqueReadResponse> =>
    solutionCriticApi.critiqueTarget(
      sessionId,
      { targetType: "plan_artifact", id: targetArtifactId },
      compiledContextArtifactId
    ),

  critiqueTarget: (
    sessionId: string,
    target: SolutionCritiqueTargetInput,
    compiledContextArtifactId: string
  ): Promise<SolutionCritiqueReadResponse> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...targetToApi(target),
          compiled_context_artifact_id: compiledContextArtifactId,
        }),
      },
      SolutionCritiqueReadResponseSchema,
      "Failed to critique artifact"
    ),
} as const;
