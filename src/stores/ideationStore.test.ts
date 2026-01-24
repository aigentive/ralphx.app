import { describe, it, expect, beforeEach } from "vitest";
import {
  useIdeationStore,
  selectActiveSession,
  selectSessionsByProject,
  selectSessionsByStatus,
} from "./ideationStore";
import type { IdeationSession } from "@/types/ideation";

// Helper to create test sessions
const createTestSession = (overrides: Partial<IdeationSession> = {}): IdeationSession => ({
  id: `session-${Math.random().toString(36).slice(2)}`,
  projectId: "project-1",
  title: "Test Session",
  status: "active",
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  archivedAt: null,
  convertedAt: null,
  ...overrides,
});

describe("ideationStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useIdeationStore.setState({
      sessions: {},
      activeSessionId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("initial state", () => {
    it("has empty sessions", () => {
      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(0);
    });

    it("has null activeSessionId", () => {
      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBeNull();
    });

    it("has isLoading false", () => {
      const state = useIdeationStore.getState();
      expect(state.isLoading).toBe(false);
    });

    it("has null error", () => {
      const state = useIdeationStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe("setActiveSession", () => {
    it("sets activeSessionId", () => {
      useIdeationStore.getState().setActiveSession("session-1");

      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBe("session-1");
    });

    it("sets activeSessionId to null", () => {
      useIdeationStore.setState({ activeSessionId: "session-1" });

      useIdeationStore.getState().setActiveSession(null);

      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBeNull();
    });

    it("replaces previous activeSessionId", () => {
      useIdeationStore.setState({ activeSessionId: "session-1" });

      useIdeationStore.getState().setActiveSession("session-2");

      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBe("session-2");
    });
  });

  describe("addSession", () => {
    it("adds a session to the store", () => {
      const session = createTestSession({ id: "session-1" });

      useIdeationStore.getState().addSession(session);

      const state = useIdeationStore.getState();
      expect(state.sessions["session-1"]).toBeDefined();
      expect(state.sessions["session-1"]?.title).toBe("Test Session");
    });

    it("overwrites session with same id", () => {
      const session1 = createTestSession({ id: "session-1", title: "First" });
      const session2 = createTestSession({ id: "session-1", title: "Second" });

      useIdeationStore.getState().addSession(session1);
      useIdeationStore.getState().addSession(session2);

      const state = useIdeationStore.getState();
      expect(state.sessions["session-1"]?.title).toBe("Second");
    });

    it("preserves existing sessions", () => {
      const session1 = createTestSession({ id: "session-1" });
      const session2 = createTestSession({ id: "session-2" });

      useIdeationStore.getState().addSession(session1);
      useIdeationStore.getState().addSession(session2);

      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(2);
    });
  });

  describe("setSessions", () => {
    it("converts array to Record keyed by id", () => {
      const sessions = [
        createTestSession({ id: "session-1", title: "Session 1" }),
        createTestSession({ id: "session-2", title: "Session 2" }),
        createTestSession({ id: "session-3", title: "Session 3" }),
      ];

      useIdeationStore.getState().setSessions(sessions);

      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(3);
      expect(state.sessions["session-1"]?.title).toBe("Session 1");
      expect(state.sessions["session-2"]?.title).toBe("Session 2");
      expect(state.sessions["session-3"]?.title).toBe("Session 3");
    });

    it("replaces existing sessions", () => {
      useIdeationStore.setState({
        sessions: {
          "old-session": createTestSession({ id: "old-session", title: "Old" }),
        },
      });

      const newSessions = [createTestSession({ id: "new-session", title: "New" })];
      useIdeationStore.getState().setSessions(newSessions);

      const state = useIdeationStore.getState();
      expect(state.sessions["old-session"]).toBeUndefined();
      expect(state.sessions["new-session"]?.title).toBe("New");
    });

    it("handles empty array", () => {
      useIdeationStore.getState().setSessions([]);

      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(0);
    });
  });

  describe("updateSession", () => {
    it("modifies existing session", () => {
      const session = createTestSession({ id: "session-1", title: "Original" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().updateSession("session-1", { title: "Updated" });

      const state = useIdeationStore.getState();
      expect(state.sessions["session-1"]?.title).toBe("Updated");
    });

    it("updates multiple fields", () => {
      const session = createTestSession({
        id: "session-1",
        title: "Original",
        status: "active",
      });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().updateSession("session-1", {
        title: "Updated",
        status: "archived",
        archivedAt: "2026-01-24T14:00:00Z",
      });

      const state = useIdeationStore.getState();
      const updated = state.sessions["session-1"];
      expect(updated?.title).toBe("Updated");
      expect(updated?.status).toBe("archived");
      expect(updated?.archivedAt).toBe("2026-01-24T14:00:00Z");
    });

    it("does nothing if session not found", () => {
      const session = createTestSession({ id: "session-1" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().updateSession("nonexistent", { title: "Updated" });

      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(1);
      expect(state.sessions["session-1"]?.title).toBe("Test Session");
    });

    it("preserves other session fields", () => {
      const session = createTestSession({
        id: "session-1",
        title: "Original",
        projectId: "project-1",
        status: "active",
      });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().updateSession("session-1", { title: "Updated" });

      const state = useIdeationStore.getState();
      const updated = state.sessions["session-1"];
      expect(updated?.title).toBe("Updated");
      expect(updated?.projectId).toBe("project-1");
      expect(updated?.status).toBe("active");
    });
  });

  describe("removeSession", () => {
    it("removes a session from the store", () => {
      const session = createTestSession({ id: "session-1" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().removeSession("session-1");

      const state = useIdeationStore.getState();
      expect(state.sessions["session-1"]).toBeUndefined();
    });

    it("clears activeSessionId if active session is removed", () => {
      const session = createTestSession({ id: "session-1" });
      useIdeationStore.setState({
        sessions: { "session-1": session },
        activeSessionId: "session-1",
      });

      useIdeationStore.getState().removeSession("session-1");

      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBeNull();
    });

    it("does not affect activeSessionId if different session is removed", () => {
      const session1 = createTestSession({ id: "session-1" });
      const session2 = createTestSession({ id: "session-2" });
      useIdeationStore.setState({
        sessions: { "session-1": session1, "session-2": session2 },
        activeSessionId: "session-1",
      });

      useIdeationStore.getState().removeSession("session-2");

      const state = useIdeationStore.getState();
      expect(state.activeSessionId).toBe("session-1");
    });

    it("does nothing if session not found", () => {
      const session = createTestSession({ id: "session-1" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      useIdeationStore.getState().removeSession("nonexistent");

      const state = useIdeationStore.getState();
      expect(Object.keys(state.sessions)).toHaveLength(1);
    });
  });

  describe("setLoading", () => {
    it("sets isLoading to true", () => {
      useIdeationStore.getState().setLoading(true);

      const state = useIdeationStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets isLoading to false", () => {
      useIdeationStore.setState({ isLoading: true });

      useIdeationStore.getState().setLoading(false);

      const state = useIdeationStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useIdeationStore.getState().setError("Something went wrong");

      const state = useIdeationStore.getState();
      expect(state.error).toBe("Something went wrong");
    });

    it("clears error with null", () => {
      useIdeationStore.setState({ error: "Previous error" });

      useIdeationStore.getState().setError(null);

      const state = useIdeationStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe("clearError", () => {
    it("clears existing error", () => {
      useIdeationStore.setState({ error: "Some error" });

      useIdeationStore.getState().clearError();

      const state = useIdeationStore.getState();
      expect(state.error).toBeNull();
    });

    it("does nothing if no error", () => {
      useIdeationStore.getState().clearError();

      const state = useIdeationStore.getState();
      expect(state.error).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useIdeationStore.setState({
      sessions: {},
      activeSessionId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("selectActiveSession", () => {
    it("returns active session when it exists", () => {
      const session = createTestSession({ id: "session-1", title: "Active Session" });
      useIdeationStore.setState({
        sessions: { "session-1": session },
        activeSessionId: "session-1",
      });

      const result = selectActiveSession(useIdeationStore.getState());

      expect(result).not.toBeNull();
      expect(result?.title).toBe("Active Session");
    });

    it("returns null when no session is active", () => {
      const session = createTestSession({ id: "session-1" });
      useIdeationStore.setState({
        sessions: { "session-1": session },
        activeSessionId: null,
      });

      const result = selectActiveSession(useIdeationStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when active session does not exist", () => {
      useIdeationStore.setState({
        sessions: {},
        activeSessionId: "nonexistent",
      });

      const result = selectActiveSession(useIdeationStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectSessionsByProject", () => {
    it("returns sessions for matching project", () => {
      const sessions = [
        createTestSession({ id: "session-1", projectId: "project-1" }),
        createTestSession({ id: "session-2", projectId: "project-2" }),
        createTestSession({ id: "session-3", projectId: "project-1" }),
      ];
      useIdeationStore.getState().setSessions(sessions);

      const selector = selectSessionsByProject("project-1");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((s) => s.id).sort()).toEqual(["session-1", "session-3"]);
    });

    it("returns empty array when no sessions match", () => {
      const session = createTestSession({ id: "session-1", projectId: "project-1" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      const selector = selectSessionsByProject("project-2");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(0);
    });

    it("returns empty array when store is empty", () => {
      const selector = selectSessionsByProject("project-1");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectSessionsByStatus", () => {
    it("returns sessions with matching status", () => {
      const sessions = [
        createTestSession({ id: "session-1", status: "active" }),
        createTestSession({ id: "session-2", status: "archived" }),
        createTestSession({ id: "session-3", status: "active" }),
      ];
      useIdeationStore.getState().setSessions(sessions);

      const selector = selectSessionsByStatus("active");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((s) => s.id).sort()).toEqual(["session-1", "session-3"]);
    });

    it("returns empty array when no sessions match", () => {
      const session = createTestSession({ id: "session-1", status: "active" });
      useIdeationStore.setState({ sessions: { "session-1": session } });

      const selector = selectSessionsByStatus("converted");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(0);
    });

    it("returns empty array when store is empty", () => {
      const selector = selectSessionsByStatus("active");
      const result = selector(useIdeationStore.getState());

      expect(result).toHaveLength(0);
    });
  });
});
