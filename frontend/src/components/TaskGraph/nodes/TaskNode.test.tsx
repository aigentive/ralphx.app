import React from "react";
import { ReactFlowProvider } from "@xyflow/react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TaskNode, type TaskNodeData } from "./TaskNode";

vi.mock("@/hooks/useTaskSteps", () => ({
  useStepProgress: vi.fn(() => ({ data: null })),
}));

function renderTaskNode(data: TaskNodeData) {
  const props = {
    id: data.taskId,
    type: "task",
    selected: false,
    data,
    isConnectable: true,
    positionAbsoluteX: 0,
    positionAbsoluteY: 0,
    dragging: false,
    zIndex: 0,
  } as React.ComponentProps<typeof TaskNode>;

  return render(
    <ReactFlowProvider>
      <TaskNode {...props} />
    </ReactFlowProvider>
  );
}

describe("TaskNode", () => {
  it("renders plan merge category and PR context", () => {
    renderTaskNode({
      label: "Merge plan into main",
      taskId: "task-123",
      internalStatus: "merged",
      priority: 2,
      isCriticalPath: false,
      description: "Auto-created merge task",
      category: "plan_merge",
      mergeTarget: "main",
      prNumber: 68,
      prStatus: "Open",
      planBranchStatus: "merged",
    });

    expect(screen.getByText("Merge -> main")).toBeInTheDocument();
    expect(screen.getByTestId("graph-pr-indicator")).toHaveTextContent("PR #68 merged");
    expect(screen.queryByText("plan_merge")).not.toBeInTheDocument();
  });
});
