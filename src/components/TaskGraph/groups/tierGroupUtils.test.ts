import { describe, it, expect } from "vitest";
import type { PlanGroupInfo, TaskGraphNode } from "@/api/task-graph.types";
import {
  buildTierGroups,
  getTierGroupId,
} from "./tierGroupUtils";

const STATUS_SUMMARY = {
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

function makeNode(overrides: Partial<TaskGraphNode>): TaskGraphNode {
  return {
    taskId: overrides.taskId ?? "task",
    title: overrides.title ?? "Task",
    description: overrides.description ?? null,
    category: overrides.category ?? "general",
    internalStatus: overrides.internalStatus ?? "backlog",
    priority: overrides.priority ?? 0,
    inDegree: overrides.inDegree ?? 0,
    outDegree: overrides.outDegree ?? 0,
    tier: overrides.tier ?? 0,
    planArtifactId: overrides.planArtifactId ?? null,
    sourceProposalId: overrides.sourceProposalId ?? null,
  };
}

describe("buildTierGroups", () => {
  it("returns no tier groups when a plan has only one tier", () => {
    const nodes = [
      makeNode({ taskId: "t1", tier: 0, planArtifactId: "p1" }),
      makeNode({ taskId: "t2", tier: 0, planArtifactId: "p1" }),
    ];
    const planGroups: PlanGroupInfo[] = [
      {
        planArtifactId: "p1",
        sessionId: "s1",
        sessionTitle: "Plan 1",
        taskIds: ["t1", "t2"],
        statusSummary: STATUS_SUMMARY,
      },
    ];

    expect(buildTierGroups(nodes, planGroups)).toEqual([]);
  });

  it("creates tier groups per plan when multiple tiers exist", () => {
    const nodes = [
      makeNode({ taskId: "t1", tier: 0, planArtifactId: "p1" }),
      makeNode({ taskId: "t2", tier: 1, planArtifactId: "p1" }),
      makeNode({ taskId: "t3", tier: 1, planArtifactId: "p1" }),
    ];
    const planGroups: PlanGroupInfo[] = [
      {
        planArtifactId: "p1",
        sessionId: "s1",
        sessionTitle: "Plan 1",
        taskIds: ["t1", "t2", "t3"],
        statusSummary: STATUS_SUMMARY,
      },
    ];

    const groups = buildTierGroups(nodes, planGroups);
    expect(groups).toHaveLength(2);
    expect(groups[0]).toEqual({
      id: getTierGroupId("p1", 0),
      planArtifactId: "p1",
      tier: 0,
      taskIds: ["t1"],
    });
    expect(groups[1]).toEqual({
      id: getTierGroupId("p1", 1),
      planArtifactId: "p1",
      tier: 1,
      taskIds: ["t2", "t3"],
    });
  });

  it("does not create tier groups for uncategorized tasks even when they span multiple tiers", () => {
    const nodes = [
      makeNode({ taskId: "t1", tier: 0 }),
      makeNode({ taskId: "t2", tier: 1 }),
      makeNode({ taskId: "t3", tier: 2 }),
    ];

    const groups = buildTierGroups(nodes, []);
    expect(groups).toEqual([]);
  });

  it("does not create tier groups for uncategorized tasks when includeUngrouped is true", () => {
    const nodes = [
      makeNode({ taskId: "t1", tier: 0 }),
      makeNode({ taskId: "t2", tier: 2 }),
    ];

    const groups = buildTierGroups(nodes, [], { includeUngrouped: true });
    expect(groups).toEqual([]);
  });
});
