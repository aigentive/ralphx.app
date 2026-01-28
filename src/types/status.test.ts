import { describe, it, expect } from "vitest";
import {
  InternalStatusSchema,
  INTERNAL_STATUS_VALUES,
  IDLE_STATUSES,
  ACTIVE_STATUSES,
  TERMINAL_STATUSES,
  isTerminalStatus,
  isActiveStatus,
  isIdleStatus,
} from "./status";

describe("InternalStatusSchema", () => {
  it("should have exactly 13 status values", () => {
    expect(INTERNAL_STATUS_VALUES.length).toBe(13);
  });

  it("should parse all valid status values", () => {
    const validStatuses = [
      "backlog",
      "ready",
      "blocked",
      "executing",
      "qa_refining",
      "qa_testing",
      "qa_passed",
      "qa_failed",
      "pending_review",
      "revision_needed",
      "approved",
      "failed",
      "cancelled",
    ];

    for (const status of validStatuses) {
      expect(() => InternalStatusSchema.parse(status)).not.toThrow();
    }
  });

  it("should reject invalid status values", () => {
    const invalidStatuses = [
      "invalid",
      "Backlog", // Wrong case
      "READY",
      "pending", // partial match
      "",
      null,
      undefined,
      123,
    ];

    for (const status of invalidStatuses) {
      expect(() => InternalStatusSchema.parse(status)).toThrow();
    }
  });

  it("should provide helpful error for invalid values", () => {
    const result = InternalStatusSchema.safeParse("invalid");
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0]?.message).toBeDefined();
    }
  });
});

describe("Status Categories", () => {
  it("should have 3 idle statuses", () => {
    expect(IDLE_STATUSES.length).toBe(3);
    expect(IDLE_STATUSES).toContain("backlog");
    expect(IDLE_STATUSES).toContain("ready");
    expect(IDLE_STATUSES).toContain("blocked");
  });

  it("should have 5 active statuses", () => {
    expect(ACTIVE_STATUSES.length).toBe(5);
    expect(ACTIVE_STATUSES).toContain("executing");
    expect(ACTIVE_STATUSES).toContain("qa_refining");
    expect(ACTIVE_STATUSES).toContain("qa_testing");
    expect(ACTIVE_STATUSES).toContain("pending_review");
    expect(ACTIVE_STATUSES).toContain("revision_needed");
  });

  it("should have 3 terminal statuses", () => {
    expect(TERMINAL_STATUSES.length).toBe(3);
    expect(TERMINAL_STATUSES).toContain("approved");
    expect(TERMINAL_STATUSES).toContain("failed");
    expect(TERMINAL_STATUSES).toContain("cancelled");
  });

  it("should have no overlap between categories", () => {
    const idleSet = new Set(IDLE_STATUSES);
    const activeSet = new Set(ACTIVE_STATUSES);
    const terminalSet = new Set(TERMINAL_STATUSES);

    for (const status of IDLE_STATUSES) {
      expect(activeSet.has(status)).toBe(false);
      expect(terminalSet.has(status)).toBe(false);
    }

    for (const status of ACTIVE_STATUSES) {
      expect(idleSet.has(status)).toBe(false);
      expect(terminalSet.has(status)).toBe(false);
    }

    for (const status of TERMINAL_STATUSES) {
      expect(idleSet.has(status)).toBe(false);
      expect(activeSet.has(status)).toBe(false);
    }
  });

  it("should cover all 13 statuses between categories plus qa_passed and qa_failed", () => {
    const allCategorized = [
      ...IDLE_STATUSES,
      ...ACTIVE_STATUSES,
      ...TERMINAL_STATUSES,
      "qa_passed",
      "qa_failed",
    ];
    expect(allCategorized.length).toBe(13);
  });
});

describe("Status Helper Functions", () => {
  describe("isTerminalStatus", () => {
    it("should return true for terminal statuses", () => {
      expect(isTerminalStatus("approved")).toBe(true);
      expect(isTerminalStatus("failed")).toBe(true);
      expect(isTerminalStatus("cancelled")).toBe(true);
    });

    it("should return false for non-terminal statuses", () => {
      expect(isTerminalStatus("backlog")).toBe(false);
      expect(isTerminalStatus("executing")).toBe(false);
      expect(isTerminalStatus("qa_testing")).toBe(false);
    });
  });

  describe("isActiveStatus", () => {
    it("should return true for active statuses", () => {
      expect(isActiveStatus("executing")).toBe(true);
      expect(isActiveStatus("qa_refining")).toBe(true);
      expect(isActiveStatus("qa_testing")).toBe(true);
      expect(isActiveStatus("pending_review")).toBe(true);
      expect(isActiveStatus("revision_needed")).toBe(true);
    });

    it("should return false for non-active statuses", () => {
      expect(isActiveStatus("backlog")).toBe(false);
      expect(isActiveStatus("approved")).toBe(false);
      expect(isActiveStatus("qa_passed")).toBe(false);
    });
  });

  describe("isIdleStatus", () => {
    it("should return true for idle statuses", () => {
      expect(isIdleStatus("backlog")).toBe(true);
      expect(isIdleStatus("ready")).toBe(true);
      expect(isIdleStatus("blocked")).toBe(true);
    });

    it("should return false for non-idle statuses", () => {
      expect(isIdleStatus("executing")).toBe(false);
      expect(isIdleStatus("approved")).toBe(false);
      expect(isIdleStatus("qa_testing")).toBe(false);
    });
  });
});
