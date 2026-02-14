/**
 * usePlanCandidateSort hook tests
 *
 * Tests for plan candidate tier-based sorting with within-tier date ordering.
 */

import { describe, it, expect } from "vitest";
import { renderHook } from "@testing-library/react";
import { sortPlanCandidates, usePlanCandidateSort } from "./usePlanCandidateSort";
import type { PlanCandidate } from "@/stores/planStore";

// Helper to create a PlanCandidate with specific stats
function createCandidate(
  overrides: Partial<PlanCandidate> = {}
): PlanCandidate {
  return {
    sessionId: `session-${Math.random()}`,
    title: "Test Plan",
    acceptedAt: new Date().toISOString(),
    taskStats: {
      total: 0,
      incomplete: 0,
      activeNow: 0,
    },
    interactionStats: {
      selectedCount: 0,
      lastSelectedAt: null,
    },
    score: 0,
    ...overrides,
  };
}

describe("sortPlanCandidates", () => {
  describe("tier ordering", () => {
    it("sorts Tier 0 (activeNow > 0) before Tier 1 (incomplete > 0)", () => {
      const tier0 = createCandidate({
        sessionId: "tier0",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });
      const tier1 = createCandidate({
        sessionId: "tier1",
        taskStats: { total: 5, incomplete: 3, activeNow: 0 },
      });

      const sorted = sortPlanCandidates([tier1, tier0]);

      expect(sorted[0].sessionId).toBe("tier0");
      expect(sorted[1].sessionId).toBe("tier1");
    });

    it("sorts Tier 1 (incomplete > 0) before Tier 2 (all complete)", () => {
      const tier1 = createCandidate({
        sessionId: "tier1",
        taskStats: { total: 5, incomplete: 2, activeNow: 0 },
      });
      const tier2 = createCandidate({
        sessionId: "tier2",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      });

      const sorted = sortPlanCandidates([tier2, tier1]);

      expect(sorted[0].sessionId).toBe("tier1");
      expect(sorted[1].sessionId).toBe("tier2");
    });

    it("sorts all three tiers correctly", () => {
      const tier0 = createCandidate({
        sessionId: "tier0",
        taskStats: { total: 5, incomplete: 3, activeNow: 2 },
      });
      const tier1 = createCandidate({
        sessionId: "tier1",
        taskStats: { total: 5, incomplete: 3, activeNow: 0 },
      });
      const tier2 = createCandidate({
        sessionId: "tier2",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      });

      const sorted = sortPlanCandidates([tier2, tier1, tier0]);

      expect(sorted[0].sessionId).toBe("tier0");
      expect(sorted[1].sessionId).toBe("tier1");
      expect(sorted[2].sessionId).toBe("tier2");
    });
  });

  describe("within-tier date sorting", () => {
    it("sorts most recently accepted first within same tier", () => {
      const older = createCandidate({
        sessionId: "older",
        acceptedAt: "2026-01-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });
      const newer = createCandidate({
        sessionId: "newer",
        acceptedAt: "2026-02-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });

      const sorted = sortPlanCandidates([older, newer]);

      expect(sorted[0].sessionId).toBe("newer");
      expect(sorted[1].sessionId).toBe("older");
    });

    it("applies date sorting within each tier independently", () => {
      const tier0Newer = createCandidate({
        sessionId: "tier0-newer",
        acceptedAt: "2026-02-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });
      const tier0Older = createCandidate({
        sessionId: "tier0-older",
        acceptedAt: "2026-01-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });
      const tier1Newer = createCandidate({
        sessionId: "tier1-newer",
        acceptedAt: "2026-02-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 0 },
      });
      const tier1Older = createCandidate({
        sessionId: "tier1-older",
        acceptedAt: "2026-01-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 3, activeNow: 0 },
      });

      const sorted = sortPlanCandidates([
        tier1Older,
        tier0Older,
        tier1Newer,
        tier0Newer,
      ]);

      expect(sorted[0].sessionId).toBe("tier0-newer");
      expect(sorted[1].sessionId).toBe("tier0-older");
      expect(sorted[2].sessionId).toBe("tier1-newer");
      expect(sorted[3].sessionId).toBe("tier1-older");
    });
  });

  describe("edge cases", () => {
    it("handles empty array", () => {
      const sorted = sortPlanCandidates([]);
      expect(sorted).toEqual([]);
    });

    it("handles single item", () => {
      const candidate = createCandidate({ sessionId: "solo" });
      const sorted = sortPlanCandidates([candidate]);
      expect(sorted).toHaveLength(1);
      expect(sorted[0].sessionId).toBe("solo");
    });

    it("handles all-same-tier candidates", () => {
      const plan1 = createCandidate({
        sessionId: "plan1",
        acceptedAt: "2026-01-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      });
      const plan2 = createCandidate({
        sessionId: "plan2",
        acceptedAt: "2026-02-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      });
      const plan3 = createCandidate({
        sessionId: "plan3",
        acceptedAt: "2026-03-01T00:00:00Z",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      });

      const sorted = sortPlanCandidates([plan1, plan2, plan3]);

      expect(sorted[0].sessionId).toBe("plan3");
      expect(sorted[1].sessionId).toBe("plan2");
      expect(sorted[2].sessionId).toBe("plan1");
    });

    it("does not mutate original array", () => {
      const candidates = [
        createCandidate({
          sessionId: "a",
          taskStats: { total: 5, incomplete: 0, activeNow: 0 },
        }),
        createCandidate({
          sessionId: "b",
          taskStats: { total: 5, incomplete: 3, activeNow: 1 },
        }),
      ];
      const originalOrder = candidates.map((c) => c.sessionId);

      sortPlanCandidates(candidates);

      expect(candidates.map((c) => c.sessionId)).toEqual(originalOrder);
    });
  });

  describe("mixed tiers with complex scenarios", () => {
    it("handles plans with no tasks (total: 0)", () => {
      const noTasks = createCandidate({
        sessionId: "no-tasks",
        taskStats: { total: 0, incomplete: 0, activeNow: 0 },
      });
      const withTasks = createCandidate({
        sessionId: "with-tasks",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      });

      const sorted = sortPlanCandidates([noTasks, withTasks]);

      expect(sorted[0].sessionId).toBe("with-tasks");
      expect(sorted[1].sessionId).toBe("no-tasks");
    });

    it("sorts multiple active plans by date", () => {
      const active1 = createCandidate({
        sessionId: "active1",
        acceptedAt: "2026-01-15T00:00:00Z",
        taskStats: { total: 10, incomplete: 5, activeNow: 2 },
      });
      const active2 = createCandidate({
        sessionId: "active2",
        acceptedAt: "2026-02-10T00:00:00Z",
        taskStats: { total: 8, incomplete: 4, activeNow: 3 },
      });
      const active3 = createCandidate({
        sessionId: "active3",
        acceptedAt: "2026-01-20T00:00:00Z",
        taskStats: { total: 6, incomplete: 3, activeNow: 1 },
      });

      const sorted = sortPlanCandidates([active1, active2, active3]);

      expect(sorted[0].sessionId).toBe("active2");
      expect(sorted[1].sessionId).toBe("active3");
      expect(sorted[2].sessionId).toBe("active1");
    });
  });
});

describe("usePlanCandidateSort hook", () => {
  it("returns sorted candidates", () => {
    const candidates = [
      createCandidate({
        sessionId: "tier2",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      }),
      createCandidate({
        sessionId: "tier0",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      }),
    ];

    const { result } = renderHook(() => usePlanCandidateSort(candidates));

    expect(result.current[0].sessionId).toBe("tier0");
    expect(result.current[1].sessionId).toBe("tier2");
  });

  it("memoizes the result when input array identity stays the same", () => {
    const candidates = [
      createCandidate({
        sessionId: "plan1",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      }),
    ];

    const { result, rerender } = renderHook(
      ({ input }) => usePlanCandidateSort(input),
      { initialProps: { input: candidates } }
    );

    const firstResult = result.current;
    rerender({ input: candidates });

    expect(result.current).toBe(firstResult);
  });

  it("recomputes when input array changes", () => {
    const candidates1 = [
      createCandidate({
        sessionId: "plan1",
        taskStats: { total: 5, incomplete: 3, activeNow: 1 },
      }),
    ];
    const candidates2 = [
      createCandidate({
        sessionId: "plan2",
        taskStats: { total: 5, incomplete: 0, activeNow: 0 },
      }),
    ];

    const { result, rerender } = renderHook(
      ({ input }) => usePlanCandidateSort(input),
      { initialProps: { input: candidates1 } }
    );

    const firstResult = result.current;
    rerender({ input: candidates2 });

    expect(result.current).not.toBe(firstResult);
    expect(result.current[0].sessionId).toBe("plan2");
  });
});
