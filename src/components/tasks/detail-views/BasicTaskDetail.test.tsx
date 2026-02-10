/**
 * BasicTaskDetail component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BasicTaskDetail } from "./BasicTaskDetail";
import type { Task } from "@/types/task";

vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(),
}));

const mockStepList = vi.fn(({ taskId, editable, hideCompletionNotes }) => (
  <div
    data-testid="mock-step-list"
    data-task-id={taskId}
    data-editable={String(editable)}
    data-hide-completion-notes={String(hideCompletionNotes)}
  />
));

vi.mock("../StepList", () => ({
  StepList: (props: unknown) => mockStepList(props),
}));

import { useTaskSteps } from "@/hooks/useTaskSteps";
const mockUseTaskSteps = vi.mocked(useTaskSteps);

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task Title",
    description: "Test task description",
    priority: 2,
    internalStatus: "ready",
    needsReviewPoint: false,
    sourceProposalId: null,
    planArtifactId: null,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    ...overrides,
  };
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("BasicTaskDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);
  });

  it("renders container and description content", () => {
    const task = createTestTask({ description: "Task description here" });
    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("basic-task-detail")).toBeInTheDocument();
    expect(screen.getByText("Description")).toBeInTheDocument();
    expect(screen.getByText("Task description here")).toBeInTheDocument();
  });

  it("renders placeholder when description is null", () => {
    const task = createTestTask({ description: null });
    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByText(/no description provided/i)).toBeInTheDocument();
  });

  it("shows loading state while fetching steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });
    expect(screen.getByTestId("basic-task-steps-loading")).toBeInTheDocument();
  });

  it("renders StepList section when task has steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [
        {
          id: "step-1",
          taskId: task.id,
          title: "Step 1",
          status: "pending",
          order: 0,
          createdAt: "2026-01-28T12:00:00+00:00",
          updatedAt: "2026-01-28T12:00:00+00:00",
        },
      ],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("basic-task-steps-section")).toBeInTheDocument();
    expect(screen.getByTestId("mock-step-list")).toBeInTheDocument();
    expect(screen.getByTestId("mock-step-list")).toHaveAttribute(
      "data-hide-completion-notes",
      "false"
    );
  });

  it("passes historical mode to StepList", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [
        {
          id: "step-1",
          taskId: task.id,
          title: "Step 1",
          status: "pending",
          order: 0,
          createdAt: "2026-01-28T12:00:00+00:00",
          updatedAt: "2026-01-28T12:00:00+00:00",
        },
      ],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });
    expect(screen.getByTestId("mock-step-list")).toHaveAttribute(
      "data-hide-completion-notes",
      "true"
    );
  });

  it("shows empty steps state when no steps exist", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });
    expect(screen.getByText("No steps defined yet")).toBeInTheDocument();
  });
});

