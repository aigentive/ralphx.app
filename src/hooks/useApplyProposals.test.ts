/**
 * useApplyProposals hook tests
 *
 * Tests for useApplyProposals hook for applying proposals to Kanban
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useApplyProposals } from "./useApplyProposals";
import { ideationApi } from "@/api/ideation";
import type { ApplyProposalsResultResponse } from "@/api/ideation";

// Mock the ideation API
vi.mock("@/api/ideation", () => ({
  ideationApi: {
    apply: {
      toKanban: vi.fn(),
    },
  },
}));

// Create mock data
const mockSuccessResult: ApplyProposalsResultResponse = {
  createdTaskIds: ["task-1", "task-2", "task-3"],
  dependenciesCreated: 2,
  warnings: [],
  sessionConverted: false,
};

const mockSuccessWithWarnings: ApplyProposalsResultResponse = {
  createdTaskIds: ["task-1", "task-2"],
  dependenciesCreated: 1,
  warnings: ["Proposal 3 was skipped due to missing dependency"],
  sessionConverted: false,
};

const mockConvertedSession: ApplyProposalsResultResponse = {
  createdTaskIds: ["task-1"],
  dependenciesCreated: 0,
  warnings: [],
  sessionConverted: true,
};

// Test wrapper with QueryClientProvider
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("useApplyProposals", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should apply proposals successfully", async () => {
    vi.mocked(ideationApi.apply.toKanban).mockResolvedValueOnce(mockSuccessResult);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    let applyResult: ApplyProposalsResultResponse | undefined;
    await act(async () => {
      applyResult = await result.current.apply.mutateAsync({
        sessionId: "session-1",
        proposalIds: ["proposal-1", "proposal-2", "proposal-3"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    expect(applyResult).toEqual(mockSuccessResult);
    expect(ideationApi.apply.toKanban).toHaveBeenCalledWith({
      sessionId: "session-1",
      proposalIds: ["proposal-1", "proposal-2", "proposal-3"],
      targetColumn: "backlog",
      preserveDependencies: true,
    });
  });

  it("should handle apply with warnings", async () => {
    vi.mocked(ideationApi.apply.toKanban).mockResolvedValueOnce(mockSuccessWithWarnings);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    let applyResult: ApplyProposalsResultResponse | undefined;
    await act(async () => {
      applyResult = await result.current.apply.mutateAsync({
        sessionId: "session-1",
        proposalIds: ["proposal-1", "proposal-2", "proposal-3"],
        targetColumn: "todo",
        preserveDependencies: true,
      });
    });

    expect(applyResult?.warnings).toHaveLength(1);
    expect(applyResult?.createdTaskIds).toHaveLength(2);
  });

  it("should handle session conversion", async () => {
    vi.mocked(ideationApi.apply.toKanban).mockResolvedValueOnce(mockConvertedSession);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    let applyResult: ApplyProposalsResultResponse | undefined;
    await act(async () => {
      applyResult = await result.current.apply.mutateAsync({
        sessionId: "session-1",
        proposalIds: ["proposal-1"],
        targetColumn: "draft",
        preserveDependencies: false,
      });
    });

    expect(applyResult?.sessionConverted).toBe(true);
  });

  it("should apply to different target columns", async () => {
    vi.mocked(ideationApi.apply.toKanban).mockResolvedValueOnce(mockSuccessResult);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.apply.mutateAsync({
        sessionId: "session-1",
        proposalIds: ["proposal-1"],
        targetColumn: "draft",
        preserveDependencies: false,
      });
    });

    expect(ideationApi.apply.toKanban).toHaveBeenCalledWith({
      sessionId: "session-1",
      proposalIds: ["proposal-1"],
      targetColumn: "draft",
      preserveDependencies: false,
    });
  });

  it("should handle apply error", async () => {
    const error = new Error("Failed to apply proposals");
    vi.mocked(ideationApi.apply.toKanban).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.apply.mutateAsync({
          sessionId: "session-1",
          proposalIds: ["proposal-1"],
          targetColumn: "backlog",
          preserveDependencies: true,
        });
      })
    ).rejects.toThrow("Failed to apply proposals");
  });

  it("should set loading state during apply", async () => {
    let resolvePromise: (value: ApplyProposalsResultResponse) => void;
    const promise = new Promise<ApplyProposalsResultResponse>((resolve) => {
      resolvePromise = resolve;
    });
    vi.mocked(ideationApi.apply.toKanban).mockReturnValueOnce(promise);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    expect(result.current.apply.isPending).toBe(false);

    act(() => {
      result.current.apply.mutate({
        sessionId: "session-1",
        proposalIds: ["proposal-1"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    await waitFor(() => {
      expect(result.current.apply.isPending).toBe(true);
    });

    await act(async () => {
      resolvePromise!(mockSuccessResult);
      await promise;
    });

    await waitFor(() => {
      expect(result.current.apply.isPending).toBe(false);
    });
  });

  it("should handle circular dependency error", async () => {
    const error = new Error("Circular dependencies detected in selection");
    vi.mocked(ideationApi.apply.toKanban).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.apply.mutateAsync({
          sessionId: "session-1",
          proposalIds: ["proposal-1", "proposal-2"],
          targetColumn: "backlog",
          preserveDependencies: true,
        });
      })
    ).rejects.toThrow("Circular dependencies detected in selection");
  });

  it("should handle empty proposal selection", async () => {
    const emptyResult: ApplyProposalsResultResponse = {
      createdTaskIds: [],
      dependenciesCreated: 0,
      warnings: ["No proposals were selected"],
      sessionConverted: false,
    };
    vi.mocked(ideationApi.apply.toKanban).mockResolvedValueOnce(emptyResult);

    const { result } = renderHook(() => useApplyProposals(), {
      wrapper: createWrapper(),
    });

    let applyResult: ApplyProposalsResultResponse | undefined;
    await act(async () => {
      applyResult = await result.current.apply.mutateAsync({
        sessionId: "session-1",
        proposalIds: [],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    expect(applyResult?.createdTaskIds).toHaveLength(0);
    expect(applyResult?.warnings).toContain("No proposals were selected");
  });
});
