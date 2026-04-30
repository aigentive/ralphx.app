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

const SolutionCritiqueGapActionSchema = z.object({
  id: z.string(),
  session_id: z.string(),
  project_id: z.string(),
  target_type: z.string(),
  target_id: z.string(),
  critique_artifact_id: z.string(),
  context_artifact_id: z.string(),
  gap_id: z.string(),
  gap_fingerprint: z.string(),
  action: z.enum(["promoted", "deferred", "covered", "reopened"]),
  note: nullableString,
  actor_kind: z.string(),
  verification_generation: z.number().nullable().optional(),
  promoted_round: z.number().nullable().optional(),
  created_at: z.string(),
}).transform((value) => ({
  id: value.id,
  sessionId: value.session_id,
  projectId: value.project_id,
  targetType: value.target_type,
  targetId: value.target_id,
  critiqueArtifactId: value.critique_artifact_id,
  contextArtifactId: value.context_artifact_id,
  gapId: value.gap_id,
  gapFingerprint: value.gap_fingerprint,
  action: value.action,
  ...(value.note ? { note: value.note } : {}),
  actorKind: value.actor_kind,
  ...(value.verification_generation !== undefined && value.verification_generation !== null
    ? { verificationGeneration: value.verification_generation }
    : {}),
  ...(value.promoted_round !== undefined && value.promoted_round !== null
    ? { promotedRound: value.promoted_round }
    : {}),
  createdAt: value.created_at,
}));

const SolutionCritiqueGapActionSummarySchema = z.object({
  gap_id: z.string(),
  gap_fingerprint: z.string(),
  action: z.enum(["promoted", "deferred", "covered", "reopened"]),
  note: nullableString,
  verification_generation: z.number().nullable().optional(),
  created_at: z.string(),
}).transform((value) => ({
  gapId: value.gap_id,
  gapFingerprint: value.gap_fingerprint,
  action: value.action,
  ...(value.note ? { note: value.note } : {}),
  ...(value.verification_generation !== undefined && value.verification_generation !== null
    ? { verificationGeneration: value.verification_generation }
    : {}),
  createdAt: value.created_at,
}));

export const ProjectedCritiqueGapSchema = z.object({
  id: z.string(),
  critique_artifact_id: z.string(),
  context_artifact_id: z.string(),
  origin: z.object({
    kind: z.enum(["claim", "risk", "verification"]),
    item_id: z.string(),
  }),
  fingerprint: z.string(),
  status: z.enum(["open", "promoted", "deferred", "covered"]),
  verification_gap: ApiVerificationGapSchema,
  latest_action: SolutionCritiqueGapActionSchema.nullable().optional(),
}).transform((value) => ({
  id: value.id,
  critiqueArtifactId: value.critique_artifact_id,
  contextArtifactId: value.context_artifact_id,
  origin: {
    kind: value.origin.kind,
    itemId: value.origin.item_id,
  },
  fingerprint: value.fingerprint,
  status: value.status,
  verificationGap: value.verification_gap,
  ...(value.latest_action ? { latestAction: value.latest_action } : {}),
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
  projected_gap_items: z.array(ProjectedCritiqueGapSchema).optional().default([]),
}).transform((value) => ({
  artifactId: value.artifact_id,
  solutionCritique: value.solution_critique,
  projectedGaps: value.projected_gaps,
  projectedGapItems: value.projected_gap_items,
}));

export const ProjectedCritiqueGapActionResponseSchema = z.object({
  gap: ProjectedCritiqueGapSchema,
  action: SolutionCritiqueGapActionSchema,
  verification_updated: z.boolean(),
  verification_generation: z.number().nullable().optional(),
}).transform((value) => ({
  gap: value.gap,
  action: value.action,
  verificationUpdated: value.verification_updated,
  ...(value.verification_generation !== undefined && value.verification_generation !== null
    ? { verificationGeneration: value.verification_generation }
    : {}),
}));

export const CompiledContextHistoryItemSchema = z.object({
  artifact_id: z.string(),
  target: ContextTargetRefSchema,
  generated_at: z.string(),
  source_count: z.number(),
  claim_count: z.number(),
  open_question_count: z.number(),
  stale_assumption_count: z.number(),
}).transform((value) => ({
  artifactId: value.artifact_id,
  target: value.target,
  generatedAt: value.generated_at,
  sourceCount: value.source_count,
  claimCount: value.claim_count,
  openQuestionCount: value.open_question_count,
  staleAssumptionCount: value.stale_assumption_count,
}));

export const SolutionCritiqueHistoryItemSchema = z.object({
  artifact_id: z.string(),
  context_artifact_id: z.string(),
  target: ContextTargetRefSchema,
  verdict: z.string(),
  confidence: z.string(),
  generated_at: z.string(),
  source_count: z.number(),
  claim_count: z.number(),
  risk_count: z.number(),
  projected_gap_count: z.number(),
  stale: z.boolean(),
  latest_gap_actions: z.array(SolutionCritiqueGapActionSummarySchema).optional().default([]),
}).transform((value) => ({
  artifactId: value.artifact_id,
  contextArtifactId: value.context_artifact_id,
  target: value.target,
  verdict: value.verdict,
  confidence: value.confidence,
  generatedAt: value.generated_at,
  sourceCount: value.source_count,
  claimCount: value.claim_count,
  riskCount: value.risk_count,
  projectedGapCount: value.projected_gap_count,
  stale: value.stale,
  latestGapActions: value.latest_gap_actions,
}));

const SolutionCritiqueTargetRollupItemSchema = z.object({
  target: ContextTargetRefSchema,
  artifact_id: z.string(),
  context_artifact_id: z.string(),
  verdict: z.string(),
  confidence: z.string(),
  generated_at: z.string(),
  stale: z.boolean(),
  risk_count: z.number(),
  projected_gap_count: z.number(),
  promoted_gap_count: z.number(),
  deferred_gap_count: z.number(),
  covered_gap_count: z.number(),
}).transform((value) => ({
  target: value.target,
  artifactId: value.artifact_id,
  contextArtifactId: value.context_artifact_id,
  verdict: value.verdict,
  confidence: value.confidence,
  generatedAt: value.generated_at,
  stale: value.stale,
  riskCount: value.risk_count,
  projectedGapCount: value.projected_gap_count,
  promotedGapCount: value.promoted_gap_count,
  deferredGapCount: value.deferred_gap_count,
  coveredGapCount: value.covered_gap_count,
}));

export const SolutionCritiqueSessionRollupSchema = z.object({
  session_id: z.string(),
  generated_at: z.string(),
  target_count: z.number(),
  critique_count: z.number(),
  worst_verdict: z.string().nullable().optional(),
  highest_risk: z.string().nullable().optional(),
  stale_count: z.number(),
  promoted_gap_count: z.number(),
  deferred_gap_count: z.number(),
  covered_gap_count: z.number(),
  targets: z.array(SolutionCritiqueTargetRollupItemSchema).optional().default([]),
}).transform((value) => ({
  sessionId: value.session_id,
  generatedAt: value.generated_at,
  targetCount: value.target_count,
  critiqueCount: value.critique_count,
  ...(value.worst_verdict ? { worstVerdict: value.worst_verdict } : {}),
  ...(value.highest_risk ? { highestRisk: value.highest_risk } : {}),
  staleCount: value.stale_count,
  promotedGapCount: value.promoted_gap_count,
  deferredGapCount: value.deferred_gap_count,
  coveredGapCount: value.covered_gap_count,
  targets: value.targets,
}));

export type CompiledContextReadResponse = z.infer<typeof CompiledContextReadResponseSchema>;
export type SolutionCritiqueReadResponse = z.infer<typeof SolutionCritiqueReadResponseSchema>;
export type ProjectedCritiqueGap = z.infer<typeof ProjectedCritiqueGapSchema>;
export type ProjectedCritiqueGapActionResponse = z.infer<typeof ProjectedCritiqueGapActionResponseSchema>;
export type CompiledContextHistoryItem = z.infer<typeof CompiledContextHistoryItemSchema>;
export type SolutionCritiqueHistoryItem = z.infer<typeof SolutionCritiqueHistoryItemSchema>;
export type SolutionCritiqueSessionRollup = z.infer<typeof SolutionCritiqueSessionRollupSchema>;

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

type SolutionCritiqueTargetKeyInput = Pick<SolutionCritiqueTargetInput, "targetType" | "id">;

export const solutionCriticQueryKeys = {
  session: (sessionId: string | null | undefined) => ["solutionCritic", sessionId ?? "none"] as const,
  targetContext: (
    sessionId: string | null | undefined,
    target: SolutionCritiqueTargetKeyInput
  ) => ["solutionCritic", sessionId ?? "none", "targetContext", target.targetType, target.id] as const,
  targetCritique: (
    sessionId: string | null | undefined,
    target: SolutionCritiqueTargetKeyInput
  ) => ["solutionCritic", sessionId ?? "none", "target", target.targetType, target.id] as const,
  projectedGaps: (
    sessionId: string | null | undefined,
    critiqueArtifactId: string | null | undefined
  ) => ["solutionCritic", sessionId ?? "none", "projectedGaps", critiqueArtifactId ?? "none"] as const,
  targetContextHistory: (
    sessionId: string | null | undefined,
    target: SolutionCritiqueTargetKeyInput
  ) => ["solutionCritic", sessionId ?? "none", "targetContextHistory", target.targetType, target.id] as const,
  targetCritiqueHistory: (
    sessionId: string | null | undefined,
    target: SolutionCritiqueTargetKeyInput
  ) => ["solutionCritic", sessionId ?? "none", "targetCritiqueHistory", target.targetType, target.id] as const,
  rollup: (sessionId: string | null | undefined) =>
    ["solutionCritic", sessionId ?? "none", "rollup"] as const,
} as const;

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

  getProjectedCritiqueGaps: (
    sessionId: string,
    critiqueArtifactId: string
  ): Promise<ProjectedCritiqueGap[]> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/${encodeURIComponent(critiqueArtifactId)}/projected-gaps`,
      {},
      z.array(ProjectedCritiqueGapSchema),
      "Failed to get projected critique gaps"
    ),

  getCompiledContextHistoryForTarget: (
    sessionId: string,
    target: SolutionCritiqueTargetInput
  ): Promise<CompiledContextHistoryItem[]> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/compiled-context/target/${targetPath(target)}/history`,
      {},
      z.array(CompiledContextHistoryItemSchema),
      "Failed to get compiled context history"
    ),

  getSolutionCritiqueHistoryForTarget: (
    sessionId: string,
    target: SolutionCritiqueTargetInput
  ): Promise<SolutionCritiqueHistoryItem[]> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/target/${targetPath(target)}/history`,
      {},
      z.array(SolutionCritiqueHistoryItemSchema),
      "Failed to get solution critique history"
    ),

  getSolutionCritiqueRollup: (sessionId: string): Promise<SolutionCritiqueSessionRollup> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/rollup`,
      {},
      SolutionCritiqueSessionRollupSchema,
      "Failed to get solution critique rollup"
    ),

  applyProjectedGapAction: (
    sessionId: string,
    critiqueArtifactId: string,
    gapId: string,
    action: "promoted" | "deferred" | "covered" | "reopened",
    note?: string
  ): Promise<ProjectedCritiqueGapActionResponse> =>
    solutionCriticFetch(
      `${API_BASE}/ideation/sessions/${encodeURIComponent(sessionId)}/solution-critique/${encodeURIComponent(critiqueArtifactId)}/projected-gaps/${encodeURIComponent(gapId)}/actions`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action,
          ...(note ? { note } : {}),
        }),
      },
      ProjectedCritiqueGapActionResponseSchema,
      "Failed to apply projected critique gap action"
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
