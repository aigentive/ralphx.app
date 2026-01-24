/**
 * useDependencyGraph hook tests
 *
 * Tests for useDependencyGraph and useDependencyMutations hooks
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useDependencyGraph,
  useDependencyMutations,
  dependencyKeys,
} from "./useDependencyGraph";
import { ideationApi } from "@/api/ideation";
import type { DependencyGraphResponse } from "@/api/ideation";

// Mock the ideation API
vi.mock("@/api/ideation", () => ({
  ideationApi: {
    dependencies: {
      analyze: vi.fn(),
      add: vi.fn(),
      remove: vi.fn(),
    },
  },
}));

// Create mock data
const mockGraph: DependencyGraphResponse = {
  nodes: [
    { proposalId: "proposal-1", title: "Setup database", inDegree: 0, outDegree: 2 },
    { proposalId: "proposal-2", title: "Create API", inDegree: 1, outDegree: 1 },
    { proposalId: "proposal-3", title: "Build UI", inDegree: 1, outDegree: 0 },
  ],
  edges: [
    { from: "proposal-1", to: "proposal-2" },
    { from: "proposal-2", to: "proposal-3" },
  ],
  criticalPath: ["proposal-1", "proposal-2", "proposal-3"],
  hasCycles: false,
  cycles: null,
};

const mockGraphWithCycles: DependencyGraphResponse = {
  nodes: [
    { proposalId: "proposal-1", title: "Task A", inDegree: 1, outDegree: 1 },
    { proposalId: "proposal-2", title: "Task B", inDegree: 1, outDegree: 1 },
  ],
  edges: [
    { from: "proposal-1", to: "proposal-2" },
    { from: "proposal-2", to: "proposal-1" },
  ],
  criticalPath: [],
  hasCycles: true,
  cycles: [["proposal-1", "proposal-2", "proposal-1"]],
};

const emptyGraph: DependencyGraphResponse = {
  nodes: [],
  edges: [],
  criticalPath: [],
  hasCycles: false,
  cycles: null,
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

describe("dependencyKeys", () => {
  it("should generate correct key for all", () => {
    expect(dependencyKeys.all).toEqual(["dependencies"]);
  });

  it("should generate correct key for graphs", () => {
    expect(dependencyKeys.graphs()).toEqual(["dependencies", "graph"]);
  });

  it("should generate correct key for graph by session", () => {
    expect(dependencyKeys.graph("session-1")).toEqual([
      "dependencies",
      "graph",
      "session-1",
    ]);
  });
});

describe("useDependencyGraph", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch dependency graph for session", async () => {
    vi.mocked(ideationApi.dependencies.analyze).mockResolvedValueOnce(mockGraph);

    const { result } = renderHook(() => useDependencyGraph("session-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockGraph);
    expect(ideationApi.dependencies.analyze).toHaveBeenCalledWith("session-1");
  });

  it("should return empty graph for session with no proposals", async () => {
    vi.mocked(ideationApi.dependencies.analyze).mockResolvedValueOnce(emptyGraph);

    const { result } = renderHook(() => useDependencyGraph("session-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(emptyGraph);
    expect(result.current.data?.hasCycles).toBe(false);
    expect(result.current.data?.nodes).toHaveLength(0);
  });

  it("should detect cycles in graph", async () => {
    vi.mocked(ideationApi.dependencies.analyze).mockResolvedValueOnce(mockGraphWithCycles);

    const { result } = renderHook(() => useDependencyGraph("session-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data?.hasCycles).toBe(true);
    expect(result.current.data?.cycles).toHaveLength(1);
  });

  it("should return critical path", async () => {
    vi.mocked(ideationApi.dependencies.analyze).mockResolvedValueOnce(mockGraph);

    const { result } = renderHook(() => useDependencyGraph("session-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data?.criticalPath).toEqual([
      "proposal-1",
      "proposal-2",
      "proposal-3",
    ]);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to analyze dependencies");
    vi.mocked(ideationApi.dependencies.analyze).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useDependencyGraph("session-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });

  it("should not fetch when sessionId is empty", async () => {
    const { result } = renderHook(() => useDependencyGraph(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(ideationApi.dependencies.analyze).not.toHaveBeenCalled();
  });
});

describe("useDependencyMutations", () => {
  describe("addDependency", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    afterEach(() => {
      vi.resetAllMocks();
    });

    it("should add a dependency successfully", async () => {
      vi.mocked(ideationApi.dependencies.add).mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useDependencyMutations(), {
        wrapper: createWrapper(),
      });

      await act(async () => {
        await result.current.addDependency.mutateAsync({
          proposalId: "proposal-2",
          dependsOnId: "proposal-1",
        });
      });

      expect(ideationApi.dependencies.add).toHaveBeenCalledWith("proposal-2", "proposal-1");
    });

    it("should handle add dependency error", async () => {
      const error = new Error("Failed to add dependency");
      vi.mocked(ideationApi.dependencies.add).mockRejectedValueOnce(error);

      const { result } = renderHook(() => useDependencyMutations(), {
        wrapper: createWrapper(),
      });

      await expect(
        act(async () => {
          await result.current.addDependency.mutateAsync({
            proposalId: "proposal-2",
            dependsOnId: "proposal-1",
          });
        })
      ).rejects.toThrow("Failed to add dependency");
    });
  });

  describe("removeDependency", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    afterEach(() => {
      vi.resetAllMocks();
    });

    it("should remove a dependency successfully", async () => {
      vi.mocked(ideationApi.dependencies.remove).mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useDependencyMutations(), {
        wrapper: createWrapper(),
      });

      await act(async () => {
        await result.current.removeDependency.mutateAsync({
          proposalId: "proposal-2",
          dependsOnId: "proposal-1",
        });
      });

      expect(ideationApi.dependencies.remove).toHaveBeenCalledWith(
        "proposal-2",
        "proposal-1"
      );
    });

    it("should handle remove dependency error", async () => {
      const error = new Error("Failed to remove dependency");
      vi.mocked(ideationApi.dependencies.remove).mockRejectedValueOnce(error);

      const { result } = renderHook(() => useDependencyMutations(), {
        wrapper: createWrapper(),
      });

      await expect(
        act(async () => {
          await result.current.removeDependency.mutateAsync({
            proposalId: "proposal-2",
            dependsOnId: "proposal-1",
          });
        })
      ).rejects.toThrow("Failed to remove dependency");
    });
  });
});
