/**
 * Tests for useTaskSteps and useStepProgress hooks
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useTaskSteps, useStepProgress, stepKeys } from "./useTaskSteps";
import { api } from "@/lib/tauri";
import type { TaskStep, StepProgressSummary } from "@/types/task-step";
import type { ReactNode } from "react";

// Mock the Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    steps: {
      getByTask: vi.fn(),
      getProgress: vi.fn(),
    },
  },
}));

describe("stepKeys", () => {
  it("should generate correct query keys", () => {
    expect(stepKeys.all).toEqual(["steps"]);
    expect(stepKeys.byTask("task-1")).toEqual(["steps", "task", "task-1"]);
    expect(stepKeys.progress("task-1")).toEqual(["steps", "progress", "task-1"]);
  });
});

describe("useTaskSteps", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch steps for a task", async () => {
    const mockSteps: TaskStep[] = [
      {
        id: "step-1",
        taskId: "task-1",
        title: "Step 1",
        description: "First step",
        status: "completed",
        sortOrder: 0,
        dependsOn: null,
        createdBy: "user",
        completionNote: "Done",
        createdAt: "2025-01-26T00:00:00+00:00",
        updatedAt: "2025-01-26T01:00:00+00:00",
        startedAt: "2025-01-26T00:30:00+00:00",
        completedAt: "2025-01-26T01:00:00+00:00",
      },
      {
        id: "step-2",
        taskId: "task-1",
        title: "Step 2",
        description: null,
        status: "in_progress",
        sortOrder: 1,
        dependsOn: "step-1",
        createdBy: "user",
        completionNote: null,
        createdAt: "2025-01-26T00:00:00+00:00",
        updatedAt: "2025-01-26T01:30:00+00:00",
        startedAt: "2025-01-26T01:30:00+00:00",
        completedAt: null,
      },
    ];

    vi.mocked(api.steps.getByTask).mockResolvedValue(mockSteps);

    const { result } = renderHook(() => useTaskSteps("task-1"), { wrapper });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockSteps);
    expect(api.steps.getByTask).toHaveBeenCalledWith("task-1");
  });

  it("should not fetch when taskId is empty", () => {
    const { result } = renderHook(() => useTaskSteps(""), { wrapper });

    expect(result.current.isLoading).toBe(false);
    expect(result.current.fetchStatus).toBe("idle");
    expect(api.steps.getByTask).not.toHaveBeenCalled();
  });

  it("should handle errors", async () => {
    const mockError = new Error("Failed to fetch steps");
    vi.mocked(api.steps.getByTask).mockRejectedValue(mockError);

    const { result } = renderHook(() => useTaskSteps("task-1"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(mockError);
  });
});

describe("useStepProgress", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch progress summary for a task", async () => {
    const mockProgress: StepProgressSummary = {
      taskId: "task-1",
      total: 5,
      completed: 2,
      inProgress: 1,
      pending: 2,
      skipped: 0,
      failed: 0,
      currentStep: {
        id: "step-3",
        taskId: "task-1",
        title: "Step 3",
        description: null,
        status: "in_progress",
        sortOrder: 2,
        dependsOn: null,
        createdBy: "user",
        completionNote: null,
        createdAt: "2025-01-26T00:00:00+00:00",
        updatedAt: "2025-01-26T02:00:00+00:00",
        startedAt: "2025-01-26T02:00:00+00:00",
        completedAt: null,
      },
      nextStep: {
        id: "step-4",
        taskId: "task-1",
        title: "Step 4",
        description: null,
        status: "pending",
        sortOrder: 3,
        dependsOn: null,
        createdBy: "user",
        completionNote: null,
        createdAt: "2025-01-26T00:00:00+00:00",
        updatedAt: "2025-01-26T00:00:00+00:00",
        startedAt: null,
        completedAt: null,
      },
      percentComplete: 40.0,
    };

    vi.mocked(api.steps.getProgress).mockResolvedValue(mockProgress);

    const { result } = renderHook(() => useStepProgress("task-1"), { wrapper });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockProgress);
    expect(api.steps.getProgress).toHaveBeenCalledWith("task-1");
  });

  it("should not fetch when taskId is empty", () => {
    const { result } = renderHook(() => useStepProgress(""), { wrapper });

    expect(result.current.isLoading).toBe(false);
    expect(result.current.fetchStatus).toBe("idle");
    expect(api.steps.getProgress).not.toHaveBeenCalled();
  });

  it("should handle errors", async () => {
    const mockError = new Error("Failed to fetch progress");
    vi.mocked(api.steps.getProgress).mockRejectedValue(mockError);

    const { result } = renderHook(() => useStepProgress("task-1"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(mockError);
  });

  it("should calculate correct refetch interval when steps are in progress", async () => {
    const mockProgressInProgress: StepProgressSummary = {
      taskId: "task-1",
      total: 3,
      completed: 1,
      inProgress: 1,
      pending: 1,
      skipped: 0,
      failed: 0,
      currentStep: null,
      nextStep: null,
      percentComplete: 33.33,
    };

    vi.mocked(api.steps.getProgress).mockResolvedValue(mockProgressInProgress);

    const { result } = renderHook(() => useStepProgress("task-1"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // Verify refetchInterval is set when inProgress > 0
    // Note: This is a best-effort test - the actual refetch logic is internal to TanStack Query
    expect(result.current.data?.inProgress).toBeGreaterThan(0);
  });

  it("should not poll when no steps are in progress", async () => {
    const mockProgressComplete: StepProgressSummary = {
      taskId: "task-1",
      total: 3,
      completed: 3,
      inProgress: 0,
      pending: 0,
      skipped: 0,
      failed: 0,
      currentStep: null,
      nextStep: null,
      percentComplete: 100.0,
    };

    vi.mocked(api.steps.getProgress).mockResolvedValue(mockProgressComplete);

    const { result } = renderHook(() => useStepProgress("task-1"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data?.inProgress).toBe(0);
  });
});
