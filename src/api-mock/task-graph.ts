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

type TaskDependencyGraphResponseRaw = z.infer<typeof TaskDependencyGraphResponseSchema>;
type TimelineEventsResponseRaw = z.infer<typeof TimelineEventsResponseSchema>;

function buildGraphResponse(projectId: string): TaskDependencyGraphResponseRaw {
  const store = getStore();
  const tasks = Array.from(store.tasks.values()).filter(
    (task) => task.projectId === projectId
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
    plan_groups: [],
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
