/**
 * useResearch hooks tests
 *
 * Tests for useResearchProcesses, useResearchProcess, useResearchPresets,
 * and research mutation hooks using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useResearchProcesses,
  useResearchProcess,
  useResearchPresets,
  useStartResearch,
  usePauseResearch,
  useResumeResearch,
  useStopResearch,
  researchKeys,
} from "./useResearch";
import { api } from "@/lib/tauri";
import type {
  ResearchProcessResponse,
  ResearchPresetResponse,
} from "@/api/research";

// Mock the tauri API wrapper used by research hooks
vi.mock("@/lib/tauri", () => ({
  api: {
    research: {
      getProcesses: vi.fn(),
      getProcess: vi.fn(),
      getPresets: vi.fn(),
      start: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      stop: vi.fn(),
    },
  },
}));

// Create mock data
const mockProcess: ResearchProcessResponse = {
  id: "process-1",
  name: "Test Research",
  question: "How to implement feature X?",
  context: "We are building a task management system",
  scope: "Frontend components",
  constraints: ["Must use React", "Must support dark mode"],
  agent_profile_id: "deep-researcher",
  depth_preset: "standard",
  max_iterations: 50,
  timeout_hours: 2,
  checkpoint_interval: 10,
  target_bucket: "research-outputs",
  status: "running",
  current_iteration: 15,
  progress_percentage: 30,
  error_message: null,
  created_at: "2026-01-24T10:00:00Z",
  started_at: "2026-01-24T10:01:00Z",
  completed_at: null,
};

const mockProcess2: ResearchProcessResponse = {
  id: "process-2",
  name: "Completed Research",
  question: "What are best practices for Y?",
  context: null,
  scope: null,
  constraints: [],
  agent_profile_id: "deep-researcher",
  depth_preset: "quick-scan",
  max_iterations: 10,
  timeout_hours: 0.5,
  checkpoint_interval: 5,
  target_bucket: "research-outputs",
  status: "completed",
  current_iteration: 10,
  progress_percentage: 100,
  error_message: null,
  created_at: "2026-01-24T08:00:00Z",
  started_at: "2026-01-24T08:01:00Z",
  completed_at: "2026-01-24T08:20:00Z",
};

const mockPreset: ResearchPresetResponse = {
  id: "quick-scan",
  name: "Quick Scan",
  max_iterations: 10,
  timeout_hours: 0.5,
  checkpoint_interval: 5,
  description: "Fast overview",
};

const mockPreset2: ResearchPresetResponse = {
  id: "standard",
  name: "Standard",
  max_iterations: 50,
  timeout_hours: 2,
  checkpoint_interval: 10,
  description: "Thorough investigation",
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

describe("researchKeys", () => {
  it("should generate correct key for all", () => {
    expect(researchKeys.all).toEqual(["research"]);
  });

  it("should generate correct key for processes", () => {
    expect(researchKeys.processes()).toEqual(["research", "processes"]);
  });

  it("should generate correct key for processList with status", () => {
    expect(researchKeys.processList("running")).toEqual([
      "research",
      "processes",
      "list",
      "running",
    ]);
  });

  it("should generate correct key for processList without status", () => {
    expect(researchKeys.processList()).toEqual([
      "research",
      "processes",
      "list",
      undefined,
    ]);
  });

  it("should generate correct key for processDetail", () => {
    expect(researchKeys.processDetail("process-1")).toEqual([
      "research",
      "processes",
      "detail",
      "process-1",
    ]);
  });

  it("should generate correct key for presets", () => {
    expect(researchKeys.presets()).toEqual(["research", "presets"]);
  });
});

describe("useResearchProcesses", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all research processes successfully", async () => {
    const mockProcesses = [mockProcess, mockProcess2];
    vi.mocked(api.research.getProcesses).mockResolvedValueOnce(mockProcesses);

    const { result } = renderHook(() => useResearchProcesses(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockProcesses);
    expect(api.research.getProcesses).toHaveBeenCalledWith(undefined);
  });

  it("should fetch processes filtered by status", async () => {
    vi.mocked(api.research.getProcesses).mockResolvedValueOnce([mockProcess]);

    const { result } = renderHook(() => useResearchProcesses("running"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([mockProcess]);
    expect(api.research.getProcesses).toHaveBeenCalledWith("running");
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch processes");
    vi.mocked(api.research.getProcesses).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useResearchProcesses(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useResearchProcess", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch a single research process successfully", async () => {
    vi.mocked(api.research.getProcess).mockResolvedValueOnce(mockProcess);

    const { result } = renderHook(() => useResearchProcess("process-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockProcess);
    expect(api.research.getProcess).toHaveBeenCalledWith("process-1");
  });

  it("should return null for non-existent process", async () => {
    vi.mocked(api.research.getProcess).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useResearchProcess("non-existent"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("should not fetch when id is empty", async () => {
    const { result } = renderHook(() => useResearchProcess(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(api.research.getProcess).not.toHaveBeenCalled();
  });
});

describe("useResearchPresets", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all research presets successfully", async () => {
    const mockPresets = [mockPreset, mockPreset2];
    vi.mocked(api.research.getPresets).mockResolvedValueOnce(mockPresets);

    const { result } = renderHook(() => useResearchPresets(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockPresets);
    expect(api.research.getPresets).toHaveBeenCalledTimes(1);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch presets");
    vi.mocked(api.research.getPresets).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useResearchPresets(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useStartResearch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should start a research process successfully", async () => {
    vi.mocked(api.research.start).mockResolvedValueOnce(mockProcess);

    const { result } = renderHook(() => useStartResearch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        name: "Test Research",
        question: "How to implement feature X?",
        agent_profile_id: "deep-researcher",
        depth_preset: "standard",
      });
    });

    expect(api.research.start).toHaveBeenCalled();
    expect(vi.mocked(api.research.start).mock.calls[0][0]).toEqual({
      name: "Test Research",
      question: "How to implement feature X?",
      agent_profile_id: "deep-researcher",
      depth_preset: "standard",
    });
  });

  it("should handle start error", async () => {
    const error = new Error("Failed to start research");
    vi.mocked(api.research.start).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useStartResearch(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          name: "Test Research",
          question: "Question?",
          agent_profile_id: "deep-researcher",
        });
      })
    ).rejects.toThrow("Failed to start research");
  });
});

describe("usePauseResearch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should pause a research process successfully", async () => {
    const pausedProcess = { ...mockProcess, status: "paused" as const };
    vi.mocked(api.research.pause).mockResolvedValueOnce(pausedProcess);

    const { result } = renderHook(() => usePauseResearch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("process-1");
    });

    expect(api.research.pause).toHaveBeenCalled();
    expect(vi.mocked(api.research.pause).mock.calls[0][0]).toBe("process-1");
  });

  it("should handle pause error", async () => {
    const error = new Error("Failed to pause research");
    vi.mocked(api.research.pause).mockRejectedValueOnce(error);

    const { result } = renderHook(() => usePauseResearch(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("process-1");
      })
    ).rejects.toThrow("Failed to pause research");
  });
});

describe("useResumeResearch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should resume a research process successfully", async () => {
    vi.mocked(api.research.resume).mockResolvedValueOnce(mockProcess);

    const { result } = renderHook(() => useResumeResearch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("process-1");
    });

    expect(api.research.resume).toHaveBeenCalled();
    expect(vi.mocked(api.research.resume).mock.calls[0][0]).toBe("process-1");
  });

  it("should handle resume error", async () => {
    const error = new Error("Failed to resume research");
    vi.mocked(api.research.resume).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useResumeResearch(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("process-1");
      })
    ).rejects.toThrow("Failed to resume research");
  });
});

describe("useStopResearch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should stop a research process successfully", async () => {
    const stoppedProcess = { ...mockProcess, status: "failed" as const };
    vi.mocked(api.research.stop).mockResolvedValueOnce(stoppedProcess);

    const { result } = renderHook(() => useStopResearch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("process-1");
    });

    expect(api.research.stop).toHaveBeenCalled();
    expect(vi.mocked(api.research.stop).mock.calls[0][0]).toBe("process-1");
  });

  it("should handle stop error", async () => {
    const error = new Error("Failed to stop research");
    vi.mocked(api.research.stop).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useStopResearch(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("process-1");
      })
    ).rejects.toThrow("Failed to stop research");
  });
});
