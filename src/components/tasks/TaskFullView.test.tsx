/**
 * TaskFullView.test.tsx - Tests for TaskFullView component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskFullView } from "./TaskFullView";
import type { Task } from "@/types/task";

// Mock dependencies
vi.mock("@/hooks/useTasks", () => ({
  useTasks: vi.fn(() => ({
    data: [
      {
        id: "task-1",
        projectId: "project-1",
        title: "Test Task",
        description: "Test description",
        category: "feature",
        priority: 2,
        internalStatus: "executing",
        needsReviewPoint: false,
        sourceProposalId: null,
        planArtifactId: null,
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-01T00:00:00Z",
        startedAt: null,
        completedAt: null,
        archivedAt: null,
      } satisfies Task,
    ],
    isLoading: false,
    error: null,
  })),
  taskKeys: {
    all: ["tasks"],
    lists: () => ["tasks", "list"],
    list: (projectId: string) => ["tasks", "list", projectId],
    details: () => ["tasks", "detail"],
    detail: (taskId: string) => ["tasks", "detail", taskId],
  },
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: vi.fn(() => ({
    currentProjectId: "project-1",
  })),
}));

vi.mock("./TaskDetailPanel", () => ({
  TaskDetailPanel: ({ task }: { task: Task }) => (
    <div data-testid="task-detail-panel">Task Detail: {task.title}</div>
  ),
}));

vi.mock("./TaskChatPanel", () => ({
  TaskChatPanel: ({
    taskId,
    contextType,
  }: {
    taskId: string;
    contextType: string;
  }) => (
    <div data-testid="task-chat-panel">
      Chat: {taskId} ({contextType})
    </div>
  ),
}));

describe("TaskFullView", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
  });

  const renderTaskFullView = (taskId = "task-1") => {
    const onClose = vi.fn();
    const result = render(
      <QueryClientProvider client={queryClient}>
        <TaskFullView taskId={taskId} onClose={onClose} />
      </QueryClientProvider>
    );
    return { ...result, onClose };
  };

  it("renders task fullview", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview")).toBeInTheDocument();
  });

  it("displays task title in header", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-title")).toHaveTextContent(
      "Test Task"
    );
  });

  it("displays priority badge", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-priority")).toHaveTextContent(
      "P2"
    );
  });

  it("displays status badge", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-status")).toHaveTextContent(
      "Executing"
    );
  });

  it("renders left panel with TaskDetailPanel", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-detail-panel")).toBeInTheDocument();
    expect(screen.getByTestId("task-detail-panel")).toHaveTextContent(
      "Task Detail: Test Task"
    );
  });

  it("renders right panel with TaskChatPanel", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-chat-panel")).toBeInTheDocument();
    expect(screen.getByTestId("task-chat-panel")).toHaveTextContent(
      "Chat: task-1 (task_execution)"
    );
  });

  it("renders drag handle between panels", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-drag-handle")).toBeInTheDocument();
  });

  it("shows execution controls footer when task is executing", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-footer")).toBeInTheDocument();
    expect(screen.getByTestId("task-fullview-pause-button")).toBeInTheDocument();
    expect(screen.getByTestId("task-fullview-stop-button")).toBeInTheDocument();
  });

  it("calls onClose when back button is clicked", () => {
    const { onClose } = renderTaskFullView();
    fireEvent.click(screen.getByTestId("task-fullview-back-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when close button is clicked", () => {
    const { onClose } = renderTaskFullView();
    fireEvent.click(screen.getByTestId("task-fullview-close-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when Escape key is pressed", () => {
    const { onClose } = renderTaskFullView();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("renders action buttons in header", () => {
    renderTaskFullView();
    expect(screen.getByTestId("task-fullview-edit-button")).toBeInTheDocument();
    expect(
      screen.getByTestId("task-fullview-archive-button")
    ).toBeInTheDocument();
  });

  it("shows loading state when task is not found", () => {
    const onClose = vi.fn();

    // Render with a task ID that doesn't exist in the mock data
    render(
      <QueryClientProvider client={queryClient}>
        <TaskFullView taskId="non-existent-task" onClose={onClose} />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("task-fullview-loading")).toBeInTheDocument();
    expect(screen.getByText("Loading task...")).toBeInTheDocument();
  });
});
