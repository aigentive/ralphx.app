import { describe, it, expect } from "vitest";
import {
  getRemoveAllLabel,
  resolveGroupCleanupParams,
  GROUP_ACTIONS,
} from "./group-actions";

describe("group-actions", () => {
  describe("getRemoveAllLabel", () => {
    it("returns 'Remove all [status]' for column kind", () => {
      expect(getRemoveAllLabel("column", "Ready")).toBe("Remove all Ready");
    });

    it("returns 'Remove all from [plan]' for plan kind", () => {
      expect(getRemoveAllLabel("plan", "My Plan")).toBe("Remove all from My Plan");
    });

    it("returns 'Remove all Uncategorized' for uncategorized kind", () => {
      expect(getRemoveAllLabel("uncategorized", "anything")).toBe("Remove all Uncategorized");
    });
  });

  describe("resolveGroupCleanupParams", () => {
    it("maps column kind to status groupKind", () => {
      const result = resolveGroupCleanupParams("column", "ready");
      expect(result).toEqual({ groupKind: "status", groupId: "ready" });
    });

    it("maps plan kind to session groupKind", () => {
      const result = resolveGroupCleanupParams("plan", "session-123");
      expect(result).toEqual({ groupKind: "session", groupId: "session-123" });
    });

    it("maps uncategorized kind to uncategorized groupKind with empty groupId", () => {
      const result = resolveGroupCleanupParams("uncategorized", "ignored");
      expect(result).toEqual({ groupKind: "uncategorized", groupId: "" });
    });
  });

  describe("GROUP_ACTIONS.removeAll", () => {
    it("has destructive variant", () => {
      expect(GROUP_ACTIONS.removeAll.variant).toBe("destructive");
    });

    it("generates confirmation config with group label and task count", () => {
      const config = GROUP_ACTIONS.removeAll.confirmConfig("Ready", 5);
      expect(config.title).toBe("Remove all Ready?");
      expect(config.description).toContain("5 tasks");
      expect(config.variant).toBe("destructive");
    });

    it("uses singular 'task' for count of 1", () => {
      const config = GROUP_ACTIONS.removeAll.confirmConfig("Blocked", 1);
      expect(config.description).toContain("1 task.");
      expect(config.description).not.toContain("1 tasks");
    });
  });
});
