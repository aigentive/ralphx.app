import { describe, it, expect } from "vitest";
import type { IdeationSession } from "@/types/ideation";
import type { SessionProgress } from "@/hooks/useSessionProgress";
import { groupSessions } from "./planBrowserUtils";

const createSession = (overrides: Partial<IdeationSession> = {}): IdeationSession => ({
  id: "session-1",
  projectId: "project-1",
  title: "Test Session",
  status: "active",
  planArtifactId: null,
  seedTaskId: null,
  parentSessionId: null,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  archivedAt: null,
  convertedAt: null,
  ...overrides,
});

const createProgress = (overrides: Partial<SessionProgress> = {}): SessionProgress => ({
  idle: 0,
  active: 0,
  done: 0,
  total: 0,
  ...overrides,
});

describe("planBrowserUtils", () => {
  describe("groupSessions", () => {
    it("should classify active sessions as drafts", () => {
      const sessions = [createSession({ id: "s1", status: "active" })];
      const result = groupSessions(sessions, new Map());

      expect(result.drafts).toHaveLength(1);
      expect(result.drafts[0].id).toBe("s1");
      expect(result["in-progress"]).toHaveLength(0);
      expect(result.accepted).toHaveLength(0);
      expect(result.done).toHaveLength(0);
      expect(result.archived).toHaveLength(0);
    });

    it("should classify archived sessions as archived", () => {
      const sessions = [createSession({ id: "s1", status: "archived" })];
      const result = groupSessions(sessions, new Map());

      expect(result.archived).toHaveLength(1);
      expect(result.archived[0].id).toBe("s1");
      expect(result.drafts).toHaveLength(0);
    });

    it("should classify accepted sessions with all terminal tasks (total > 0) as done", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ done: 5, total: 5 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      expect(result.done).toHaveLength(1);
      expect(result.done[0].id).toBe("s1");
      expect(result["in-progress"]).toHaveLength(0);
      expect(result.accepted).toHaveLength(0);
    });

    it("should classify accepted sessions with active tasks as in-progress", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ idle: 2, active: 1, done: 3, total: 6 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      expect(result["in-progress"]).toHaveLength(1);
      expect(result["in-progress"][0].id).toBe("s1");
      expect(result.done).toHaveLength(0);
      expect(result.accepted).toHaveLength(0);
    });

    it("should classify accepted sessions with 0 tasks as accepted", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ total: 0 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      expect(result.accepted).toHaveLength(1);
      expect(result.accepted[0].id).toBe("s1");
    });

    it("should classify accepted sessions with all idle tasks (no active) as accepted", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ idle: 3, active: 0, done: 0, total: 3 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      expect(result.accepted).toHaveLength(1);
      expect(result.accepted[0].id).toBe("s1");
    });

    it("should classify accepted sessions with no progress data as accepted", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>();

      const result = groupSessions(sessions, progressMap);

      expect(result.accepted).toHaveLength(1);
      expect(result.accepted[0].id).toBe("s1");
    });

    it("should sort each group by updatedAt descending", () => {
      const sessions = [
        createSession({ id: "s1", status: "active", updatedAt: "2026-01-20T12:00:00Z" }),
        createSession({ id: "s2", status: "active", updatedAt: "2026-01-25T12:00:00Z" }),
        createSession({ id: "s3", status: "active", updatedAt: "2026-01-22T12:00:00Z" }),
      ];

      const result = groupSessions(sessions, new Map());

      expect(result.drafts.map((s) => s.id)).toEqual(["s2", "s3", "s1"]);
    });

    it("should sort multiple groups independently", () => {
      const sessions = [
        createSession({ id: "s1", status: "active", updatedAt: "2026-01-20T12:00:00Z" }),
        createSession({ id: "s2", status: "active", updatedAt: "2026-01-25T12:00:00Z" }),
        createSession({ id: "s3", status: "archived", updatedAt: "2026-01-22T12:00:00Z" }),
        createSession({ id: "s4", status: "archived", updatedAt: "2026-01-24T12:00:00Z" }),
      ];

      const result = groupSessions(sessions, new Map());

      expect(result.drafts.map((s) => s.id)).toEqual(["s2", "s1"]);
      expect(result.archived.map((s) => s.id)).toEqual(["s4", "s3"]);
    });

    it("should handle a mix of all five groups", () => {
      const sessions = [
        createSession({ id: "draft-1", status: "active" }),
        createSession({ id: "accepted-1", status: "accepted" }),
        createSession({ id: "in-progress-1", status: "accepted" }),
        createSession({ id: "done-1", status: "accepted" }),
        createSession({ id: "archived-1", status: "archived" }),
      ];
      const progressMap = new Map<string, SessionProgress>([
        ["accepted-1", createProgress({ idle: 2, total: 2 })],
        ["in-progress-1", createProgress({ active: 1, idle: 1, total: 2 })],
        ["done-1", createProgress({ done: 3, total: 3 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      expect(result.drafts.map((s) => s.id)).toEqual(["draft-1"]);
      expect(result["in-progress"].map((s) => s.id)).toEqual(["in-progress-1"]);
      expect(result.accepted.map((s) => s.id)).toEqual(["accepted-1"]);
      expect(result.done.map((s) => s.id)).toEqual(["done-1"]);
      expect(result.archived.map((s) => s.id)).toEqual(["archived-1"]);
    });

    it("should return empty arrays for all groups when no sessions", () => {
      const result = groupSessions([], new Map());

      expect(result.drafts).toHaveLength(0);
      expect(result["in-progress"]).toHaveLength(0);
      expect(result.accepted).toHaveLength(0);
      expect(result.done).toHaveLength(0);
      expect(result.archived).toHaveLength(0);
    });

    it("should handle accepted sessions with idle + done tasks (no active) as accepted", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ idle: 2, done: 1, total: 3 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      // Has idle tasks and no active tasks, but not all terminal → accepted
      expect(result.accepted).toHaveLength(1);
    });

    it("should not count idle-only + done sessions as done if idle > 0", () => {
      const sessions = [createSession({ id: "s1", status: "accepted" })];
      const progressMap = new Map<string, SessionProgress>([
        ["s1", createProgress({ idle: 1, done: 4, total: 5 })],
      ]);

      const result = groupSessions(sessions, progressMap);

      // Has idle tasks, so not "all terminal" — should be accepted
      expect(result.accepted).toHaveLength(1);
      expect(result.done).toHaveLength(0);
    });
  });
});
