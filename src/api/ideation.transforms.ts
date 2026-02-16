// Transform functions for converting snake_case API responses to camelCase frontend types

import { z } from "zod";
import type {
  IdeationSettings,
  IdeationSettingsResponse,
} from "../types/ideation-config";
import type {
  IdeationSessionStatus,
  TaskProposal,
  Priority,
  Complexity,
  ProposalStatus,
} from "../types/ideation";
import type {
  IdeationSessionResponse,
  TaskProposalResponse,
  ChatMessageResponse,
  SessionWithDataResponse,
  PriorityAssessmentResponse,
  DependencyGraphResponse,
  ApplyProposalsResultResponse,
  CreateChildSessionResponse,
  ParentSessionContextResponse,
} from "./ideation.types";
import {
  IdeationSessionResponseSchema,
  TaskProposalResponseSchema,
  ChatMessageResponseSchema,
  SessionWithDataResponseSchema,
  PriorityAssessmentResponseSchema,
  DependencyGraphResponseSchema,
  ApplyProposalsResultResponseSchema,
  CreateChildSessionResponseSchema,
  ParentSessionContextResponseSchema,
} from "./ideation.schemas";

export function transformSession(raw: z.infer<typeof IdeationSessionResponseSchema>): IdeationSessionResponse {
  return {
    id: raw.id,
    projectId: raw.project_id,
    title: raw.title,
    status: raw.status as IdeationSessionStatus,
    planArtifactId: raw.plan_artifact_id,
    seedTaskId: raw.seed_task_id ?? null,
    parentSessionId: raw.parent_session_id,
    teamMode: raw.team_mode ?? null,
    teamConfig: raw.team_config ? {
      maxTeammates: raw.team_config.max_teammates,
      modelCeiling: raw.team_config.model_ceiling,
      ...(raw.team_config.budget_limit != null && { budgetLimit: raw.team_config.budget_limit }),
      compositionMode: raw.team_config.composition_mode as "dynamic" | "constrained",
    } : null,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    archivedAt: raw.archived_at,
    convertedAt: raw.converted_at,
  };
}

export function transformProposal(raw: z.infer<typeof TaskProposalResponseSchema>): TaskProposalResponse {
  return {
    id: raw.id,
    sessionId: raw.session_id,
    title: raw.title,
    description: raw.description,
    category: raw.category,
    steps: raw.steps,
    acceptanceCriteria: raw.acceptance_criteria,
    suggestedPriority: raw.suggested_priority,
    priorityScore: raw.priority_score,
    priorityReason: raw.priority_reason,
    estimatedComplexity: raw.estimated_complexity,
    userPriority: raw.user_priority,
    userModified: raw.user_modified,
    status: raw.status,
    createdTaskId: raw.created_task_id,
    planArtifactId: raw.plan_artifact_id,
    planVersionAtCreation: raw.plan_version_at_creation,
    sortOrder: raw.sort_order,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

/**
 * Convert TaskProposalResponse to TaskProposal (store type)
 *
 * This function properly types the enum fields that come as strings from the API
 * to the branded enum types expected by the store.
 */
export function toTaskProposal(response: TaskProposalResponse): TaskProposal {
  return {
    id: response.id,
    sessionId: response.sessionId,
    title: response.title,
    description: response.description,
    category: response.category,
    steps: response.steps,
    acceptanceCriteria: response.acceptanceCriteria,
    suggestedPriority: response.suggestedPriority as Priority,
    priorityScore: response.priorityScore,
    priorityReason: response.priorityReason,
    estimatedComplexity: response.estimatedComplexity as Complexity,
    userPriority: response.userPriority as Priority | null,
    userModified: response.userModified,
    status: response.status as ProposalStatus,
    createdTaskId: response.createdTaskId,
    planArtifactId: response.planArtifactId,
    planVersionAtCreation: response.planVersionAtCreation,
    sortOrder: response.sortOrder,
    createdAt: response.createdAt,
    updatedAt: response.updatedAt,
  };
}

export function transformMessage(raw: z.infer<typeof ChatMessageResponseSchema>): ChatMessageResponse {
  return {
    id: raw.id,
    sessionId: raw.session_id,
    projectId: raw.project_id,
    taskId: raw.task_id,
    role: raw.role,
    content: raw.content,
    metadata: raw.metadata,
    toolCalls: raw.tool_calls,
    parentMessageId: raw.parent_message_id,
    createdAt: raw.created_at,
  };
}

export function transformSessionWithData(raw: z.infer<typeof SessionWithDataResponseSchema>): SessionWithDataResponse {
  return {
    session: transformSession(raw.session),
    proposals: raw.proposals.map(transformProposal),
    messages: raw.messages.map(transformMessage),
  };
}

export function transformPriorityAssessment(raw: z.infer<typeof PriorityAssessmentResponseSchema>): PriorityAssessmentResponse {
  return {
    proposalId: raw.proposal_id,
    priority: raw.priority,
    score: raw.score,
    reason: raw.reason,
  };
}

export function transformDependencyGraph(raw: z.infer<typeof DependencyGraphResponseSchema>): DependencyGraphResponse {
  return {
    nodes: raw.nodes.map((n) => ({
      proposalId: n.proposal_id,
      title: n.title,
      inDegree: n.in_degree,
      outDegree: n.out_degree,
    })),
    edges: raw.edges.map((e) => ({
      from: e.from,
      to: e.to,
      reason: e.reason ?? null,
    })),
    criticalPath: raw.critical_path,
    hasCycles: raw.has_cycles,
    cycles: raw.cycles,
  };
}

export function transformApplyResult(raw: z.infer<typeof ApplyProposalsResultResponseSchema>): ApplyProposalsResultResponse {
  return {
    createdTaskIds: raw.created_task_ids,
    dependenciesCreated: raw.dependencies_created,
    warnings: raw.warnings,
    sessionConverted: raw.session_converted,
  };
}

export function transformIdeationSettings(raw: IdeationSettingsResponse): IdeationSettings {
  return {
    planMode: raw.plan_mode as IdeationSettings["planMode"],
    requirePlanApproval: raw.require_plan_approval,
    suggestPlansForComplex: raw.suggest_plans_for_complex,
    autoLinkProposals: raw.auto_link_proposals,
  };
}

export function transformParentSessionContext(
  raw: z.infer<typeof ParentSessionContextResponseSchema>
): ParentSessionContextResponse {
  return {
    parentSession: {
      id: raw.parent_session.id,
      title: raw.parent_session.title,
      status: raw.parent_session.status,
    },
    planContent: raw.plan_content,
    proposals: raw.proposals.map((p) => ({
      id: p.id,
      title: p.title,
      category: p.category,
      priority: p.priority,
      status: p.status,
      acceptanceCriteria: p.acceptance_criteria,
    })),
  };
}

export function transformCreateChildSession(
  raw: z.infer<typeof CreateChildSessionResponseSchema>
): CreateChildSessionResponse {
  return {
    sessionId: raw.session_id,
    parentSessionId: raw.parent_session_id,
    title: raw.title,
    status: raw.status,
    createdAt: raw.created_at,
    parentContext: raw.parent_context ? transformParentSessionContext(raw.parent_context) : undefined,
  };
}
