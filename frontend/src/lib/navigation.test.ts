import { beforeEach, describe, expect, it, vi } from "vitest";
import type { IdeationSession } from "@/types/ideation";
import { navigateToIdeationSession } from "./navigation";

// ============================================================================
// Hoisted mock factories
// ============================================================================

const { mockUiGetState, mockUiSetState, mockIdeationGetState, mockProjectGetState } =
  vi.hoisted(() => ({
    mockUiGetState: vi.fn(),
    mockUiSetState: vi.fn(),
    mockIdeationGetState: vi.fn(),
    mockProjectGetState: vi.fn(),
  }));

vi.mock("@/stores/uiStore", () => ({
  useUiStore: {
    getState: mockUiGetState,
    setState: mockUiSetState,
  },
}));

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: {
    getState: mockIdeationGetState,
  },
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: {
    getState: mockProjectGetState,
  },
}));

// ============================================================================
// Test constants & helpers
// ============================================================================

const PROJECT_A = "project-a";
const PROJECT_B = "project-b";
const SESSION_A = "session-a"; // belongs to PROJECT_A
const SESSION_B = "session-b"; // belongs to PROJECT_B

function makeSession(id: string, projectId: string): IdeationSession {
  return {
    id,
    projectId,
    title: null,
    titleSource: null,
    status: "active",
    planArtifactId: null,
    seedTaskId: null,
    parentSessionId: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    archivedAt: null,
    convertedAt: null,
    verificationStatus: "unverified",
    teamMode: null,
    teamConfig: null,
  };
}

const setCurrentViewMock = vi.fn();
const setActiveSessionMock = vi.fn();
const selectProjectMock = vi.fn();

// ============================================================================
// Tests
// ============================================================================

describe("navigateToIdeationSession", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Default: active project is PROJECT_A
    mockProjectGetState.mockReturnValue({
      activeProjectId: PROJECT_A,
      selectProject: selectProjectMock,
    });

    // Default: UI store has empty maps
    mockUiGetState.mockReturnValue({
      viewByProject: {},
      sessionByProject: {},
      setCurrentView: setCurrentViewMock,
    });

    // Default: sessions contain SESSION_A (PROJECT_A) and SESSION_B (PROJECT_B)
    mockIdeationGetState.mockReturnValue({
      sessions: {
        [SESSION_A]: makeSession(SESSION_A, PROJECT_A),
        [SESSION_B]: makeSession(SESSION_B, PROJECT_B),
      },
      setActiveSession: setActiveSessionMock,
    });
  });

  // --------------------------------------------------------------------------
  // Cross-project navigation
  // --------------------------------------------------------------------------

  describe("cross-project navigation", () => {
    it("pre-writes viewByProject[targetProjectId] = 'ideation' via useUiStore.setState", () => {
      navigateToIdeationSession(SESSION_B);

      expect(mockUiSetState).toHaveBeenCalledOnce();
      const args = mockUiSetState.mock.calls[0][0] as Record<string, unknown>;
      expect((args.viewByProject as Record<string, string>)[PROJECT_B]).toBe("ideation");
    });

    it("pre-writes sessionByProject[targetProjectId] = sessionId via useUiStore.setState", () => {
      navigateToIdeationSession(SESSION_B);

      expect(mockUiSetState).toHaveBeenCalledOnce();
      const args = mockUiSetState.mock.calls[0][0] as Record<string, unknown>;
      expect((args.sessionByProject as Record<string, string>)[PROJECT_B]).toBe(SESSION_B);
    });

    it("calls selectProject with targetProjectId", () => {
      navigateToIdeationSession(SESSION_B);

      expect(selectProjectMock).toHaveBeenCalledOnce();
      expect(selectProjectMock).toHaveBeenCalledWith(PROJECT_B);
    });

    it("does NOT call setCurrentView or setActiveSession directly", () => {
      navigateToIdeationSession(SESSION_B);

      expect(setCurrentViewMock).not.toHaveBeenCalled();
      expect(setActiveSessionMock).not.toHaveBeenCalled();
    });

    it("preserves existing viewByProject entries for other projects (spread)", () => {
      mockUiGetState.mockReturnValue({
        viewByProject: { [PROJECT_A]: "kanban", "proj-other": "graph" },
        sessionByProject: { [PROJECT_A]: SESSION_A },
        setCurrentView: setCurrentViewMock,
      });

      navigateToIdeationSession(SESSION_B);

      const args = mockUiSetState.mock.calls[0][0] as Record<string, unknown>;
      const vbp = args.viewByProject as Record<string, string>;
      expect(vbp[PROJECT_A]).toBe("kanban");
      expect(vbp["proj-other"]).toBe("graph");
      expect(vbp[PROJECT_B]).toBe("ideation");
    });

    it("preserves existing sessionByProject entries for other projects (spread)", () => {
      mockUiGetState.mockReturnValue({
        viewByProject: {},
        sessionByProject: { [PROJECT_A]: SESSION_A, "proj-other": "session-other" },
        setCurrentView: setCurrentViewMock,
      });

      navigateToIdeationSession(SESSION_B);

      const args = mockUiSetState.mock.calls[0][0] as Record<string, unknown>;
      const sbp = args.sessionByProject as Record<string, string>;
      expect(sbp[PROJECT_A]).toBe(SESSION_A);
      expect(sbp["proj-other"]).toBe("session-other");
      expect(sbp[PROJECT_B]).toBe(SESSION_B);
    });
  });

  // --------------------------------------------------------------------------
  // Same-project navigation
  // --------------------------------------------------------------------------

  describe("same-project navigation", () => {
    it("calls setCurrentView('ideation') for a session in the active project", () => {
      navigateToIdeationSession(SESSION_A);

      expect(setCurrentViewMock).toHaveBeenCalledOnce();
      expect(setCurrentViewMock).toHaveBeenCalledWith("ideation");
    });

    it("calls setActiveSession with the sessionId for a session in the active project", () => {
      navigateToIdeationSession(SESSION_A);

      expect(setActiveSessionMock).toHaveBeenCalledOnce();
      expect(setActiveSessionMock).toHaveBeenCalledWith(SESSION_A);
    });

    it("does NOT call selectProject for same-project navigation", () => {
      navigateToIdeationSession(SESSION_A);

      expect(selectProjectMock).not.toHaveBeenCalled();
    });

    it("does NOT call useUiStore.setState for same-project navigation", () => {
      navigateToIdeationSession(SESSION_A);

      expect(mockUiSetState).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // Missing session fallback
  // --------------------------------------------------------------------------

  describe("missing session fallback", () => {
    it("falls through to setCurrentView + setActiveSession when session not in store", () => {
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      navigateToIdeationSession("unknown-session");

      expect(setCurrentViewMock).toHaveBeenCalledWith("ideation");
      expect(setActiveSessionMock).toHaveBeenCalledWith("unknown-session");

      warnSpy.mockRestore();
    });

    it("emits console.warn when session not found in store", () => {
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      navigateToIdeationSession("unknown-session");

      expect(warnSpy).toHaveBeenCalledOnce();
      expect(warnSpy.mock.calls[0][0]).toContain("unknown-session");

      warnSpy.mockRestore();
    });

    it("does NOT call selectProject when session not found in store", () => {
      vi.spyOn(console, "warn").mockImplementation(() => {});

      navigateToIdeationSession("unknown-session");

      expect(selectProjectMock).not.toHaveBeenCalled();

      vi.restoreAllMocks();
    });

    it("does NOT call useUiStore.setState when session not found in store", () => {
      vi.spyOn(console, "warn").mockImplementation(() => {});

      navigateToIdeationSession("unknown-session");

      expect(mockUiSetState).not.toHaveBeenCalled();

      vi.restoreAllMocks();
    });
  });

  // --------------------------------------------------------------------------
  // No active project
  // --------------------------------------------------------------------------

  describe("no active project", () => {
    beforeEach(() => {
      mockProjectGetState.mockReturnValue({
        activeProjectId: null,
        selectProject: selectProjectMock,
      });
    });

    it("navigates directly when activeProjectId is null (session in some project)", () => {
      navigateToIdeationSession(SESSION_B);

      expect(setCurrentViewMock).toHaveBeenCalledWith("ideation");
      expect(setActiveSessionMock).toHaveBeenCalledWith(SESSION_B);
    });

    it("does NOT call selectProject when activeProjectId is null", () => {
      navigateToIdeationSession(SESSION_B);

      expect(selectProjectMock).not.toHaveBeenCalled();
    });
  });
});
