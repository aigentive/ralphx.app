/**
 * Mock Task Graph API
 *
 * Mirrors src/api/task-graph.ts with mock implementations.
 */

import { z } from "zod";
import { getStore } from "./store";
import {
  TaskDependencyGraphResponseSchema,
  TimelineEventsResponseSchema,
} from "@/api/task-graph";
import type { InternalStatus } from "@/types/status";

type TaskDependencyGraphResponseRaw = z.infer<typeof TaskDependencyGraphResponseSchema>;
type TimelineEventsResponseRaw = z.infer<typeof TimelineEventsResponseSchema>;

function createEmptyStatusSummary() {
  return {
    backlog: 0,
    ready: 0,
    blocked: 0,
    executing: 0,
    qa: 0,
    review: 0,
    merge: 0,
    completed: 0,
    terminal: 0,
  };
}

function bucketStatus(status: InternalStatus): keyof ReturnType<typeof createEmptyStatusSummary> {
  switch (status) {
    case "backlog":
      return "backlog";
    case "ready":
      return "ready";
    case "blocked":
    case "paused":
      return "blocked";
    case "executing":
    case "re_executing":
      return "executing";
    case "qa_refining":
    case "qa_testing":
    case "qa_passed":
    case "qa_failed":
      return "qa";
    case "pending_review":
    case "reviewing":
    case "review_passed":
    case "escalated":
    case "revision_needed":
      return "review";
    case "pending_merge":
    case "merging":
    case "merge_conflict":
      return "merge";
    case "approved":
    case "merged":
      return "completed";
    case "failed":
    case "cancelled":
    case "stopped":
      return "terminal";
  }
}

function formatPlanTitle(planArtifactId: string, index: number): string {
  const match = planArtifactId.match(/(\d+)$/);
  const suffix = match?.[1] ?? String(index + 1);
  return `Plan ${suffix}`;
}

function buildGraphResponse(projectId: string): TaskDependencyGraphResponseRaw {
  const store = getStore();
  const tasks = Array.from(store.tasks.values()).filter(
    (task) => task.projectId === projectId
  );
  const planGroupMap = new Map<
    string,
    {
      sessionId: string;
      sessionTitle: string | null;
      taskIds: string[];
      statusSummary: ReturnType<typeof createEmptyStatusSummary>;
    }
  >();

  tasks.forEach((task) => {
    if (!task.planArtifactId) return;
    const existing = planGroupMap.get(task.planArtifactId);
    const bucket = bucketStatus(task.internalStatus);
    if (existing) {
      existing.taskIds.push(task.id);
      existing.statusSummary[bucket] += 1;
    } else {
      const statusSummary = createEmptyStatusSummary();
      statusSummary[bucket] = 1;
      planGroupMap.set(task.planArtifactId, {
        sessionId: task.planArtifactId,
        sessionTitle: null,
        taskIds: [task.id],
        statusSummary,
      });
    }
  });

  const planGroups = Array.from(planGroupMap.entries()).map(
    ([planArtifactId, group], index) => ({
      plan_artifact_id: planArtifactId,
      session_id: group.sessionId,
      session_title: group.sessionTitle ?? formatPlanTitle(planArtifactId, index),
      task_ids: group.taskIds,
      status_summary: group.statusSummary,
    })
  );

  return {
    nodes: tasks.map((task) => ({
      task_id: task.id,
      title: task.title,
      description: task.description ?? null,
      category: task.category,
      internal_status: task.internalStatus,
      priority: task.priority,
      in_degree: 0,
      out_degree: 0,
      tier: 0,
      plan_artifact_id: task.planArtifactId ?? null,
      source_proposal_id: task.sourceProposalId ?? null,
    })),
    edges: [],
    plan_groups: planGroups,
    critical_path: [],
    has_cycles: false,
  };
}

export const mockTaskGraphApi = {
  getDependencyGraph: async (projectId: string): Promise<TaskDependencyGraphResponseRaw> =>
    buildGraphResponse(projectId),

  getTimelineEvents: async (
    _projectId: string,
    _limit: number = 50,
    _offset: number = 0
  ): Promise<TimelineEventsResponseRaw> => ({
    events: [],
    total: 0,
    has_more: false,
  }),
} as const;
