import { describe, expect, it, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { useUiStore } from "@/stores/uiStore";
import { GraphSplitLayout } from "./GraphSplitLayout";

const { mockUseTaskChatAvailability } = vi.hoisted(() => ({
  mockUseTaskChatAvailability: vi.fn(() => false),
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: ({ projectId }: { projectId: string }) => (
    <div data-testid="integrated-chat-panel">Task chat for {projectId}</div>
  ),
}));

vi.mock("@/components/tasks/TaskDetailOverlay", () => ({
  TaskDetailOverlay: ({
    projectId,
    constrainContent,
  }: {
    projectId: string;
    constrainContent?: boolean;
  }) => (
    <div data-testid="task-detail-overlay" data-constrained={String(Boolean(constrainContent))}>
      Task detail for {projectId}
    </div>
  ),
}));

vi.mock("@/components/tasks/TaskCreationOverlay", () => ({
  TaskCreationOverlay: () => <div data-testid="task-creation-overlay" />,
}));

vi.mock("@/hooks/useTaskChatAvailability", () => ({
  useTaskChatAvailability: mockUseTaskChatAvailability,
}));

function renderGraphSplitLayout() {
  return render(
    <GraphSplitLayout
      projectId="project-1"
      rightPanelMode="split"
      timelineContent={<div data-testid="timeline-content">Timeline</div>}
    >
      <div data-testid="graph-canvas">Graph</div>
    </GraphSplitLayout>
  );
}

describe("GraphSplitLayout", () => {
  beforeEach(() => {
    mockUseTaskChatAvailability.mockReturnValue(false);
    useUiStore.setState({
      selectedTaskId: null,
      taskCreationContext: null,
      taskHistoryState: null,
    });
    localStorage.clear();
  });

  it("shows the timeline instead of an empty chat pane for selected tasks without chat", () => {
    useUiStore.setState({ selectedTaskId: "task-1" });

    renderGraphSplitLayout();

    expect(screen.getByTestId("task-detail-overlay")).toBeInTheDocument();
    expect(screen.getByTestId("task-detail-overlay")).toHaveAttribute("data-constrained", "true");
    expect(screen.getByTestId("timeline-content")).toBeInTheDocument();
    expect(screen.queryByTestId("integrated-chat-panel")).not.toBeInTheDocument();
  });

  it("shows selected-task chat when an agent chat is available", () => {
    mockUseTaskChatAvailability.mockReturnValue(true);
    useUiStore.setState({ selectedTaskId: "task-1" });

    renderGraphSplitLayout();

    expect(screen.getByTestId("task-detail-overlay")).toHaveAttribute("data-constrained", "false");
    expect(screen.getByTestId("integrated-chat-panel")).toHaveTextContent("Task chat for project-1");
    expect(screen.queryByTestId("timeline-content")).not.toBeInTheDocument();
  });
});
