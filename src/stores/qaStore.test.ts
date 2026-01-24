import { describe, it, expect, beforeEach } from "vitest";
import {
  useQAStore,
  selectTaskQA,
  selectIsQAEnabled,
  selectIsTaskLoading,
  selectTaskQAResults,
  selectHasTaskQA,
} from "./qaStore";
import { DEFAULT_QA_SETTINGS } from "@/types/qa-config";
import type { TaskQAResponse, QAResultsResponse } from "@/lib/tauri";

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

describe("qaStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  describe("setSettings", () => {
    it("sets settings and marks as loaded", () => {
      const newSettings = {
        ...DEFAULT_QA_SETTINGS,
        qa_enabled: false,
        browser_testing_url: "http://localhost:3000",
      };

      useQAStore.getState().setSettings(newSettings);

      const state = useQAStore.getState();
      expect(state.settings.qa_enabled).toBe(false);
      expect(state.settings.browser_testing_url).toBe("http://localhost:3000");
      expect(state.settingsLoaded).toBe(true);
      expect(state.isLoadingSettings).toBe(false);
    });

    it("preserves other settings when setting new values", () => {
      const newSettings = {
        ...DEFAULT_QA_SETTINGS,
        qa_enabled: false,
      };

      useQAStore.getState().setSettings(newSettings);

      const state = useQAStore.getState();
      expect(state.settings.auto_qa_for_ui_tasks).toBe(true);
      expect(state.settings.qa_prep_enabled).toBe(true);
    });
  });

  describe("updateSettings", () => {
    it("updates specific settings fields", () => {
      useQAStore.getState().updateSettings({ qa_enabled: false });

      const state = useQAStore.getState();
      expect(state.settings.qa_enabled).toBe(false);
      expect(state.settings.auto_qa_for_ui_tasks).toBe(true); // Unchanged
    });

    it("updates multiple fields at once", () => {
      useQAStore.getState().updateSettings({
        qa_enabled: false,
        browser_testing_url: "http://localhost:5000",
      });

      const state = useQAStore.getState();
      expect(state.settings.qa_enabled).toBe(false);
      expect(state.settings.browser_testing_url).toBe("http://localhost:5000");
    });
  });

  describe("setLoadingSettings", () => {
    it("sets loading state to true", () => {
      useQAStore.getState().setLoadingSettings(true);

      expect(useQAStore.getState().isLoadingSettings).toBe(true);
    });

    it("sets loading state to false", () => {
      useQAStore.setState({ isLoadingSettings: true });

      useQAStore.getState().setLoadingSettings(false);

      expect(useQAStore.getState().isLoadingSettings).toBe(false);
    });
  });

  describe("setTaskQA", () => {
    it("sets task QA data for a task", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });

      useQAStore.getState().setTaskQA("task-1", taskQA);

      const state = useQAStore.getState();
      expect(state.taskQA["task-1"]).toBeDefined();
      expect(state.taskQA["task-1"]?.task_id).toBe("task-1");
    });

    it("removes task QA data when null", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      useQAStore.getState().setTaskQA("task-1", null);

      expect(useQAStore.getState().taskQA["task-1"]).toBeUndefined();
    });

    it("clears loading state for task", () => {
      useQAStore.setState({ loadingTasks: new Set(["task-1"]) });
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });

      useQAStore.getState().setTaskQA("task-1", taskQA);

      expect(useQAStore.getState().loadingTasks.has("task-1")).toBe(false);
    });

    it("can store multiple tasks", () => {
      const taskQA1 = createMockTaskQAResponse({ id: "qa-1", task_id: "task-1" });
      const taskQA2 = createMockTaskQAResponse({ id: "qa-2", task_id: "task-2" });

      useQAStore.getState().setTaskQA("task-1", taskQA1);
      useQAStore.getState().setTaskQA("task-2", taskQA2);

      const state = useQAStore.getState();
      expect(Object.keys(state.taskQA)).toHaveLength(2);
    });
  });

  describe("updateTaskQA", () => {
    it("updates existing task QA data", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      useQAStore.getState().updateTaskQA("task-1", {
        prep_completed_at: "2026-01-24T13:00:00Z",
      });

      expect(useQAStore.getState().taskQA["task-1"]?.prep_completed_at).toBe(
        "2026-01-24T13:00:00Z"
      );
    });

    it("does nothing if task does not exist", () => {
      useQAStore.getState().updateTaskQA("nonexistent", {
        prep_completed_at: "2026-01-24T13:00:00Z",
      });

      expect(useQAStore.getState().taskQA["nonexistent"]).toBeUndefined();
    });

    it("preserves other task QA fields", () => {
      const taskQA = createMockTaskQAResponse({
        task_id: "task-1",
        screenshots: ["ss1.png", "ss2.png"],
      });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      useQAStore.getState().updateTaskQA("task-1", {
        prep_completed_at: "2026-01-24T13:00:00Z",
      });

      const updated = useQAStore.getState().taskQA["task-1"];
      expect(updated?.screenshots).toEqual(["ss1.png", "ss2.png"]);
    });
  });

  describe("setLoadingTask", () => {
    it("adds task to loading set when true", () => {
      useQAStore.getState().setLoadingTask("task-1", true);

      expect(useQAStore.getState().loadingTasks.has("task-1")).toBe(true);
    });

    it("removes task from loading set when false", () => {
      useQAStore.setState({ loadingTasks: new Set(["task-1"]) });

      useQAStore.getState().setLoadingTask("task-1", false);

      expect(useQAStore.getState().loadingTasks.has("task-1")).toBe(false);
    });

    it("handles multiple loading tasks", () => {
      useQAStore.getState().setLoadingTask("task-1", true);
      useQAStore.getState().setLoadingTask("task-2", true);

      const loadingTasks = useQAStore.getState().loadingTasks;
      expect(loadingTasks.has("task-1")).toBe(true);
      expect(loadingTasks.has("task-2")).toBe(true);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useQAStore.getState().setError("Something went wrong");

      expect(useQAStore.getState().error).toBe("Something went wrong");
    });

    it("clears error when null", () => {
      useQAStore.setState({ error: "Previous error" });

      useQAStore.getState().setError(null);

      expect(useQAStore.getState().error).toBeNull();
    });
  });

  describe("clearTaskQA", () => {
    it("clears all task QA data", () => {
      useQAStore.setState({
        taskQA: {
          "task-1": createMockTaskQAResponse({ task_id: "task-1" }),
          "task-2": createMockTaskQAResponse({ task_id: "task-2" }),
        },
        loadingTasks: new Set(["task-1", "task-2"]),
      });

      useQAStore.getState().clearTaskQA();

      const state = useQAStore.getState();
      expect(Object.keys(state.taskQA)).toHaveLength(0);
      expect(state.loadingTasks.size).toBe(0);
    });
  });

  describe("removeTaskQA", () => {
    it("removes specific task QA data", () => {
      useQAStore.setState({
        taskQA: {
          "task-1": createMockTaskQAResponse({ task_id: "task-1" }),
          "task-2": createMockTaskQAResponse({ task_id: "task-2" }),
        },
      });

      useQAStore.getState().removeTaskQA("task-1");

      const state = useQAStore.getState();
      expect(state.taskQA["task-1"]).toBeUndefined();
      expect(state.taskQA["task-2"]).toBeDefined();
    });

    it("clears loading state for removed task", () => {
      useQAStore.setState({
        taskQA: { "task-1": createMockTaskQAResponse({ task_id: "task-1" }) },
        loadingTasks: new Set(["task-1"]),
      });

      useQAStore.getState().removeTaskQA("task-1");

      expect(useQAStore.getState().loadingTasks.has("task-1")).toBe(false);
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useQAStore.setState({
      settings: DEFAULT_QA_SETTINGS,
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  describe("selectTaskQA", () => {
    it("returns task QA when exists", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      const result = selectTaskQA("task-1")(useQAStore.getState());

      expect(result?.task_id).toBe("task-1");
    });

    it("returns null when task QA does not exist", () => {
      const result = selectTaskQA("nonexistent")(useQAStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectIsQAEnabled", () => {
    it("returns true when QA is enabled", () => {
      useQAStore.setState({ settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: true } });

      expect(selectIsQAEnabled(useQAStore.getState())).toBe(true);
    });

    it("returns false when QA is disabled", () => {
      useQAStore.setState({ settings: { ...DEFAULT_QA_SETTINGS, qa_enabled: false } });

      expect(selectIsQAEnabled(useQAStore.getState())).toBe(false);
    });
  });

  describe("selectIsTaskLoading", () => {
    it("returns true when task is loading", () => {
      useQAStore.setState({ loadingTasks: new Set(["task-1"]) });

      expect(selectIsTaskLoading("task-1")(useQAStore.getState())).toBe(true);
    });

    it("returns false when task is not loading", () => {
      expect(selectIsTaskLoading("task-1")(useQAStore.getState())).toBe(false);
    });
  });

  describe("selectTaskQAResults", () => {
    it("returns test results when available", () => {
      const results = createMockQAResults();
      const taskQA = createMockTaskQAResponse({
        task_id: "task-1",
        test_results: results,
      });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      const result = selectTaskQAResults("task-1")(useQAStore.getState());

      expect(result?.overall_status).toBe("passed");
      expect(result?.total_steps).toBe(2);
    });

    it("returns null when no test results", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      expect(selectTaskQAResults("task-1")(useQAStore.getState())).toBeNull();
    });

    it("returns null when task QA does not exist", () => {
      expect(selectTaskQAResults("nonexistent")(useQAStore.getState())).toBeNull();
    });
  });

  describe("selectHasTaskQA", () => {
    it("returns true when task has QA data", () => {
      const taskQA = createMockTaskQAResponse({ task_id: "task-1" });
      useQAStore.setState({ taskQA: { "task-1": taskQA } });

      expect(selectHasTaskQA("task-1")(useQAStore.getState())).toBe(true);
    });

    it("returns false when task has no QA data", () => {
      expect(selectHasTaskQA("nonexistent")(useQAStore.getState())).toBe(false);
    });
  });
});
