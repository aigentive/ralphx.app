import { describe, it, expect, beforeEach } from "vitest";
import {
  useWorkflowStore,
  selectActiveWorkflow,
  selectWorkflowColumns,
  selectWorkflowById,
} from "./workflowStore";
import type { WorkflowSchema } from "@/types/workflow";

// Helper to create test workflows
const createTestWorkflow = (overrides: Partial<WorkflowSchema> = {}): WorkflowSchema => ({
  id: `workflow-${Math.random().toString(36).slice(2)}`,
  name: "Test Workflow",
  description: "A test workflow",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  isDefault: false,
  ...overrides,
});

describe("workflowStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useWorkflowStore.setState({
      workflows: {},
      activeWorkflowId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("setWorkflows", () => {
    it("converts array to Record keyed by id", () => {
      const workflows = [
        createTestWorkflow({ id: "workflow-1", name: "Workflow 1" }),
        createTestWorkflow({ id: "workflow-2", name: "Workflow 2" }),
      ];

      useWorkflowStore.getState().setWorkflows(workflows);

      const state = useWorkflowStore.getState();
      expect(Object.keys(state.workflows)).toHaveLength(2);
      expect(state.workflows["workflow-1"]?.name).toBe("Workflow 1");
      expect(state.workflows["workflow-2"]?.name).toBe("Workflow 2");
    });

    it("replaces existing workflows", () => {
      useWorkflowStore.setState({
        workflows: {
          "old-workflow": createTestWorkflow({ id: "old-workflow", name: "Old" }),
        },
      });

      const newWorkflows = [createTestWorkflow({ id: "new-workflow", name: "New" })];
      useWorkflowStore.getState().setWorkflows(newWorkflows);

      const state = useWorkflowStore.getState();
      expect(state.workflows["old-workflow"]).toBeUndefined();
      expect(state.workflows["new-workflow"]?.name).toBe("New");
    });

    it("handles empty array", () => {
      useWorkflowStore.getState().setWorkflows([]);

      const state = useWorkflowStore.getState();
      expect(Object.keys(state.workflows)).toHaveLength(0);
    });

    it("sets activeWorkflowId to default workflow if present", () => {
      const workflows = [
        createTestWorkflow({ id: "workflow-1", isDefault: false }),
        createTestWorkflow({ id: "workflow-2", isDefault: true }),
      ];

      useWorkflowStore.getState().setWorkflows(workflows);

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("workflow-2");
    });

    it("does not change activeWorkflowId if no default and already set", () => {
      useWorkflowStore.setState({ activeWorkflowId: "existing" });

      const workflows = [
        createTestWorkflow({ id: "workflow-1", isDefault: false }),
        createTestWorkflow({ id: "workflow-2", isDefault: false }),
      ];

      useWorkflowStore.getState().setWorkflows(workflows);

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("existing");
    });
  });

  describe("setActiveWorkflow", () => {
    it("updates activeWorkflowId", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().setActiveWorkflow("workflow-1");

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("workflow-1");
    });

    it("sets activeWorkflowId to null", () => {
      useWorkflowStore.setState({ activeWorkflowId: "workflow-1" });

      useWorkflowStore.getState().setActiveWorkflow(null);

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBeNull();
    });

    it("replaces previous active workflow", () => {
      useWorkflowStore.setState({ activeWorkflowId: "workflow-1" });

      useWorkflowStore.getState().setActiveWorkflow("workflow-2");

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("workflow-2");
    });
  });

  describe("addWorkflow", () => {
    it("adds a new workflow to the store", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });

      useWorkflowStore.getState().addWorkflow(workflow);

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]).toBeDefined();
      expect(state.workflows["workflow-1"]?.name).toBe("Test Workflow");
    });

    it("overwrites workflow with same id", () => {
      const workflow1 = createTestWorkflow({ id: "workflow-1", name: "First" });
      const workflow2 = createTestWorkflow({ id: "workflow-1", name: "Second" });

      useWorkflowStore.getState().addWorkflow(workflow1);
      useWorkflowStore.getState().addWorkflow(workflow2);

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]?.name).toBe("Second");
    });

    it("sets as active if isDefault is true and no active workflow", () => {
      const workflow = createTestWorkflow({ id: "workflow-1", isDefault: true });

      useWorkflowStore.getState().addWorkflow(workflow);

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("workflow-1");
    });

    it("does not override existing active workflow even if isDefault", () => {
      useWorkflowStore.setState({ activeWorkflowId: "existing" });
      const workflow = createTestWorkflow({ id: "workflow-1", isDefault: true });

      useWorkflowStore.getState().addWorkflow(workflow);

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("existing");
    });
  });

  describe("updateWorkflow", () => {
    it("modifies existing workflow", () => {
      const workflow = createTestWorkflow({ id: "workflow-1", name: "Original" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().updateWorkflow("workflow-1", { name: "Updated" });

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]?.name).toBe("Updated");
    });

    it("updates multiple fields", () => {
      const workflow = createTestWorkflow({
        id: "workflow-1",
        name: "Original",
        description: "Original desc",
      });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().updateWorkflow("workflow-1", {
        name: "Updated",
        description: "Updated desc",
      });

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]?.name).toBe("Updated");
      expect(state.workflows["workflow-1"]?.description).toBe("Updated desc");
    });

    it("does nothing if workflow not found", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().updateWorkflow("nonexistent", { name: "Updated" });

      const state = useWorkflowStore.getState();
      expect(Object.keys(state.workflows)).toHaveLength(1);
      expect(state.workflows["workflow-1"]?.name).toBe("Test Workflow");
    });

    it("preserves other workflow fields", () => {
      const workflow = createTestWorkflow({
        id: "workflow-1",
        name: "Original",
        description: "A description",
        columns: [
          { id: "backlog", name: "Backlog", mapsTo: "backlog" },
        ],
      });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().updateWorkflow("workflow-1", { name: "Updated" });

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]?.name).toBe("Updated");
      expect(state.workflows["workflow-1"]?.description).toBe("A description");
      expect(state.workflows["workflow-1"]?.columns).toHaveLength(1);
    });

    it("can update columns", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      const newColumns = [
        { id: "new-col", name: "New Column", mapsTo: "ready" as const },
      ];
      useWorkflowStore.getState().updateWorkflow("workflow-1", { columns: newColumns });

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]?.columns).toHaveLength(1);
      expect(state.workflows["workflow-1"]?.columns[0]?.name).toBe("New Column");
    });
  });

  describe("deleteWorkflow", () => {
    it("removes a workflow from the store", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().deleteWorkflow("workflow-1");

      const state = useWorkflowStore.getState();
      expect(state.workflows["workflow-1"]).toBeUndefined();
    });

    it("clears activeWorkflowId if active workflow is deleted", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({
        workflows: { "workflow-1": workflow },
        activeWorkflowId: "workflow-1",
      });

      useWorkflowStore.getState().deleteWorkflow("workflow-1");

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBeNull();
    });

    it("does not affect activeWorkflowId if different workflow is deleted", () => {
      const workflow1 = createTestWorkflow({ id: "workflow-1" });
      const workflow2 = createTestWorkflow({ id: "workflow-2" });
      useWorkflowStore.setState({
        workflows: { "workflow-1": workflow1, "workflow-2": workflow2 },
        activeWorkflowId: "workflow-1",
      });

      useWorkflowStore.getState().deleteWorkflow("workflow-2");

      const state = useWorkflowStore.getState();
      expect(state.activeWorkflowId).toBe("workflow-1");
    });

    it("does nothing if workflow not found", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      useWorkflowStore.getState().deleteWorkflow("nonexistent");

      const state = useWorkflowStore.getState();
      expect(Object.keys(state.workflows)).toHaveLength(1);
    });
  });

  describe("setLoading", () => {
    it("sets loading state to true", () => {
      useWorkflowStore.getState().setLoading(true);

      const state = useWorkflowStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets loading state to false", () => {
      useWorkflowStore.setState({ isLoading: true });

      useWorkflowStore.getState().setLoading(false);

      const state = useWorkflowStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useWorkflowStore.getState().setError("Something went wrong");

      const state = useWorkflowStore.getState();
      expect(state.error).toBe("Something went wrong");
    });

    it("clears error with null", () => {
      useWorkflowStore.setState({ error: "Previous error" });

      useWorkflowStore.getState().setError(null);

      const state = useWorkflowStore.getState();
      expect(state.error).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useWorkflowStore.setState({
      workflows: {},
      activeWorkflowId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("selectActiveWorkflow", () => {
    it("returns active workflow when it exists", () => {
      const workflow = createTestWorkflow({ id: "workflow-1", name: "Active Workflow" });
      useWorkflowStore.setState({
        workflows: { "workflow-1": workflow },
        activeWorkflowId: "workflow-1",
      });

      const result = selectActiveWorkflow(useWorkflowStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Active Workflow");
    });

    it("returns null when no workflow is active", () => {
      const workflow = createTestWorkflow({ id: "workflow-1" });
      useWorkflowStore.setState({
        workflows: { "workflow-1": workflow },
        activeWorkflowId: null,
      });

      const result = selectActiveWorkflow(useWorkflowStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when active workflow does not exist", () => {
      useWorkflowStore.setState({
        workflows: {},
        activeWorkflowId: "nonexistent",
      });

      const result = selectActiveWorkflow(useWorkflowStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectWorkflowColumns", () => {
    it("returns columns for active workflow", () => {
      const columns = [
        { id: "backlog", name: "Backlog", mapsTo: "backlog" as const },
        { id: "done", name: "Done", mapsTo: "approved" as const },
      ];
      const workflow = createTestWorkflow({ id: "workflow-1", columns });
      useWorkflowStore.setState({
        workflows: { "workflow-1": workflow },
        activeWorkflowId: "workflow-1",
      });

      const result = selectWorkflowColumns(useWorkflowStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0]?.name).toBe("Backlog");
      expect(result[1]?.name).toBe("Done");
    });

    it("returns empty array when no active workflow", () => {
      useWorkflowStore.setState({
        workflows: {},
        activeWorkflowId: null,
      });

      const result = selectWorkflowColumns(useWorkflowStore.getState());

      expect(result).toEqual([]);
    });
  });

  describe("selectWorkflowById", () => {
    it("returns workflow when it exists", () => {
      const workflow = createTestWorkflow({ id: "workflow-1", name: "Found Workflow" });
      useWorkflowStore.setState({ workflows: { "workflow-1": workflow } });

      const selector = selectWorkflowById("workflow-1");
      const result = selector(useWorkflowStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Found Workflow");
    });

    it("returns undefined when workflow does not exist", () => {
      useWorkflowStore.setState({ workflows: {} });

      const selector = selectWorkflowById("nonexistent");
      const result = selector(useWorkflowStore.getState());

      expect(result).toBeUndefined();
    });
  });
});
