import { describe, it, expect } from "vitest";
import { getTaskActions } from "./task-actions";
import { canEdit, SYSTEM_CONTROLLED_STATUSES, CONFIRMATION_CONFIGS } from "./constants";
import type { InternalStatus } from "@/types/status";
import { INTERNAL_STATUS_VALUES } from "@/types/status";

// ============================================================================
// canEdit
// ============================================================================

describe("canEdit", () => {
  it("returns true for non-archived, non-system-controlled task", () => {
    expect(canEdit({ archivedAt: null, internalStatus: "backlog" })).toBe(true);
    expect(canEdit({ archivedAt: null, internalStatus: "ready" })).toBe(true);
    expect(canEdit({ archivedAt: null, internalStatus: "blocked" })).toBe(true);
  });

  it("returns false for archived tasks", () => {
    expect(canEdit({ archivedAt: "2026-01-01T00:00:00Z", internalStatus: "backlog" })).toBe(false);
  });

  it("returns false for system-controlled statuses", () => {
    for (const status of SYSTEM_CONTROLLED_STATUSES) {
      expect(canEdit({ archivedAt: null, internalStatus: status })).toBe(false);
    }
  });
});

// ============================================================================
// SYSTEM_CONTROLLED_STATUSES
// ============================================================================

describe("SYSTEM_CONTROLLED_STATUSES", () => {
  it("includes all expected agent-managed statuses", () => {
    const expected = [
      "executing", "qa_refining", "qa_testing", "qa_passed", "qa_failed",
      "pending_review", "revision_needed", "reviewing", "review_passed", "re_executing",
    ];
    for (const status of expected) {
      expect(SYSTEM_CONTROLLED_STATUSES).toContain(status);
    }
  });

  it("does not include user-manageable statuses", () => {
    expect(SYSTEM_CONTROLLED_STATUSES).not.toContain("backlog");
    expect(SYSTEM_CONTROLLED_STATUSES).not.toContain("ready");
    expect(SYSTEM_CONTROLLED_STATUSES).not.toContain("blocked");
    expect(SYSTEM_CONTROLLED_STATUSES).not.toContain("cancelled");
    expect(SYSTEM_CONTROLLED_STATUSES).not.toContain("failed");
  });
});

// ============================================================================
// getTaskActions — Kanban surface
// ============================================================================

describe("getTaskActions (kanban)", () => {
  it("returns Cancel for backlog", () => {
    const actions = getTaskActions("backlog", "kanban");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("cancel");
    expect(actions[0].label).toBe("Cancel");
    expect(actions[0].confirmConfig).toBeDefined();
  });

  it("returns Block and Cancel for ready", () => {
    const actions = getTaskActions("ready", "kanban");
    expect(actions).toHaveLength(2);
    expect(actions.map(a => a.id)).toEqual(["block", "cancel"]);
    expect(actions[0].opensDialog).toBe(true);
  });

  it("returns Unblock and Cancel for blocked", () => {
    const actions = getTaskActions("blocked", "kanban");
    expect(actions).toHaveLength(2);
    expect(actions.map(a => a.id)).toEqual(["unblock", "cancel"]);
  });

  it("returns Re-open for approved", () => {
    const actions = getTaskActions("approved", "kanban");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("reopen");
    expect(actions[0].label).toBe("Re-open");
  });

  it("returns Retry for failed", () => {
    const actions = getTaskActions("failed", "kanban");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("retry");
    expect(actions[0].label).toBe("Retry");
    expect(actions[0].confirmConfig).toEqual(CONFIRMATION_CONFIGS.retry);
  });

  it("returns Re-open for cancelled", () => {
    const actions = getTaskActions("cancelled", "kanban");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("reopen");
  });

  it("returns empty for system-controlled statuses", () => {
    const systemStatuses: InternalStatus[] = [
      "executing", "re_executing", "reviewing", "pending_review",
    ];
    for (const status of systemStatuses) {
      expect(getTaskActions(status, "kanban")).toHaveLength(0);
    }
  });
});

// ============================================================================
// getTaskActions — Graph surface
// ============================================================================

describe("getTaskActions (graph)", () => {
  it("returns Start Execution and Block for ready", () => {
    const actions = getTaskActions("ready", "graph");
    expect(actions).toHaveLength(2);
    expect(actions[0].id).toBe("start");
    expect(actions[0].label).toBe("Start Execution");
    expect(actions[1].id).toBe("block");
    expect(actions[1].opensDialog).toBe(true);
  });

  it("returns Unblock and View Blockers for blocked", () => {
    const actions = getTaskActions("blocked", "graph");
    expect(actions).toHaveLength(2);
    expect(actions[0].id).toBe("unblock");
    expect(actions[1].id).toBe("view-blockers");
    expect(actions[1].isViewAction).toBe(true);
  });

  it("returns View Agent Chat for executing", () => {
    const actions = getTaskActions("executing", "graph");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("view-chat");
    expect(actions[0].isViewAction).toBe(true);
  });

  it("returns View Agent Chat for re_executing", () => {
    const actions = getTaskActions("re_executing", "graph");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("view-chat");
  });

  it("returns View Work Summary for pending_review", () => {
    const actions = getTaskActions("pending_review", "graph");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("view-summary");
    expect(actions[0].isViewAction).toBe(true);
  });

  it("returns Approve and Request Changes for review_passed", () => {
    const actions = getTaskActions("review_passed", "graph");
    expect(actions).toHaveLength(2);
    expect(actions.map(a => a.id)).toEqual(["approve", "request-changes"]);
  });

  it("returns Approve, Reject, Request Changes for escalated", () => {
    const actions = getTaskActions("escalated", "graph");
    expect(actions).toHaveLength(3);
    expect(actions.map(a => a.id)).toEqual(["approve", "reject", "request-changes"]);
    expect(actions[1].variant).toBe("destructive");
  });

  it("returns View Feedback for revision_needed", () => {
    const actions = getTaskActions("revision_needed", "graph");
    expect(actions).toHaveLength(1);
    expect(actions[0].id).toBe("view-feedback");
    expect(actions[0].isViewAction).toBe(true);
  });

  it("returns View Conflicts and Mark Resolved for merge_conflict", () => {
    const actions = getTaskActions("merge_conflict", "graph");
    expect(actions).toHaveLength(2);
    expect(actions[0].id).toBe("view-conflicts");
    expect(actions[0].isViewAction).toBe(true);
    expect(actions[1].id).toBe("mark-resolved");
    expect(actions[1].confirmConfig).toBeDefined();
  });

  it("returns empty for statuses without quick actions", () => {
    const noActionStatuses: InternalStatus[] = [
      "backlog", "qa_refining", "qa_testing", "qa_passed", "qa_failed",
      "approved", "pending_merge", "merging", "merge_incomplete",
      "merged", "failed", "cancelled", "paused", "stopped",
    ];
    for (const status of noActionStatuses) {
      expect(getTaskActions(status, "graph")).toHaveLength(0);
    }
  });
});

// ============================================================================
// Exhaustiveness — every status returns without error
// ============================================================================

describe("getTaskActions exhaustiveness", () => {
  it("handles every InternalStatus for kanban without throwing", () => {
    for (const status of INTERNAL_STATUS_VALUES) {
      expect(() => getTaskActions(status, "kanban")).not.toThrow();
    }
  });

  it("handles every InternalStatus for graph without throwing", () => {
    for (const status of INTERNAL_STATUS_VALUES) {
      expect(() => getTaskActions(status, "graph")).not.toThrow();
    }
  });
});

// ============================================================================
// CONFIRMATION_CONFIGS
// ============================================================================

describe("CONFIRMATION_CONFIGS", () => {
  it("has configs for all confirmation-requiring action IDs", () => {
    const requiredKeys = [
      "cancelled", "blocked", "ready", "backlog", "retry",
      "start", "unblock", "approve", "reject", "request-changes", "mark-resolved",
      "archive", "restore", "permanent-delete",
    ];
    for (const key of requiredKeys) {
      expect(CONFIRMATION_CONFIGS[key]).toBeDefined();
      expect(CONFIRMATION_CONFIGS[key].title).toBeTruthy();
      expect(CONFIRMATION_CONFIGS[key].description).toBeTruthy();
      expect(["default", "destructive"]).toContain(CONFIRMATION_CONFIGS[key].variant);
    }
  });

  it("marks destructive actions correctly", () => {
    expect(CONFIRMATION_CONFIGS.cancelled.variant).toBe("destructive");
    expect(CONFIRMATION_CONFIGS.reject.variant).toBe("destructive");
    expect(CONFIRMATION_CONFIGS["permanent-delete"].variant).toBe("destructive");
  });

  it("marks non-destructive actions as default", () => {
    expect(CONFIRMATION_CONFIGS.approve.variant).toBe("default");
    expect(CONFIRMATION_CONFIGS.unblock.variant).toBe("default");
    expect(CONFIRMATION_CONFIGS.start.variant).toBe("default");
  });
});
