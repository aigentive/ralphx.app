/**
 * usePlanCandidateSort tests
 *
 * Tests for plan candidate sorting logic based on task statistics
 */

import { describe, it, expect } from "vitest";
import { renderHook } from "@testing-library/react";
import {
  sortPlanCandidates,
  getCandidateTier,
  usePlanCandidateSort,
} from "./usePlanCandidateSort";
import type { PlanCandidate } from "@/stores/planStore";

// Mock plan candidates for testing
const createCandidate = (
  sessionId: string,
  activeNow: number,
  incomplete: number,
  total: number,
  acceptedAt: string
): PlanCandidate => ({
  sessionId,
  title: `Plan ${sessionId}`,
  acceptedAt,
  taskStats: { total, incomplete, activeNow },
  interactionStats: { selectedCount: 0, lastSelectedAt: null },
  score: 0,
});

describe("getCandidateTier", () => {
  it("should return tier 0 for candidates with activeNow > 0", () => {
    const candidate = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    expect(getCandidateTier(candidate)).toBe(0);
  });

  it("should return tier 0 for candidates with multiple active tasks", () => {
    const candidate = createCandidate("1", 3, 5, 10, "2026-01-01T00:00:00Z");
    expect(getCandidateTier(candidate)).toBe(0);
  });

  it("should return tier 1 for candidates with incomplete > 0 and activeNow === 0", () => {
    const candidate = createCandidate("1", 0, 5, 10, "2026-01-01T00:00:00Z");
    expect(getCandidateTier(candidate)).toBe(1);
  });

  it("should return tier 2 for candidates with all tasks complete", () => {
    const candidate = createCandidate("1", 0, 0, 10, "2026-01-01T00:00:00Z");
    expect(getCandidateTier(candidate)).toBe(2);
  });

  it("should return tier 2 for candidates with no tasks", () => {
    const candidate = createCandidate("1", 0, 0, 0, "2026-01-01T00:00:00Z");
    expect(getCandidateTier(candidate)).toBe(2);
  });
});

describe("sortPlanCandidates", () => {
  it("should return empty array for empty input", () => {
    const result = sortPlanCandidates([]);
    expect(result).toEqual([]);
  });

  it("should return single candidate unchanged", () => {
    const candidate = createCandidate("1", 0, 5, 10, "2026-01-01T00:00:00Z");
    const result = sortPlanCandidates([candidate]);
    expect(result).toEqual([candidate]);
  });

  it("should prioritize tier 0 (activeNow > 0) over tier 1", () => {
    const tier0 = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    const tier1 = createCandidate("2", 0, 5, 10, "2026-01-02T00:00:00Z");

    const result = sortPlanCandidates([tier1, tier0]);
    expect(result[0]).toEqual(tier0);
    expect(result[1]).toEqual(tier1);
  });

  it("should prioritize tier 0 over tier 2", () => {
    const tier0 = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    const tier2 = createCandidate("2", 0, 0, 10, "2026-01-02T00:00:00Z");

    const result = sortPlanCandidates([tier2, tier0]);
    expect(result[0]).toEqual(tier0);
    expect(result[1]).toEqual(tier2);
  });

  it("should prioritize tier 1 over tier 2", () => {
    const tier1 = createCandidate("1", 0, 5, 10, "2026-01-01T00:00:00Z");
    const tier2 = createCandidate("2", 0, 0, 10, "2026-01-02T00:00:00Z");

    const result = sortPlanCandidates([tier2, tier1]);
    expect(result[0]).toEqual(tier1);
    expect(result[1]).toEqual(tier2);
  });

  it("should sort by acceptedAt DESC within same tier", () => {
    const older = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    const newer = createCandidate("2", 1, 5, 10, "2026-01-02T00:00:00Z");

    const result = sortPlanCandidates([older, newer]);
    expect(result[0]).toEqual(newer); // Most recent first
    expect(result[1]).toEqual(older);
  });

  it("should handle multiple candidates across all tiers", () => {
    const tier0a = createCandidate("1", 2, 5, 10, "2026-01-03T00:00:00Z");
    const tier0b = createCandidate("2", 1, 5, 10, "2026-01-05T00:00:00Z");
    const tier1a = createCandidate("3", 0, 3, 10, "2026-01-04T00:00:00Z");
    const tier1b = createCandidate("4", 0, 8, 10, "2026-01-02T00:00:00Z");
    const tier2a = createCandidate("5", 0, 0, 10, "2026-01-06T00:00:00Z");
    const tier2b = createCandidate("6", 0, 0, 0, "2026-01-01T00:00:00Z");

    const result = sortPlanCandidates([tier2a, tier1a, tier0a, tier2b, tier1b, tier0b]);

    // Tier 0 first (most recent first)
    expect(result[0]).toEqual(tier0b); // 2026-01-05
    expect(result[1]).toEqual(tier0a); // 2026-01-03

    // Tier 1 second (most recent first)
    expect(result[2]).toEqual(tier1a); // 2026-01-04
    expect(result[3]).toEqual(tier1b); // 2026-01-02

    // Tier 2 last (most recent first)
    expect(result[4]).toEqual(tier2a); // 2026-01-06
    expect(result[5]).toEqual(tier2b); // 2026-01-01
  });

  it("should handle all candidates in same tier", () => {
    const c1 = createCandidate("1", 0, 5, 10, "2026-01-01T00:00:00Z");
    const c2 = createCandidate("2", 0, 5, 10, "2026-01-03T00:00:00Z");
    const c3 = createCandidate("3", 0, 5, 10, "2026-01-02T00:00:00Z");

    const result = sortPlanCandidates([c1, c2, c3]);

    expect(result[0]).toEqual(c2); // 2026-01-03
    expect(result[1]).toEqual(c3); // 2026-01-02
    expect(result[2]).toEqual(c1); // 2026-01-01
  });

  it("should handle date parsing correctly", () => {
    const older = createCandidate("1", 1, 5, 10, "2026-01-01T10:30:00Z");
    const newer = createCandidate("2", 1, 5, 10, "2026-01-01T15:45:00Z");

    const result = sortPlanCandidates([older, newer]);
    expect(result[0]).toEqual(newer);
    expect(result[1]).toEqual(older);
  });

  it("should not mutate original array", () => {
    const c1 = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    const c2 = createCandidate("2", 0, 5, 10, "2026-01-02T00:00:00Z");
    const original = [c2, c1];
    const originalCopy = [...original];

    sortPlanCandidates(original);

    expect(original).toEqual(originalCopy);
  });
});

describe("usePlanCandidateSort", () => {
  it("should sort candidates using the hook", () => {
    const tier0 = createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z");
    const tier1 = createCandidate("2", 0, 5, 10, "2026-01-02T00:00:00Z");
    const candidates = [tier1, tier0];

    const { result } = renderHook(() => usePlanCandidateSort(candidates));

    expect(result.current[0]).toEqual(tier0);
    expect(result.current[1]).toEqual(tier1);
  });

  it("should return empty array for empty input", () => {
    const { result } = renderHook(() => usePlanCandidateSort([]));
    expect(result.current).toEqual([]);
  });

  it("should memoize result when input unchanged", () => {
    const candidates = [
      createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z"),
      createCandidate("2", 0, 5, 10, "2026-01-02T00:00:00Z"),
    ];

    const { result, rerender } = renderHook(
      ({ candidates }) => usePlanCandidateSort(candidates),
      { initialProps: { candidates } }
    );

    const firstResult = result.current;
    rerender({ candidates });
    const secondResult = result.current;

    // Same reference if input unchanged
    expect(firstResult).toBe(secondResult);
  });

  it("should recompute when candidates change", () => {
    const candidates1 = [
      createCandidate("1", 1, 5, 10, "2026-01-01T00:00:00Z"),
    ];
    const candidates2 = [
      createCandidate("2", 0, 5, 10, "2026-01-02T00:00:00Z"),
    ];

    const { result, rerender } = renderHook(
      ({ candidates }) => usePlanCandidateSort(candidates),
      { initialProps: { candidates: candidates1 } }
    );

    expect(result.current[0]?.sessionId).toBe("1");

    rerender({ candidates: candidates2 });

    expect(result.current[0]?.sessionId).toBe("2");
  });
});
