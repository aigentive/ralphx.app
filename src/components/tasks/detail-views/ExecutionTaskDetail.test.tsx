/**
 * ExecutionTaskDetail component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ExecutionTaskDetail } from "./ExecutionTaskDetail";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(),
  useStepProgress: vi.fn(),
}));

vi.mock("@/hooks/useReviews", () => ({
  useTaskStateHistory: vi.fn(),
}));

vi.mock("@/api/review-issues", () => ({
  reviewIssuesApi: {
    getByTaskId: vi.fn().mockResolvedValue([]),
  },
}));

const mockStepList = vi.fn(({ taskId, editable }) => (
  <div data-testid="mock-step-list" data-task-id={taskId} data-editable={String(editable)} />
));

vi.mock("../StepList", () => ({
  StepList: (props: unknown) => mockStepList(props),
}));

import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";

const mockUseTaskSteps = vi.mocked(useTaskSteps);
const mockUseStepProgress = vi.mocked(useStepProgress);
const mockUseTaskStateHistory = vi.mocked(useTaskStateHistory);

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task Title",
    description: "Test task description",
    priority: 2,
    internalStatus: "executing",
    needsReviewPoint: false,
    sourceProposalId: null,
    planArtifactId: null,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: "2026-01-28T12:00:00+00:00",
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

describe("ExecutionTaskDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    mockUseStepProgress.mockReturnValue({
      data: {
        total: 0,
        completed: 0,
        inProgress: 0,
        pending: 0,
        failed: 0,
        percentComplete: 0,
      },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useStepProgress>);

    mockUseTaskStateHistory.mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
      isEmpty: true,
      latestEntry: null,
      refetch: vi.fn(),
    });
  });

  it("renders executing status banner and description", () => {
    const task = createTestTask({
      internalStatus: "executing",
      description: "Task description here",
    });
    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("execution-task-detail")).toBeInTheDocument();
    expect(screen.getByText("Executing Task")).toBeInTheDocument();
    expect(screen.getByText("Live")).toBeInTheDocument();
    expect(screen.getByText("Task description here")).toBeInTheDocument();
  });

  it("renders progress section when step progress exists", () => {
    const task = createTestTask();
    mockUseStepProgress.mockReturnValue({
      data: {
        total: 4,
        completed: 2,
        inProgress: 1,
        pending: 1,
        failed: 0,
        percentComplete: 50,
      },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useStepProgress>);

    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    const progressSection = screen.getByTestId("execution-progress-section");
    expect(progressSection).toBeInTheDocument();
    expect(screen.getByText("50%")).toBeInTheDocument();
    expect(progressSection.textContent).toContain("Step 2 of 4");
  });

  it("renders revision feedback section for re_executing tasks", () => {
    const task = createTestTask({ internalStatus: "re_executing" });
    const reviewNote: ReviewNoteResponse = {
      id: "note-1",
      task_id: task.id,
      reviewer: "ai",
      outcome: "changes_requested",
      notes: "Missing error handling in auth.ts",
      created_at: "2026-01-28T11:00:00+00:00",
    };

    mockUseTaskStateHistory.mockReturnValue({
      data: [reviewNote],
      isLoading: false,
      error: null,
      isEmpty: false,
      latestEntry: reviewNote,
      refetch: vi.fn(),
    });

    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("revision-feedback-banner")).toBeInTheDocument();
    expect(screen.getByText("Feedback Being Addressed")).toBeInTheDocument();
    expect(screen.getByText("Addressing")).toBeInTheDocument();
    expect(screen.getByText("Missing error handling in auth.ts")).toBeInTheDocument();
  });

  it("shows revision feedback loading state while history is loading", () => {
    const task = createTestTask({ internalStatus: "re_executing" });
    mockUseTaskStateHistory.mockReturnValue({
      data: [],
      isLoading: true,
      error: null,
      isEmpty: true,
      latestEntry: null,
      refetch: vi.fn(),
    });

    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("revision-feedback-banner")).toBeInTheDocument();
    expect(screen.queryByText("AI Feedback")).not.toBeInTheDocument();
  });

  it("renders steps section when task has steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [
        {
          id: "step-1",
          taskId: task.id,
          title: "Step 1",
          status: "completed",
          order: 0,
          createdAt: "2026-01-28T12:00:00+00:00",
          updatedAt: "2026-01-28T12:00:00+00:00",
        },
      ],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("execution-steps-section")).toBeInTheDocument();
    expect(screen.getByTestId("mock-step-list")).toBeInTheDocument();
  });

  it("shows loading state while fetching steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("execution-steps-loading")).toBeInTheDocument();
  });
});
