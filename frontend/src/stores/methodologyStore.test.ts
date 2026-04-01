import { describe, it, expect, beforeEach } from "vitest";
import {
  useMethodologyStore,
  selectActiveMethodology,
  selectMethodologyById,
  selectMethodologyPhases,
} from "./methodologyStore";
import type { Methodology } from "./methodologyStore";

// Helper to create test methodologies
const createTestMethodology = (overrides: Partial<Methodology> = {}): Methodology => ({
  id: `methodology-${Math.random().toString(36).slice(2)}`,
  name: "Test Methodology",
  description: "A test methodology",
  agentProfiles: ["agent-1", "agent-2"],
  skills: ["skill-1"],
  workflowId: "workflow-1",
  workflowName: "Test Workflow",
  phases: [
    {
      id: "phase-1",
      name: "Phase 1",
      order: 0,
      description: "First phase",
      agentProfiles: ["agent-1"],
      columnIds: ["col-1"],
    },
  ],
  templates: [],
  isActive: false,
  phaseCount: 1,
  agentCount: 2,
  createdAt: new Date().toISOString(),
  ...overrides,
});

describe("methodologyStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useMethodologyStore.setState({
      methodologies: {},
      activeMethodologyId: null,
      isLoading: false,
      error: null,
      isActivating: false,
    });
  });

  describe("setMethodologies", () => {
    it("converts array to Record keyed by id", () => {
      const methodologies = [
        createTestMethodology({ id: "method-1", name: "Method 1" }),
        createTestMethodology({ id: "method-2", name: "Method 2" }),
      ];

      useMethodologyStore.getState().setMethodologies(methodologies);

      const state = useMethodologyStore.getState();
      expect(Object.keys(state.methodologies)).toHaveLength(2);
      expect(state.methodologies["method-1"]?.name).toBe("Method 1");
      expect(state.methodologies["method-2"]?.name).toBe("Method 2");
    });

    it("replaces existing methodologies", () => {
      useMethodologyStore.setState({
        methodologies: {
          "old-method": createTestMethodology({ id: "old-method", name: "Old" }),
        },
      });

      const newMethodologies = [createTestMethodology({ id: "new-method", name: "New" })];
      useMethodologyStore.getState().setMethodologies(newMethodologies);

      const state = useMethodologyStore.getState();
      expect(state.methodologies["old-method"]).toBeUndefined();
      expect(state.methodologies["new-method"]?.name).toBe("New");
    });

    it("handles empty array", () => {
      useMethodologyStore.getState().setMethodologies([]);

      const state = useMethodologyStore.getState();
      expect(Object.keys(state.methodologies)).toHaveLength(0);
    });

    it("sets activeMethodologyId if a methodology is active", () => {
      const methodologies = [
        createTestMethodology({ id: "method-1", isActive: false }),
        createTestMethodology({ id: "method-2", isActive: true }),
      ];

      useMethodologyStore.getState().setMethodologies(methodologies);

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-2");
    });

    it("clears activeMethodologyId if no methodology is active", () => {
      useMethodologyStore.setState({ activeMethodologyId: "old-active" });

      const methodologies = [
        createTestMethodology({ id: "method-1", isActive: false }),
        createTestMethodology({ id: "method-2", isActive: false }),
      ];

      useMethodologyStore.getState().setMethodologies(methodologies);

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBeNull();
    });
  });

  describe("setActiveMethodology", () => {
    it("updates activeMethodologyId", () => {
      const methodology = createTestMethodology({ id: "method-1" });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().setActiveMethodology("method-1");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-1");
    });

    it("sets activeMethodologyId to null", () => {
      useMethodologyStore.setState({ activeMethodologyId: "method-1" });

      useMethodologyStore.getState().setActiveMethodology(null);

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBeNull();
    });

    it("replaces previous active methodology", () => {
      useMethodologyStore.setState({ activeMethodologyId: "method-1" });

      useMethodologyStore.getState().setActiveMethodology("method-2");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-2");
    });
  });

  describe("activateMethodology", () => {
    it("marks methodology as active and sets activeMethodologyId", () => {
      const method1 = createTestMethodology({ id: "method-1", isActive: false });
      const method2 = createTestMethodology({ id: "method-2", isActive: false });
      useMethodologyStore.setState({
        methodologies: { "method-1": method1, "method-2": method2 },
      });

      useMethodologyStore.getState().activateMethodology("method-1");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-1");
      expect(state.methodologies["method-1"]?.isActive).toBe(true);
    });

    it("deactivates previously active methodology", () => {
      const method1 = createTestMethodology({ id: "method-1", isActive: true });
      const method2 = createTestMethodology({ id: "method-2", isActive: false });
      useMethodologyStore.setState({
        methodologies: { "method-1": method1, "method-2": method2 },
        activeMethodologyId: "method-1",
      });

      useMethodologyStore.getState().activateMethodology("method-2");

      const state = useMethodologyStore.getState();
      expect(state.methodologies["method-1"]?.isActive).toBe(false);
      expect(state.methodologies["method-2"]?.isActive).toBe(true);
      expect(state.activeMethodologyId).toBe("method-2");
    });

    it("does nothing if methodology not found", () => {
      const methodology = createTestMethodology({ id: "method-1", isActive: false });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().activateMethodology("nonexistent");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBeNull();
      expect(state.methodologies["method-1"]?.isActive).toBe(false);
    });
  });

  describe("deactivateMethodology", () => {
    it("marks methodology as inactive and clears activeMethodologyId", () => {
      const methodology = createTestMethodology({ id: "method-1", isActive: true });
      useMethodologyStore.setState({
        methodologies: { "method-1": methodology },
        activeMethodologyId: "method-1",
      });

      useMethodologyStore.getState().deactivateMethodology("method-1");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBeNull();
      expect(state.methodologies["method-1"]?.isActive).toBe(false);
    });

    it("does nothing if methodology not found", () => {
      const methodology = createTestMethodology({ id: "method-1", isActive: true });
      useMethodologyStore.setState({
        methodologies: { "method-1": methodology },
        activeMethodologyId: "method-1",
      });

      useMethodologyStore.getState().deactivateMethodology("nonexistent");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-1");
      expect(state.methodologies["method-1"]?.isActive).toBe(true);
    });

    it("only clears activeMethodologyId if it matches the deactivated methodology", () => {
      const method1 = createTestMethodology({ id: "method-1", isActive: false });
      const method2 = createTestMethodology({ id: "method-2", isActive: true });
      useMethodologyStore.setState({
        methodologies: { "method-1": method1, "method-2": method2 },
        activeMethodologyId: "method-2",
      });

      useMethodologyStore.getState().deactivateMethodology("method-1");

      const state = useMethodologyStore.getState();
      expect(state.activeMethodologyId).toBe("method-2");
      expect(state.methodologies["method-1"]?.isActive).toBe(false);
    });
  });

  describe("updateMethodology", () => {
    it("modifies existing methodology", () => {
      const methodology = createTestMethodology({ id: "method-1", name: "Original" });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().updateMethodology("method-1", { name: "Updated" });

      const state = useMethodologyStore.getState();
      expect(state.methodologies["method-1"]?.name).toBe("Updated");
    });

    it("updates multiple fields", () => {
      const methodology = createTestMethodology({
        id: "method-1",
        name: "Original",
        description: "Original desc",
      });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().updateMethodology("method-1", {
        name: "Updated",
        description: "Updated desc",
      });

      const state = useMethodologyStore.getState();
      expect(state.methodologies["method-1"]?.name).toBe("Updated");
      expect(state.methodologies["method-1"]?.description).toBe("Updated desc");
    });

    it("does nothing if methodology not found", () => {
      const methodology = createTestMethodology({ id: "method-1" });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().updateMethodology("nonexistent", { name: "Updated" });

      const state = useMethodologyStore.getState();
      expect(Object.keys(state.methodologies)).toHaveLength(1);
      expect(state.methodologies["method-1"]?.name).toBe("Test Methodology");
    });

    it("preserves other methodology fields", () => {
      const methodology = createTestMethodology({
        id: "method-1",
        name: "Original",
        description: "A description",
        agentProfiles: ["agent-1", "agent-2"],
      });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      useMethodologyStore.getState().updateMethodology("method-1", { name: "Updated" });

      const state = useMethodologyStore.getState();
      expect(state.methodologies["method-1"]?.name).toBe("Updated");
      expect(state.methodologies["method-1"]?.description).toBe("A description");
      expect(state.methodologies["method-1"]?.agentProfiles).toHaveLength(2);
    });
  });

  describe("setLoading", () => {
    it("sets loading state to true", () => {
      useMethodologyStore.getState().setLoading(true);

      const state = useMethodologyStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets loading state to false", () => {
      useMethodologyStore.setState({ isLoading: true });

      useMethodologyStore.getState().setLoading(false);

      const state = useMethodologyStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setActivating", () => {
    it("sets activating state to true", () => {
      useMethodologyStore.getState().setActivating(true);

      const state = useMethodologyStore.getState();
      expect(state.isActivating).toBe(true);
    });

    it("sets activating state to false", () => {
      useMethodologyStore.setState({ isActivating: true });

      useMethodologyStore.getState().setActivating(false);

      const state = useMethodologyStore.getState();
      expect(state.isActivating).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useMethodologyStore.getState().setError("Something went wrong");

      const state = useMethodologyStore.getState();
      expect(state.error).toBe("Something went wrong");
    });

    it("clears error with null", () => {
      useMethodologyStore.setState({ error: "Previous error" });

      useMethodologyStore.getState().setError(null);

      const state = useMethodologyStore.getState();
      expect(state.error).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useMethodologyStore.setState({
      methodologies: {},
      activeMethodologyId: null,
      isLoading: false,
      error: null,
      isActivating: false,
    });
  });

  describe("selectActiveMethodology", () => {
    it("returns active methodology when it exists", () => {
      const methodology = createTestMethodology({ id: "method-1", name: "Active Method" });
      useMethodologyStore.setState({
        methodologies: { "method-1": methodology },
        activeMethodologyId: "method-1",
      });

      const result = selectActiveMethodology(useMethodologyStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Active Method");
    });

    it("returns null when no methodology is active", () => {
      const methodology = createTestMethodology({ id: "method-1" });
      useMethodologyStore.setState({
        methodologies: { "method-1": methodology },
        activeMethodologyId: null,
      });

      const result = selectActiveMethodology(useMethodologyStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when active methodology does not exist", () => {
      useMethodologyStore.setState({
        methodologies: {},
        activeMethodologyId: "nonexistent",
      });

      const result = selectActiveMethodology(useMethodologyStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectMethodologyById", () => {
    it("returns methodology when it exists", () => {
      const methodology = createTestMethodology({ id: "method-1", name: "Found Method" });
      useMethodologyStore.setState({ methodologies: { "method-1": methodology } });

      const selector = selectMethodologyById("method-1");
      const result = selector(useMethodologyStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Found Method");
    });

    it("returns undefined when methodology does not exist", () => {
      useMethodologyStore.setState({ methodologies: {} });

      const selector = selectMethodologyById("nonexistent");
      const result = selector(useMethodologyStore.getState());

      expect(result).toBeUndefined();
    });
  });

  describe("selectMethodologyPhases", () => {
    it("returns phases for active methodology", () => {
      const phases = [
        {
          id: "phase-1",
          name: "Phase 1",
          order: 0,
          description: "First phase",
          agentProfiles: ["agent-1"],
          columnIds: ["col-1"],
        },
        {
          id: "phase-2",
          name: "Phase 2",
          order: 1,
          description: "Second phase",
          agentProfiles: ["agent-2"],
          columnIds: ["col-2"],
        },
      ];
      const methodology = createTestMethodology({ id: "method-1", phases });
      useMethodologyStore.setState({
        methodologies: { "method-1": methodology },
        activeMethodologyId: "method-1",
      });

      const result = selectMethodologyPhases(useMethodologyStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0]?.name).toBe("Phase 1");
      expect(result[1]?.name).toBe("Phase 2");
    });

    it("returns empty array when no active methodology", () => {
      useMethodologyStore.setState({
        methodologies: {},
        activeMethodologyId: null,
      });

      const result = selectMethodologyPhases(useMethodologyStore.getState());

      expect(result).toEqual([]);
    });
  });
});
