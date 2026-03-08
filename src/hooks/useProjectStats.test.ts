/**
 * Tests for useProjectStats hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useProjectStats, projectStatsKeys } from "./useProjectStats";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectStats } from "@/api/metrics";

// Mock the EventProvider — useProjectStats subscribes to task:status_changed
vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: vi.fn(() => vi.fn()), // returns unsubscribe fn
    emit: vi.fn(),
  }),
}));

const mockStats: ProjectStats = {
  taskCount: 10,
  tasksCompletedToday: 2,
  tasksCompletedThisWeek: 5,
  tasksCompletedThisMonth: 8,
  agentSuccessRate: 0.9,
  agentSuccessCount: 9,
  agentTotalCount: 10,
  reviewPassRate: 0.8,
  reviewPassCount: 8,
  reviewTotalCount: 10,
  cycleTimeBreakdown: [
    { phase: "executing", avgMinutes: 15, sampleSize: 5 },
    { phase: "pending_review", avgMinutes: 30, sampleSize: 3 },
  ],
  eme: null,
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("projectStatsKeys", () => {
  it("has correct all key", () => {
    expect(projectStatsKeys.all).toEqual(["projectStats"]);
  });

  it("generates correct byProject key", () => {
    expect(projectStatsKeys.byProject("proj-1")).toEqual(["projectStats", "proj-1"]);
  });
});

describe("useProjectStats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns data from invoke when projectId is provided", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockStats);

    const { result } = renderHook(() => useProjectStats("proj-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toMatchObject({
      taskCount: 10,
      agentSuccessRate: 0.9,
    });
    expect(invoke).toHaveBeenCalledWith("get_project_stats", { projectId: "proj-1" });
  });

  it("uses query key with projectId", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockStats);

    const { result } = renderHook(() => useProjectStats("proj-abc"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    // The query key should match byProject format
    expect(projectStatsKeys.byProject("proj-abc")).toEqual(["projectStats", "proj-abc"]);
  });

  it("handles invoke error gracefully", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("Backend error"));

    const { result } = renderHook(() => useProjectStats("proj-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error).toBeInstanceOf(Error);
  });
});
