import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useTaskGraphLayout, type PlanBranchNodeContext } from "./useTaskGraphLayout";
import type { TaskGraphNode } from "@/api/task-graph.types";

function makeNode(overrides: Partial<TaskGraphNode> = {}): TaskGraphNode {
  return {
    taskId: "merge-task",
    title: "Merge plan into main",
    description: "Auto-created merge task",
    category: "plan_merge",
    internalStatus: "merged",
    priority: 0,
    inDegree: 0,
    outDegree: 0,
    tier: 1,
    planArtifactId: "plan-1",
    sourceProposalId: null,
    executionPlanId: null,
    ...overrides,
  };
}

describe("useTaskGraphLayout", () => {
  it("hydrates plan merge node context from its plan artifact id", () => {
    const planBranchMap = new Map<string, PlanBranchNodeContext>([
      [
        "plan-1",
        {
          mergeTarget: "main",
          prNumber: 68,
          prStatus: "Merged",
          status: "merged",
        },
      ],
    ]);

    const { result } = renderHook(() =>
      useTaskGraphLayout(
        [makeNode()],
        [],
        [],
        [],
        { byPlan: false, byTier: false, showUncategorized: true },
        {},
        new Set(),
        new Set(),
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        planBranchMap,
      ),
    );

    expect(result.current.nodes[0]?.data).toMatchObject({
      mergeTarget: "main",
      prNumber: 68,
      prStatus: "Merged",
      planBranchStatus: "merged",
    });
  });

  it("hydrates plan merge node context from merge task id when plan artifact id is absent", () => {
    const planBranchMap = new Map<string, PlanBranchNodeContext>([
      [
        "merge-task",
        {
          mergeTarget: "main",
          prNumber: 68,
          prStatus: "Merged",
          status: "merged",
        },
      ],
    ]);

    const { result } = renderHook(() =>
      useTaskGraphLayout(
        [makeNode({ planArtifactId: null })],
        [],
        [],
        [],
        { byPlan: false, byTier: false, showUncategorized: true },
        {},
        new Set(),
        new Set(),
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        planBranchMap,
      ),
    );

    expect(result.current.nodes[0]?.data).toMatchObject({
      mergeTarget: "main",
      prNumber: 68,
      prStatus: "Merged",
      planBranchStatus: "merged",
    });
  });

  it("prefers merge task context over plan artifact context for merged branches", () => {
    const planBranchMap = new Map<string, PlanBranchNodeContext>([
      [
        "plan-1",
        {
          mergeTarget: "main",
          status: "active",
        },
      ],
      [
        "merge-task",
        {
          mergeTarget: "main",
          prNumber: 68,
          prStatus: "Open",
          status: "merged",
        },
      ],
    ]);

    const { result } = renderHook(() =>
      useTaskGraphLayout(
        [makeNode()],
        [],
        [],
        [],
        { byPlan: false, byTier: false, showUncategorized: true },
        {},
        new Set(),
        new Set(),
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        planBranchMap,
      ),
    );

    expect(result.current.nodes[0]?.data).toMatchObject({
      prNumber: 68,
      prStatus: "Open",
      planBranchStatus: "merged",
    });
  });
});
