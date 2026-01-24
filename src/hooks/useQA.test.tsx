import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import {
  useQASettings,
  useTaskQA,
  useQAResults,
  useQAActions,
  useIsQAEnabled,
  useTaskNeedsQA,
  qaKeys,
} from "./useQA";
import { useQAStore } from "@/stores/qaStore";
import { DEFAULT_QA_SETTINGS } from "@/types/qa-config";
import { api } from "@/lib/tauri";
import type { TaskQAResponse, QAResultsResponse } from "@/lib/tauri";

// Mock the Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    qa: {
      getSettings: vi.fn(),
      updateSettings: vi.fn(),
      getTaskQA: vi.fn(),
      getResults: vi.fn(),
      retry: vi.fn(),
      skip: vi.fn(),
    },
  },
}));

const mockApi = api as {
  qa: {
    getSettings: ReturnType<typeof vi.fn>;
    updateSettings: ReturnType<typeof vi.fn>;
    getTaskQA: ReturnType<typeof vi.fn>;
    getResults: ReturnType<typeof vi.fn>;
    retry: ReturnType<typeof vi.fn>;
    skip: ReturnType<typeof vi.fn>;
  };
};

// Helper to create mock TaskQA response
const createMockTaskQAResponse = (
  overrides: Partial<TaskQAResponse> = {}
): TaskQAResponse => ({
  id: "qa-1",
  task_id: "task-1",
  screenshots: [],
  created_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

// Helper to create mock QA results
const createMockQAResults = (
  overrides: Partial<QAResultsResponse> = {}
): QAResultsResponse => ({
  task_id: "task-1",
  overall_status: "passed",
  total_steps: 2,
  passed_steps: 2,
  failed_steps: 0,
  steps: [
    { step_id: "QA1", status: "passed" },
    { step_id: "QA2", status: "passed" },
  ],
  ...overrides,
});

// Create wrapper with QueryClient
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("qaKeys", () => {
  it("creates correct query keys", () => {
    expect(qaKeys.all).toEqual(["qa"]);
    expect(qaKeys.settings()).toEqual(["qa", "settings"]);
    expect(qaKeys.taskQA()).toEqual(["qa", "taskQA"]);
    expect(qaKeys.taskQAById("task-1")).toEqual(["qa", "taskQA", "task-1"]);
    expect(qaKeys.results()).toEqual(["qa", "results"]);
    expect(qaKeys.resultsById("task-1")).toEqual(["qa", "results", "task-1"]);
  });
});

describe("useQASettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  it("fetches settings on mount", async () => {
    const settings = { ...DEFAULT_QA_SETTINGS, qa_enabled: false };
    mockApi.qa.getSettings.mockResolvedValue(settings);

    const { result } = renderHook(() => useQASettings(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.qa.getSettings).toHaveBeenCalled();
    expect(result.current.settings.qa_enabled).toBe(false);
  });

  it("returns cached settings when already loaded", async () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: false },
      settingsLoaded: true,
    });

    const { result } = renderHook(() => useQASettings(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(result.current.settings.qa_enabled).toBe(false);
    expect(mockApi.qa.getSettings).not.toHaveBeenCalled();
  });

  it("updates settings optimistically", async () => {
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: true,
    });

    const updatedSettings = { ...DEFAULT_QA_SETTINGS, qa_enabled: false };
    mockApi.qa.updateSettings.mockResolvedValue(updatedSettings);

    const { result } = renderHook(() => useQASettings(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.updateSettings({ qa_enabled: false });
    });

    expect(result.current.settings.qa_enabled).toBe(false);
  });

  it("handles update errors", async () => {
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: true,
    });

    mockApi.qa.updateSettings.mockRejectedValue(new Error("Update failed"));

    const { result } = renderHook(() => useQASettings(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      try {
        await result.current.updateSettings({ qa_enabled: false });
      } catch {
        // Expected error
      }
    });

    // Wait for mutation error to be set
    await waitFor(() => {
      expect(result.current.error).toBe("Update failed");
    });
  });
});

describe("useTaskQA", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  it("fetches task QA data on mount", async () => {
    const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
    mockApi.qa.getTaskQA.mockResolvedValue(taskQA);

    const { result } = renderHook(() => useTaskQA("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.qa.getTaskQA).toHaveBeenCalledWith("task-1");
    expect(result.current.data?.task_id).toBe("task-1");
  });

  it("returns null when no QA data exists", async () => {
    mockApi.qa.getTaskQA.mockResolvedValue(null);

    const { result } = renderHook(() => useTaskQA("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toBeNull();
  });

  it("returns store data when available", async () => {
    const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
    useQAStore.setState({
      taskQA: { "task-1": taskQA },
    });

    mockApi.qa.getTaskQA.mockResolvedValue(taskQA);

    const { result } = renderHook(() => useTaskQA("task-1"), {
      wrapper: createWrapper(),
    });

    // Should return store data immediately
    expect(result.current.data?.task_id).toBe("task-1");
  });

  it("does not fetch when disabled", async () => {
    const { result } = renderHook(() => useTaskQA("task-1", { enabled: false }), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.qa.getTaskQA).not.toHaveBeenCalled();
  });
});

describe("useQAResults", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  it("fetches QA results on mount", async () => {
    const results = createMockQAResults({ overall_status: "passed" });
    mockApi.qa.getResults.mockResolvedValue(results);

    const { result } = renderHook(() => useQAResults("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.qa.getResults).toHaveBeenCalledWith("task-1");
    expect(result.current.data?.overall_status).toBe("passed");
    expect(result.current.isPassed).toBe(true);
    expect(result.current.isFailed).toBe(false);
  });

  it("computes isPassed correctly", async () => {
    const results = createMockQAResults({ overall_status: "passed" });
    mockApi.qa.getResults.mockResolvedValue(results);

    const { result } = renderHook(() => useQAResults("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isPassed).toBe(true);
    });
  });

  it("computes isFailed correctly", async () => {
    const results = createMockQAResults({ overall_status: "failed" });
    mockApi.qa.getResults.mockResolvedValue(results);

    const { result } = renderHook(() => useQAResults("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isFailed).toBe(true);
    });
  });

  it("computes isActive for running status", async () => {
    const results = createMockQAResults({ overall_status: "running" });
    useQAStore.setState({
      taskQA: {
        "task-1": createMockTaskQAResponse({
          task_id: "task-1",
          test_results: results,
        }),
      },
    });

    mockApi.qa.getResults.mockResolvedValue(results);

    const { result } = renderHook(() => useQAResults("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isActive).toBe(true);
  });

  it("returns null when no results", async () => {
    mockApi.qa.getResults.mockResolvedValue(null);

    const { result } = renderHook(() => useQAResults("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toBeNull();
  });
});

describe("useQAActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  it("retries QA and updates store", async () => {
    const taskQA = createMockTaskQAResponse({
      task_id: "task-1",
      test_results: createMockQAResults({ overall_status: "pending" }),
    });
    mockApi.qa.retry.mockResolvedValue(taskQA);

    const { result } = renderHook(() => useQAActions("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isRetrying).toBe(false);

    await act(async () => {
      await result.current.retry();
    });

    expect(mockApi.qa.retry).toHaveBeenCalledWith("task-1");
    expect(useQAStore.getState().taskQA["task-1"]).toBeDefined();
  });

  it("skips QA and updates store", async () => {
    const taskQA = createMockTaskQAResponse({
      task_id: "task-1",
      test_results: createMockQAResults({
        overall_status: "passed",
        steps: [{ step_id: "QA1", status: "skipped" }],
      }),
    });
    mockApi.qa.skip.mockResolvedValue(taskQA);

    const { result } = renderHook(() => useQAActions("task-1"), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.skip();
    });

    expect(mockApi.qa.skip).toHaveBeenCalledWith("task-1");
    expect(useQAStore.getState().taskQA["task-1"]).toBeDefined();
  });

  it("handles retry errors", async () => {
    mockApi.qa.retry.mockRejectedValue(new Error("Retry failed"));

    const { result } = renderHook(() => useQAActions("task-1"), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      try {
        await result.current.retry();
      } catch {
        // Expected
      }
    });

    await waitFor(() => {
      expect(result.current.retryError).toBe("Retry failed");
    });
  });

  it("handles skip errors", async () => {
    mockApi.qa.skip.mockRejectedValue(new Error("Skip failed"));

    const { result } = renderHook(() => useQAActions("task-1"), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      try {
        await result.current.skip();
      } catch {
        // Expected
      }
    });

    await waitFor(() => {
      expect(result.current.skipError).toBe("Skip failed");
    });
  });
});

describe("useIsQAEnabled", () => {
  beforeEach(() => {
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: true,
    });
  });

  it("returns true when QA is enabled", () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: true },
    });

    const { result } = renderHook(() => useIsQAEnabled());

    expect(result.current).toBe(true);
  });

  it("returns false when QA is disabled", () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: false },
    });

    const { result } = renderHook(() => useIsQAEnabled());

    expect(result.current).toBe(false);
  });
});

describe("useTaskNeedsQA", () => {
  beforeEach(() => {
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: true,
    });
  });

  it("returns override when not null", () => {
    const { result } = renderHook(() => useTaskNeedsQA("feature", true));
    expect(result.current).toBe(true);

    const { result: result2 } = renderHook(() => useTaskNeedsQA("feature", false));
    expect(result2.current).toBe(false);
  });

  it("returns false when QA is globally disabled", () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: false },
    });

    const { result } = renderHook(() => useTaskNeedsQA("ui", null));
    expect(result.current).toBe(false);
  });

  it("returns auto_qa_for_ui_tasks for UI categories", () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, auto_qa_for_ui_tasks: true },
    });

    const { result: uiResult } = renderHook(() => useTaskNeedsQA("ui", null));
    expect(uiResult.current).toBe(true);

    const { result: componentResult } = renderHook(() => useTaskNeedsQA("component", null));
    expect(componentResult.current).toBe(true);

    const { result: featureResult } = renderHook(() => useTaskNeedsQA("feature", null));
    expect(featureResult.current).toBe(true);
  });

  it("returns auto_qa_for_api_tasks for API categories", () => {
    useQAStore.setState({
      settings: { ...DEFAULT_QA_SETTINGS, auto_qa_for_api_tasks: true },
    });

    const { result: apiResult } = renderHook(() => useTaskNeedsQA("api", null));
    expect(apiResult.current).toBe(true);

    const { result: backendResult } = renderHook(() => useTaskNeedsQA("backend", null));
    expect(backendResult.current).toBe(true);

    const { result: endpointResult } = renderHook(() => useTaskNeedsQA("endpoint", null));
    expect(endpointResult.current).toBe(true);
  });

  it("returns false for unknown categories", () => {
    const { result } = renderHook(() => useTaskNeedsQA("unknown", null));
    expect(result.current).toBe(false);
  });
});
