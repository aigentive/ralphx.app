/**
 * Active count inconsistency test
 *
 * Documents via tests that TeamSplitHeader and TeamOverviewHeader use
 * different active count formulas:
 * - TeamSplitHeader: excludes only "shutdown" → completed teammates count as active
 * - TeamOverviewHeader: excludes "shutdown" AND "completed" → stricter definition
 *
 * This is a known inconsistency that should be documented and eventually resolved.
 */

import { describe, it, expect } from "vitest";

/**
 * Pure logic extracted from TeamSplitHeader (line 33):
 *   teammates.filter((m) => m.status !== "shutdown").length
 */
function splitHeaderActiveCount(statuses: string[]): number {
  return statuses.filter((s) => s !== "shutdown").length;
}

/**
 * Pure logic extracted from TeamOverviewHeader (line 29):
 *   teammates.filter((m) => m.status !== "shutdown" && m.status !== "completed").length
 */
function overviewHeaderActiveCount(statuses: string[]): number {
  return statuses.filter((s) => s !== "shutdown" && s !== "completed").length;
}

describe("Active count inconsistency between headers", () => {
  const statuses = ["running", "idle", "completed", "shutdown", "failed"];

  it("TeamSplitHeader excludes only shutdown from active count", () => {
    const count = splitHeaderActiveCount(statuses);
    // running, idle, completed, failed = 4 (shutdown excluded)
    expect(count).toBe(4);
    // Notably, "completed" IS counted as active
    expect(splitHeaderActiveCount(["completed"])).toBe(1);
  });

  it("TeamOverviewHeader excludes both shutdown AND completed from active count", () => {
    const count = overviewHeaderActiveCount(statuses);
    // running, idle, failed = 3 (shutdown + completed excluded)
    expect(count).toBe(3);
    // "completed" is NOT counted as active
    expect(overviewHeaderActiveCount(["completed"])).toBe(0);
  });
});
