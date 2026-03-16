import { describe, expect, it, vi } from "vitest";
import { render, fireEvent, act } from "@testing-library/react";
import { ReactFlowProvider, type Node } from "@xyflow/react";
import { useGraphSelectionController } from "./useGraphSelectionController";
import { useUiStore } from "@/stores/uiStore";
import { PLAN_GROUP_NODE_TYPE } from "../groups/PlanGroup";
import { getPlanGroupNodeId } from "../groups/groupTypes";
import type { PlanGroupInfo } from "@/api/task-graph.types";

const noop = () => undefined;

const EMPTY_STATUS_SUMMARY = {
  total: 0,
  completed: 0,
  blocked: 0,
  executing: 0,
  queued: 0,
  review: 0,
  merge: 0,
  ready: 0,
  failed: 0,
};

interface TestHarnessProps {
  containerId?: string;
  layoutNodes?: Node[];
  groupNodes?: Node[];
  planGroups?: PlanGroupInfo[];
  keyboardNavigationEnabled?: boolean;
  fitNode?: (node: Node, options?: { duration?: number; padding?: number; maxZoom?: number }) => void;
}

function TestHarness({
  containerId = "graph",
  layoutNodes = [],
  groupNodes = [
    {
      id: getPlanGroupNodeId("plan-1"),
      type: PLAN_GROUP_NODE_TYPE,
      position: { x: 0, y: 0 },
      data: {
        planArtifactId: "plan-1",
        sessionId: "session-1",
        sessionTitle: "Plan",
        taskIds: [],
        statusSummary: EMPTY_STATUS_SUMMARY,
        isCollapsed: false,
        width: 300,
        height: 120,
      },
    } as Node,
  ],
  planGroups = [
    {
      planArtifactId: "plan-1",
      sessionId: "session-1",
      sessionTitle: "Plan",
      taskIds: [],
      statusSummary: EMPTY_STATUS_SUMMARY,
    },
  ],
  keyboardNavigationEnabled = true,
  fitNode = noop,
}: TestHarnessProps) {
  const { containerRef, onKeyDown } = useGraphSelectionController({
    nodes: layoutNodes,
    edges: [],
    layoutNodes,
    groupNodes,
    planGroups,
    tierGroups: [],
    grouping: { byPlan: true, byTier: true, showUncategorized: true },
    collapsedPlanIds: new Set(),
    collapsedTierIds: new Set(),
    onToggleCollapse: vi.fn(),
    onToggleTierCollapse: vi.fn(),
    onToggleAllTiers: vi.fn(),
    centerOnPlanGroup: vi.fn(() => true),
    centerOnNode: vi.fn(() => true),
    centerOnNodeObject: noop,
    fitNode,
    fitViewDefault: noop,
    zoomBy: vi.fn(() => true),
    graphReady: true,
    graphError: null,
    isLoading: false,
    keyboardNavigationEnabled,
  });

  return (
    <div
      id={containerId}
      ref={containerRef}
      onKeyDown={onKeyDown}
    />
  );
}

describe("useGraphSelectionController", () => {
  it("selects first plan group on ArrowDown", () => {
    useUiStore.getState().clearGraphSelection();
    const { container } = render(
      <ReactFlowProvider>
        <TestHarness />
      </ReactFlowProvider>
    );

    fireEvent.keyDown(container.firstChild as HTMLElement, { key: "ArrowDown" });

    expect(useUiStore.getState().graphSelection).toEqual({
      kind: "planGroup",
      id: "plan-1",
    });
  });

  it("does not navigate by keyboard when keyboard navigation is disabled", () => {
    useUiStore.getState().clearGraphSelection();
    const { container } = render(
      <ReactFlowProvider>
        <TestHarness keyboardNavigationEnabled={false} />
      </ReactFlowProvider>
    );

    fireEvent.keyDown(container.firstChild as HTMLElement, { key: "ArrowDown" });

    expect(useUiStore.getState().graphSelection).toBeNull();
  });

  it("clears selection and recenters plan group on Escape", () => {
    useUiStore.getState().clearGraphSelection();
    const fitNode = vi.fn();
    const rafSpy = vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb: FrameRequestCallback) => {
      cb(0);
      return 0;
    });

    const { container } = render(
      <ReactFlowProvider>
        <TestHarness fitNode={fitNode} />
      </ReactFlowProvider>
    );

    act(() => {
      useUiStore.getState().setGraphSelection({ kind: "planGroup", id: "plan-1" });
    });

    fireEvent.keyDown(container.firstChild as HTMLElement, { key: "Escape" });

    expect(useUiStore.getState().graphSelection).toBeNull();
    expect(fitNode).toHaveBeenCalled();

    rafSpy.mockRestore();
  });

  describe("Backspace on task", () => {
    it("navigates up to plan group for a categorized task", () => {
      useUiStore.getState().clearGraphSelection();
      const taskNode: Node = {
        id: "task-1",
        type: "task",
        position: { x: 100, y: 100 },
        data: {},
      };

      const { container } = render(
        <ReactFlowProvider>
          <TestHarness
            layoutNodes={[taskNode]}
            planGroups={[
              {
                planArtifactId: "plan-1",
                sessionId: "session-1",
                sessionTitle: "Plan",
                taskIds: ["task-1"],
                statusSummary: EMPTY_STATUS_SUMMARY,
              },
            ]}
          />
        </ReactFlowProvider>
      );

      // Select the task first
      act(() => {
        useUiStore.getState().setGraphSelection({ kind: "task", id: "task-1" });
      });

      fireEvent.keyDown(container.firstChild as HTMLElement, { key: "Backspace" });

      expect(useUiStore.getState().graphSelection).toEqual({
        kind: "planGroup",
        id: "plan-1",
      });
    });

    it("does nothing for an uncategorized task (no navigation possible)", () => {
      useUiStore.getState().clearGraphSelection();
      const taskNode: Node = {
        id: "task-uncategorized",
        type: "task",
        position: { x: 100, y: 100 },
        data: {},
      };

      const { container } = render(
        <ReactFlowProvider>
          <TestHarness
            layoutNodes={[taskNode]}
            planGroups={[]}
            groupNodes={[]}
          />
        </ReactFlowProvider>
      );

      // Select the uncategorized task
      act(() => {
        useUiStore.getState().setGraphSelection({ kind: "task", id: "task-uncategorized" });
      });

      fireEvent.keyDown(container.firstChild as HTMLElement, { key: "Backspace" });

      // Selection should remain unchanged (no destructive action)
      expect(useUiStore.getState().graphSelection).toEqual({
        kind: "task",
        id: "task-uncategorized",
      });
    });
  });
});
