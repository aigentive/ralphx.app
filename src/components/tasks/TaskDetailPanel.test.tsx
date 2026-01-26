/**
 * TaskDetailPanel.test.tsx - Unit tests for TaskDetailPanel component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskDetailPanel } from "./TaskDetailPanel";
import type { Task } from "@/types/task";

// Mock hooks
vi.mock("@/hooks/useReviews", () => ({
  useReviewsByTaskId: vi.fn(() => ({
    data: [],
    isLoading: false,
  })),
}));

vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(() => ({
    data: [],
    isLoading: false,
  })),
}));

// Mock child components
vi.mock("./StateHistoryTimeline", () => ({
  StateHistoryTimeline: ({ taskId }: { taskId: string }) => (
    <div data-testid="state-history-timeline" data-task-id={taskId}>
      State History Timeline
    </div>
  ),
}));

vi.mock("./TaskContextPanel", () => ({
  TaskContextPanel: ({ taskId }: { taskId: string }) => (
    <div data-testid="task-context-panel" data-task-id={taskId}>
      Task Context Panel
    </div>
  ),
}));

vi.mock("./StepList", () => ({
  StepList: ({ taskId, editable }: { taskId: string; editable: boolean }) => (
    <div data-testid="step-list" data-task-id={taskId} data-editable={editable}>
      Step List
    </div>
  ),
}));

const createMockTask = (overrides?: Partial<Task>): Task => ({
  id: "task-123",
  projectId: "project-1",
  category: "feature",
  title: "Test Task",
  description: "Test description",
  priority: 2,
  internalStatus: "ready",
  needsReviewPoint: false,
  sourceProposalId: null,
  planArtifactId: null,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-01T00:00:00Z",
  startedAt: null,
  completedAt: null,
  archivedAt: null,
  ...overrides,
});

function renderWithQueryClient(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
  );
}

describe("TaskDetailPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders task title and priority", () => {
    const task = createMockTask();
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("task-detail-title")).toHaveTextContent("Test Task");
    expect(screen.getByTestId("task-detail-priority")).toHaveTextContent("P2");
  });

  it("renders task category and status", () => {
    const task = createMockTask();
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("task-detail-category")).toHaveTextContent("feature");
    expect(screen.getByTestId("task-detail-status")).toHaveTextContent("Ready");
  });

  it("renders task description", () => {
    const task = createMockTask({ description: "Custom description" });
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("task-detail-description")).toHaveTextContent(
      "Custom description"
    );
  });

  it("shows no description message when description is null", () => {
    const task = createMockTask({ description: null });
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByText("No description provided")).toBeInTheDocument();
  });

  it("renders history section by default", () => {
    const task = createMockTask();
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("task-detail-history-section")).toBeInTheDocument();
    expect(screen.getByTestId("state-history-timeline")).toBeInTheDocument();
  });

  it("hides history section when showHistory is false", () => {
    const task = createMockTask();
    renderWithQueryClient(<TaskDetailPanel task={task} showHistory={false} />);

    expect(
      screen.queryByTestId("task-detail-history-section")
    ).not.toBeInTheDocument();
  });

  it("does not show context button when task has no context", () => {
    const task = createMockTask();
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.queryByTestId("view-context-button")).not.toBeInTheDocument();
  });

  it("shows context button when task has sourceProposalId", () => {
    const task = createMockTask({ sourceProposalId: "proposal-1" });
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("view-context-button")).toBeInTheDocument();
  });

  it("shows context button when task has planArtifactId", () => {
    const task = createMockTask({ planArtifactId: "artifact-1" });
    renderWithQueryClient(<TaskDetailPanel task={task} />);

    expect(screen.getByTestId("view-context-button")).toBeInTheDocument();
  });

  it("renders task context panel when showContext is true", () => {
    const task = createMockTask({ sourceProposalId: "proposal-1" });
    renderWithQueryClient(<TaskDetailPanel task={task} showContext={true} />);

    expect(screen.getByTestId("task-context-section")).toBeInTheDocument();
    expect(screen.getByTestId("task-context-panel")).toBeInTheDocument();
  });
});
