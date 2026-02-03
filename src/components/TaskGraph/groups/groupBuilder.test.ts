import { describe, it, expect } from "vitest";
import type { Node } from "@xyflow/react";
import type { PlanGroupInfo, TaskGraphNode } from "@/api/task-graph.types";
import {
  buildPlanGroupNodes,
  buildTierGroupNodes,
  COLLAPSED_GROUP_HEIGHT,
  COLLAPSED_GROUP_WIDTH,
  getGroupNodeId,
} from "./groupBuilder";
import { UNGROUPED_PLAN_ID, type TierGroupInfo } from "./tierGroupUtils";

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

function makeTaskNode(id: string, position = { x: 0, y: 0 }): Node {
  return {
    id,
    type: "task",
    position,
    data: {},
  } as Node;
}

function makeGraphNode(overrides: Partial<TaskGraphNode>): TaskGraphNode {
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

describe("buildPlanGroupNodes", () => {
  it("creates collapsed plan group nodes at placeholder positions", () => {
    const planGroups: PlanGroupInfo[] = [
      {
        planArtifactId: "plan-1",
        sessionId: "session-1",
        sessionTitle: "Plan 1",
        taskIds: ["task-1"],
        statusSummary: STATUS_SUMMARY,
      },
    ];
    const positions = new Map([["__group_position_plan-1__", { x: 40, y: 60 }]]);
    const nodes = buildPlanGroupNodes({
      taskNodes: [makeTaskNode("task-1")],
      planGroups,
      collapsedPlanIds: new Set(["plan-1"]),
      tierGroups: [],
      collapsedTierIds: new Set(),
      positions,
      graphNodes: [],
      nodeWidth: 180,
      nodeHeight: 60,
      includeUncategorized: true,
    });

    expect(nodes).toHaveLength(1);
    expect(nodes[0]?.id).toBe(getGroupNodeId("plan", "plan-1"));
    expect(nodes[0]?.position).toEqual({ x: 40, y: 60 });
    expect(nodes[0]?.data.isCollapsed).toBe(true);
    expect(nodes[0]?.data.width).toBe(COLLAPSED_GROUP_WIDTH);
    expect(nodes[0]?.data.height).toBe(COLLAPSED_GROUP_HEIGHT);
  });

  it("creates an uncategorized group when tasks are ungrouped", () => {
    const nodes = buildPlanGroupNodes({
      taskNodes: [makeTaskNode("task-1")],
      planGroups: [],
      collapsedPlanIds: new Set(),
      tierGroups: [],
      collapsedTierIds: new Set(),
      positions: new Map([["__group_position___ungrouped__", { x: 0, y: 0 }]]),
      graphNodes: [makeGraphNode({ taskId: "task-1", internalStatus: "approved" })],
      nodeWidth: 180,
      nodeHeight: 60,
      includeUncategorized: true,
    });

    const uncategorized = nodes.find((node) => node.data.planArtifactId === UNGROUPED_PLAN_ID);
    expect(uncategorized).toBeTruthy();
    expect(uncategorized?.id).toBe(getGroupNodeId("plan", UNGROUPED_PLAN_ID));
    expect(uncategorized?.data.statusSummary.merge).toBe(1);
  });
});

describe("buildTierGroupNodes", () => {
  it("skips tier groups when parent plan is collapsed", () => {
    const tierGroups: TierGroupInfo[] = [
      {
        id: "tier-plan-1-0",
        planArtifactId: "plan-1",
        tier: 0,
        taskIds: ["task-1"],
      },
    ];

    const nodes = buildTierGroupNodes({
      taskNodes: [makeTaskNode("task-1")],
      tierGroups,
      collapsedTierIds: new Set(),
      collapsedPlanIds: new Set(["plan-1"]),
      planGroupBounds: new Map(),
      positions: new Map(),
      nodeWidth: 180,
      nodeHeight: 60,
    });

    expect(nodes).toHaveLength(0);
  });
});
