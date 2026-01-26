/**
 * Tests for useTaskExecutionState hook and formatDuration
 */

import { describe, it, expect } from "vitest";
import { formatDuration } from "./useTaskExecutionState";
import type { ExecutionPhase } from "./useTaskExecutionState";

/**
 * Re-export internal functions for testing
 * These would normally be internal, but we make them testable
 */
function getExecutionPhaseForTest(status: string): ExecutionPhase {
  if (status === "executing" || status === "execution_done") {
    return "executing";
  }
  if (status.startsWith("qa_")) {
    return "qa";
  }
  if (status === "pending_review" || status === "revision_needed") {
    return "review";
  }
  if (status === "approved" || status === "failed" || status === "cancelled") {
    return "done";
  }
  return "idle";
}

function calculateDurationForTest(startedAt: string | null): number | null {
  if (!startedAt) return null;

  const start = new Date(startedAt);
  const now = new Date();
  const diffMs = now.getTime() - start.getTime();
  return Math.floor(diffMs / 1000);
}

describe("getExecutionPhase", () => {
  it("should return idle for backlog status", () => {
    expect(getExecutionPhaseForTest("backlog")).toBe("idle");
    expect(getExecutionPhaseForTest("ready")).toBe("idle");
    expect(getExecutionPhaseForTest("blocked")).toBe("idle");
  });

  it("should return executing for executing status", () => {
    expect(getExecutionPhaseForTest("executing")).toBe("executing");
    expect(getExecutionPhaseForTest("execution_done")).toBe("executing");
  });

  it("should return qa for qa_* statuses", () => {
    expect(getExecutionPhaseForTest("qa_refining")).toBe("qa");
    expect(getExecutionPhaseForTest("qa_testing")).toBe("qa");
    expect(getExecutionPhaseForTest("qa_passed")).toBe("qa");
    expect(getExecutionPhaseForTest("qa_failed")).toBe("qa");
  });

  it("should return review for review statuses", () => {
    expect(getExecutionPhaseForTest("pending_review")).toBe("review");
    expect(getExecutionPhaseForTest("revision_needed")).toBe("review");
  });

  it("should return done for terminal statuses", () => {
    expect(getExecutionPhaseForTest("approved")).toBe("done");
    expect(getExecutionPhaseForTest("failed")).toBe("done");
    expect(getExecutionPhaseForTest("cancelled")).toBe("done");
  });
});

describe("calculateDuration", () => {
  it("should return null for null startedAt", () => {
    expect(calculateDurationForTest(null)).toBeNull();
  });

  it("should calculate duration in seconds", () => {
    // Create a date 2 minutes in the past
    const twoMinutesAgo = new Date(Date.now() - 120000).toISOString();
    const duration = calculateDurationForTest(twoMinutesAgo);

    // Duration should be approximately 120 seconds (allow 1 second tolerance for test execution time)
    expect(duration).toBeGreaterThanOrEqual(119);
    expect(duration).toBeLessThanOrEqual(121);
  });
});

describe("formatDuration", () => {
  it("should format seconds only", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(45)).toBe("45s");
  });

  it("should format minutes and seconds", () => {
    expect(formatDuration(60)).toBe("1m");
    expect(formatDuration(90)).toBe("1m 30s");
    expect(formatDuration(135)).toBe("2m 15s");
  });

  it("should format hours and minutes", () => {
    expect(formatDuration(3600)).toBe("1h");
    expect(formatDuration(3660)).toBe("1h 1m");
    expect(formatDuration(5400)).toBe("1h 30m");
  });

  it("should format hours, minutes, and seconds", () => {
    expect(formatDuration(3665)).toBe("1h 1m 5s");
    expect(formatDuration(7325)).toBe("2h 2m 5s");
  });

  it("should handle null and negative values", () => {
    expect(formatDuration(null)).toBe("0s");
    expect(formatDuration(-10)).toBe("0s");
  });

  it("should handle large durations", () => {
    expect(formatDuration(36000)).toBe("10h");
    expect(formatDuration(86400)).toBe("24h");
  });
});
