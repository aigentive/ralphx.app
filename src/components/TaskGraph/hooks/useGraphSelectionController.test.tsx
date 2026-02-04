import { describe, expect, it, vi } from "vitest";
import { render, fireEvent } from "@testing-library/react";
import { ReactFlowProvider, type Node } from "@xyflow/react";
import { useGraphSelectionController } from "./useGraphSelectionController";
import { useUiStore } from "@/stores/uiStore";
import { PLAN_GROUP_NODE_TYPE } from "../groups/PlanGroup";
import { getPlanGroupNodeId } from "../groups/groupTypes";

const noop = () => undefined;

function TestHarness({ containerId = "graph" }: { containerId?: string }) {
  const controller = useGraphSelectionController({
    nodes: [],
    edges: [],
    layoutNodes: [],
    groupNodes: [
      {
        id: getPlanGroupNodeId("plan-1"),
        type: PLAN_GROUP_NODE_TYPE,
        position: { x: 0, y: 0 },
        data: {
          planArtifactId: "plan-1",
          sessionId: "session-1",
          sessionTitle: "Plan",
          taskIds: [],
          statusSummary: {
            total: 0,
            completed: 0,
            blocked: 0,
            executing: 0,
            queued: 0,
            review: 0,
            merge: 0,
            ready: 0,
            failed: 0,
          },
          isCollapsed: false,
          width: 300,
          height: 120,
        },
      } as Node,
    ],
    planGroups: [
      {
        planArtifactId: "plan-1",
        sessionId: "session-1",
        sessionTitle: "Plan",
        taskIds: [],
        statusSummary: {
          total: 0,
          completed: 0,
          blocked: 0,
          executing: 0,
          queued: 0,
          review: 0,
          merge: 0,
          ready: 0,
          failed: 0,
        },
      },
    ],
    tierGroups: [],
    grouping: { byPlan: true, byTier: true, showUncategorized: true },
    collapsedPlanIds: new Set(),
    collapsedTierIds: new Set(),
    onToggleCollapse: vi.fn(),
    onToggleTierCollapse: vi.fn(),
    onToggleAllTiers: vi.fn(),
    centerOnPlanGroup: vi.fn(() => true),
    fitNodeInView: vi.fn(() => true),
    fitNode: noop,
    centerOnNode: vi.fn(() => true),
    centerOnNodeObject: noop,
    fitViewDefault: noop,
    zoomBy: vi.fn(() => true),
    graphReady: true,
    graphError: null,
    isLoading: false,
  });

  return (
    <div
      id={containerId}
      ref={controller.containerRef}
      onKeyDown={controller.onKeyDown}
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
});
