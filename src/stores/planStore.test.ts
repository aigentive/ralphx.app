import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  usePlanStore,
  selectActivePlanId,
  selectCurrentActivePlan,
  type PlanCandidate,
} from "./planStore";
import { planApi } from "@/api/plan";

// Mock the planApi module
vi.mock("@/api/plan", () => ({
  planApi: {
    getActivePlan: vi.fn(),
    setActivePlan: vi.fn(),
    clearActivePlan: vi.fn(),
  },
}));

const mockPlanApi = planApi as {
  getActivePlan: ReturnType<typeof vi.fn>;
  setActivePlan: ReturnType<typeof vi.fn>;
  clearActivePlan: ReturnType<typeof vi.fn>;
};

// Helper to create test plan candidates
const createTestCandidate = (
  overrides: Partial<PlanCandidate> = {}
): PlanCandidate => ({
  sessionId: `session-${Math.random().toString(36).slice(2)}`,
  title: "Test Plan",
  acceptedAt: "2026-01-24T12:00:00Z",
  taskStats: {
    total: 10,
    incomplete: 5,
    activeNow: 2,
  },
  interactionStats: {
    selectedCount: 3,
    lastSelectedAt: "2026-01-24T12:00:00Z",
  },
  score: 0.75,
  ...overrides,
});

describe("planStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    usePlanStore.setState({
      activePlanByProject: {},
      planCandidates: [],
      isLoading: false,
      error: null,
    });
    // Clear all mocks
    mockPlanApi.getActivePlan.mockReset();
    mockPlanApi.setActivePlan.mockReset();
    mockPlanApi.clearActivePlan.mockReset();
  });

  describe("loadActivePlan", () => {
    it("loads active plan from backend and updates state", async () => {
      mockPlanApi.getActivePlan.mockResolvedValue("session-123");

      await usePlanStore.getState().loadActivePlan("project-1");

      expect(mockPlanApi.getActivePlan).toHaveBeenCalledWith("project-1");
      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBe("session-123");
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it("handles null active plan (no plan set)", async () => {
      mockPlanApi.getActivePlan.mockResolvedValue(null);

      await usePlanStore.getState().loadActivePlan("project-1");

      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBeNull();
      expect(state.isLoading).toBe(false);
    });

    it("sets isLoading to true during load", async () => {
      let resolveLoad: (value: string) => void;
      mockPlanApi.getActivePlan.mockReturnValue(
        new Promise<string>((resolve) => {
          resolveLoad = resolve;
        })
      );

      const loadPromise = usePlanStore.getState().loadActivePlan("project-1");

      // Check loading state before resolution
      expect(usePlanStore.getState().isLoading).toBe(true);

      resolveLoad!("session-123");
      await loadPromise;

      expect(usePlanStore.getState().isLoading).toBe(false);
    });

    it("handles errors from backend", async () => {
      mockPlanApi.getActivePlan.mockRejectedValue(
        new Error("Backend error")
      );

      await usePlanStore.getState().loadActivePlan("project-1");

      const state = usePlanStore.getState();
      expect(state.error).toBe("Backend error");
      expect(state.isLoading).toBe(false);
    });

    it("handles multiple projects independently", async () => {
      mockPlanApi.getActivePlan
        .mockResolvedValueOnce("session-1")
        .mockResolvedValueOnce("session-2");

      await usePlanStore.getState().loadActivePlan("project-1");
      await usePlanStore.getState().loadActivePlan("project-2");

      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBe("session-1");
      expect(state.activePlanByProject["project-2"]).toBe("session-2");
    });
  });

  describe("setActivePlan", () => {
    it("sets active plan and updates state", async () => {
      mockPlanApi.setActivePlan.mockResolvedValue(undefined);

      await usePlanStore
        .getState()
        .setActivePlan("project-1", "session-123", "kanban_inline");

      expect(mockPlanApi.setActivePlan).toHaveBeenCalledWith(
        "project-1",
        "session-123",
        "kanban_inline"
      );
      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBe("session-123");
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it("tracks different selection sources", async () => {
      mockPlanApi.setActivePlan.mockResolvedValue(undefined);

      await usePlanStore
        .getState()
        .setActivePlan("project-1", "session-1", "graph_inline");
      await usePlanStore
        .getState()
        .setActivePlan("project-2", "session-2", "quick_switcher");
      await usePlanStore
        .getState()
        .setActivePlan("project-3", "session-3", "ideation");

      expect(mockPlanApi.setActivePlan).toHaveBeenNthCalledWith(
        1,
        "project-1",
        "session-1",
        "graph_inline"
      );
      expect(mockPlanApi.setActivePlan).toHaveBeenNthCalledWith(
        2,
        "project-2",
        "session-2",
        "quick_switcher"
      );
      expect(mockPlanApi.setActivePlan).toHaveBeenNthCalledWith(
        3,
        "project-3",
        "session-3",
        "ideation"
      );
    });

    it("replaces existing active plan", async () => {
      mockPlanApi.setActivePlan.mockResolvedValue(undefined);

      // Set initial plan
      await usePlanStore
        .getState()
        .setActivePlan("project-1", "session-1", "kanban_inline");

      // Replace with new plan
      await usePlanStore
        .getState()
        .setActivePlan("project-1", "session-2", "graph_inline");

      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBe("session-2");
    });

    it("handles errors from backend and re-throws", async () => {
      mockPlanApi.setActivePlan.mockRejectedValue(
        new Error("Session not found")
      );

      await expect(
        usePlanStore
          .getState()
          .setActivePlan("project-1", "invalid", "kanban_inline")
      ).rejects.toThrow("Session not found");

      const state = usePlanStore.getState();
      expect(state.error).toBe("Session not found");
      expect(state.isLoading).toBe(false);
    });

    it("clears previous errors on successful set", async () => {
      mockPlanApi.setActivePlan
        .mockRejectedValueOnce(new Error("First error"))
        .mockResolvedValueOnce(undefined);

      // First call fails
      await expect(
        usePlanStore
          .getState()
          .setActivePlan("project-1", "invalid", "kanban_inline")
      ).rejects.toThrow();
      expect(usePlanStore.getState().error).toBe("First error");

      // Second call succeeds
      await usePlanStore
        .getState()
        .setActivePlan("project-1", "session-123", "kanban_inline");

      const state = usePlanStore.getState();
      expect(state.error).toBeNull();
      expect(state.activePlanByProject["project-1"]).toBe("session-123");
    });
  });

  describe("clearActivePlan", () => {
    it("clears active plan from backend and state", async () => {
      mockPlanApi.clearActivePlan.mockResolvedValue(undefined);

      // Set a plan first
      usePlanStore.setState({
        activePlanByProject: { "project-1": "session-123" },
      });

      await usePlanStore.getState().clearActivePlan("project-1");

      expect(mockPlanApi.clearActivePlan).toHaveBeenCalledWith("project-1");
      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it("handles clearing when no plan is set", async () => {
      mockPlanApi.clearActivePlan.mockResolvedValue(undefined);

      await usePlanStore.getState().clearActivePlan("project-1");

      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBeNull();
    });

    it("only clears the specified project", async () => {
      mockPlanApi.clearActivePlan.mockResolvedValue(undefined);

      usePlanStore.setState({
        activePlanByProject: {
          "project-1": "session-1",
          "project-2": "session-2",
        },
      });

      await usePlanStore.getState().clearActivePlan("project-1");

      const state = usePlanStore.getState();
      expect(state.activePlanByProject["project-1"]).toBeNull();
      expect(state.activePlanByProject["project-2"]).toBe("session-2");
    });

    it("handles errors from backend and re-throws", async () => {
      mockPlanApi.clearActivePlan.mockRejectedValue(
        new Error("Clear failed")
      );

      await expect(
        usePlanStore.getState().clearActivePlan("project-1")
      ).rejects.toThrow("Clear failed");

      const state = usePlanStore.getState();
      expect(state.error).toBe("Clear failed");
      expect(state.isLoading).toBe(false);
    });
  });

  describe("loadCandidates", () => {
    it("sets loading state and placeholder implementation", async () => {
      await usePlanStore.getState().loadCandidates("project-1");

      const state = usePlanStore.getState();
      expect(state.planCandidates).toEqual([]);
      expect(state.isLoading).toBe(false);
    });

    it("accepts optional query parameter", async () => {
      await usePlanStore.getState().loadCandidates("project-1", "search term");

      const state = usePlanStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("planCandidates state", () => {
    it("can be set directly for testing", () => {
      const candidates = [
        createTestCandidate({ sessionId: "session-1", title: "Plan 1" }),
        createTestCandidate({ sessionId: "session-2", title: "Plan 2" }),
      ];

      usePlanStore.setState({ planCandidates: candidates });

      const state = usePlanStore.getState();
      expect(state.planCandidates).toHaveLength(2);
      expect(state.planCandidates[0]?.title).toBe("Plan 1");
      expect(state.planCandidates[1]?.title).toBe("Plan 2");
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    usePlanStore.setState({
      activePlanByProject: {},
      planCandidates: [],
      isLoading: false,
      error: null,
    });
  });

  describe("selectActivePlanId", () => {
    it("returns active plan ID when set", () => {
      usePlanStore.setState({
        activePlanByProject: { "project-1": "session-123" },
      });

      const selector = selectActivePlanId("project-1");
      const result = selector(usePlanStore.getState());

      expect(result).toBe("session-123");
    });

    it("returns null when no plan is set", () => {
      const selector = selectActivePlanId("project-1");
      const result = selector(usePlanStore.getState());

      expect(result).toBeNull();
    });

    it("returns null for non-existent project", () => {
      usePlanStore.setState({
        activePlanByProject: { "project-1": "session-123" },
      });

      const selector = selectActivePlanId("nonexistent");
      const result = selector(usePlanStore.getState());

      expect(result).toBeNull();
    });

    it("handles multiple projects", () => {
      usePlanStore.setState({
        activePlanByProject: {
          "project-1": "session-1",
          "project-2": "session-2",
        },
      });

      const selector1 = selectActivePlanId("project-1");
      const selector2 = selectActivePlanId("project-2");

      expect(selector1(usePlanStore.getState())).toBe("session-1");
      expect(selector2(usePlanStore.getState())).toBe("session-2");
    });
  });

  describe("selectCurrentActivePlan", () => {
    it("returns active plan for current project", () => {
      const state = {
        ...usePlanStore.getState(),
        activeProjectId: "project-1",
        activePlanByProject: { "project-1": "session-123" },
      };

      const result = selectCurrentActivePlan(state);

      expect(result).toBe("session-123");
    });

    it("returns null when no project is active", () => {
      const state = {
        ...usePlanStore.getState(),
        activeProjectId: null,
        activePlanByProject: { "project-1": "session-123" },
      };

      const result = selectCurrentActivePlan(state);

      expect(result).toBeNull();
    });

    it("returns null when active project has no plan", () => {
      const state = {
        ...usePlanStore.getState(),
        activeProjectId: "project-1",
        activePlanByProject: {},
      };

      const result = selectCurrentActivePlan(state);

      expect(result).toBeNull();
    });

    it("returns null when active project plan is explicitly null", () => {
      const state = {
        ...usePlanStore.getState(),
        activeProjectId: "project-1",
        activePlanByProject: { "project-1": null },
      };

      const result = selectCurrentActivePlan(state);

      expect(result).toBeNull();
    });
  });
});
