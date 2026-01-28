/**
 * ExecutionTaskDetail component tests
 *
 * Tests for the execution task detail view used for executing and re_executing states.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ExecutionTaskDetail } from "./ExecutionTaskDetail";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

// Mock hooks
vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(),
  useStepProgress: vi.fn(),
}));

vi.mock("@/hooks/useReviews", () => ({
  useTaskStateHistory: vi.fn(),
}));

import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";

const mockUseTaskSteps = vi.mocked(useTaskSteps);
const mockUseStepProgress = vi.mocked(useStepProgress);
const mockUseTaskStateHistory = vi.mocked(useTaskStateHistory);

// Helper to create test task
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

// Test wrapper with QueryClient
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
    // Default mock: no steps, no progress, no history
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

  describe("rendering", () => {
    it("renders live indicator badge for executing state", () => {
      const task = createTestTask({ internalStatus: "executing" });
      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-live-badge")).toBeInTheDocument();
      expect(screen.getByText(/Live/i)).toBeInTheDocument();
    });

    it("renders task title", () => {
      const task = createTestTask({ title: "My Executing Task" });
      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-task-title")).toHaveTextContent(
        "My Executing Task"
      );
    });

    it("renders description section", () => {
      const task = createTestTask({ description: "Task description here" });
      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-task-description")).toHaveTextContent(
        "Task description here"
      );
    });
  });

  describe("progress bar", () => {
    it("renders progress bar with percentage", () => {
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

      expect(screen.getByTestId("execution-progress-bar")).toBeInTheDocument();
      expect(screen.getByTestId("execution-progress-text")).toHaveTextContent("50%");
    });

    it("shows step count in progress section", () => {
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

      expect(screen.getByTestId("execution-step-count")).toHaveTextContent("2 of 4");
    });
  });

  describe("re_executing state", () => {
    it("renders revision feedback banner when re_executing", () => {
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
      expect(screen.getByText(/Addressing Review Feedback/i)).toBeInTheDocument();
    });

    it("does not show revision banner for regular executing state", () => {
      const task = createTestTask({ internalStatus: "executing" });
      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(
        screen.queryByTestId("revision-feedback-banner")
      ).not.toBeInTheDocument();
    });
  });

  describe("steps section", () => {
    it("renders StepList when task has steps", () => {
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
          {
            id: "step-2",
            taskId: task.id,
            title: "Step 2",
            status: "in_progress",
            order: 1,
            createdAt: "2026-01-28T12:00:00+00:00",
            updatedAt: "2026-01-28T12:00:00+00:00",
          },
        ],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);

      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-steps-section")).toBeInTheDocument();
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

  describe("loading states", () => {
    it("shows loading for steps", () => {
      const task = createTestTask();
      mockUseTaskSteps.mockReturnValue({
        data: undefined,
        isLoading: true,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);

      render(<ExecutionTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-steps-loading")).toBeInTheDocument();
    });

    it("shows loading for revision context when re_executing", () => {
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

      expect(
        screen.getByTestId("revision-feedback-loading")
      ).toBeInTheDocument();
    });
  });
});
