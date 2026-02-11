/**
 * Tests for planStore stub implementation
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { usePlanStore } from "./planStore";

describe("planStore (stub)", () => {
  beforeEach(() => {
    // Reset store state before each test
    usePlanStore.setState({
      activePlanByProject: {},
      planCandidates: [],
      isLoading: false,
      error: null,
    });
  });

  it("initializes with empty state", () => {
    const state = usePlanStore.getState();
    expect(state.activePlanByProject).toEqual({});
    expect(state.planCandidates).toEqual([]);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it("loadActivePlan does nothing (stub)", async () => {
    const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    await usePlanStore.getState().loadActivePlan("project-1");

    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("Stub implementation")
    );
    consoleSpy.mockRestore();
  });

  it("setActivePlan does nothing (stub)", async () => {
    const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    await usePlanStore.getState().setActivePlan("project-1", "session-1", "quick_switcher");

    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("Stub implementation")
    );
    consoleSpy.mockRestore();
  });

  it("clearActivePlan does nothing (stub)", async () => {
    const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    await usePlanStore.getState().clearActivePlan("project-1");

    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("Stub implementation")
    );
    consoleSpy.mockRestore();
  });

  it("loadCandidates sets empty candidates (stub)", async () => {
    const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    await usePlanStore.getState().loadCandidates("project-1");

    const state = usePlanStore.getState();
    expect(state.planCandidates).toEqual([]);
    expect(state.isLoading).toBe(false);
    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining("Stub implementation")
    );
    consoleSpy.mockRestore();
  });
});
