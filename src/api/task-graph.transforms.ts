// Transform functions for converting snake_case task graph API responses to camelCase frontend types

import { z } from "zod";
import {
  TaskGraphNodeSchema,
  TaskGraphEdgeSchema,
  StatusSummarySchema,
  PlanGroupInfoSchema,
  TaskDependencyGraphResponseSchema,
} from "./task-graph.schemas";
import type {
  TaskGraphNode,
  TaskGraphEdge,
  StatusSummary,
  PlanGroupInfo,
  TaskDependencyGraphResponse,
} from "./task-graph.types";

/**
 * Transform TaskGraphNodeSchema (snake_case) → TaskGraphNode (camelCase)
 */
export function transformTaskGraphNode(
  raw: z.infer<typeof TaskGraphNodeSchema>
): TaskGraphNode {
  return {
    taskId: raw.task_id,
    title: raw.title,
    internalStatus: raw.internal_status,
    priority: raw.priority,
    inDegree: raw.in_degree,
    outDegree: raw.out_degree,
    tier: raw.tier,
    planArtifactId: raw.plan_artifact_id,
    sourceProposalId: raw.source_proposal_id,
  };
}

/**
 * Transform TaskGraphEdgeSchema (snake_case) → TaskGraphEdge (camelCase)
 */
export function transformTaskGraphEdge(
  raw: z.infer<typeof TaskGraphEdgeSchema>
): TaskGraphEdge {
  return {
    source: raw.source,
    target: raw.target,
    isCriticalPath: raw.is_critical_path,
  };
}

/**
 * Transform StatusSummarySchema → StatusSummary
 * (No case conversion needed, all lowercase)
 */
export function transformStatusSummary(
  raw: z.infer<typeof StatusSummarySchema>
): StatusSummary {
  return {
    backlog: raw.backlog,
    ready: raw.ready,
    blocked: raw.blocked,
    executing: raw.executing,
    qa: raw.qa,
    review: raw.review,
    merge: raw.merge,
    completed: raw.completed,
    terminal: raw.terminal,
  };
}

/**
 * Transform PlanGroupInfoSchema (snake_case) → PlanGroupInfo (camelCase)
 */
export function transformPlanGroupInfo(
  raw: z.infer<typeof PlanGroupInfoSchema>
): PlanGroupInfo {
  return {
    planArtifactId: raw.plan_artifact_id,
    sessionId: raw.session_id,
    sessionTitle: raw.session_title,
    taskIds: raw.task_ids,
    statusSummary: transformStatusSummary(raw.status_summary),
  };
}

/**
 * Transform TaskDependencyGraphResponseSchema → TaskDependencyGraphResponse
 */
export function transformTaskDependencyGraphResponse(
  raw: z.infer<typeof TaskDependencyGraphResponseSchema>
): TaskDependencyGraphResponse {
  return {
    nodes: raw.nodes.map(transformTaskGraphNode),
    edges: raw.edges.map(transformTaskGraphEdge),
    planGroups: raw.plan_groups.map(transformPlanGroupInfo),
    criticalPath: raw.critical_path,
    hasCycles: raw.has_cycles,
  };
}
