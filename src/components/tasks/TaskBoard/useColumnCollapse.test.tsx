/**
 * Tests for useColumnCollapse hook
 */

import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { WorkflowColumn } from "@/types/workflow";
import { useColumnCollapse } from "./useColumnCollapse";
import { useUiStore } from "@/stores/uiStore";

// Mock columns
const columns: WorkflowColumn[] = [
  { id: "draft", name: "Draft", mapsTo: "backlog" },
  { id: "ready", name: "Ready", mapsTo: "ready" },
  { id: "in_progress", name: "In Progress", mapsTo: "executing" },
  { id: "done", name: "Done", mapsTo: "approved" },
];

function makeCounts(counts: Record<string, number>): Map<string, number> {
  return new Map(Object.entries(counts));
}

describe("useColumnCollapse", () => {
  beforeEach(() => {
    // Reset uiStore collapsed state
    useUiStore.getState().setCollapsedColumns(new Set());
  });

  it("returns isCollapsed, toggleCollapse, and expandColumn", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 0 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, "session-1"),
    );

    expect(result.current.isCollapsed).toBeInstanceOf(Function);
    expect(result.current.toggleCollapse).toBeInstanceOf(Function);
    expect(result.current.expandColumn).toBeInstanceOf(Function);
  });

  it("auto-collapses empty columns on initial render", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, "session-1"),
    );

    // Empty columns should be collapsed
    expect(result.current.isCollapsed("ready")).toBe(true);
    expect(result.current.isCollapsed("done")).toBe(true);
    // Non-empty columns should remain expanded
    expect(result.current.isCollapsed("draft")).toBe(false);
    expect(result.current.isCollapsed("in_progress")).toBe(false);
  });

  it("auto-collapses empty columns on plan change", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 0 });

    const { result, rerender } = renderHook(
      ({ sessionId, counts }) =>
        useColumnCollapse(columns, counts, sessionId),
      { initialProps: { sessionId: "session-1", counts: taskCounts } },
    );

    // Only "done" is empty
    expect(result.current.isCollapsed("done")).toBe(true);
    expect(result.current.isCollapsed("ready")).toBe(false);

    // Plan changes — now "ready" is also empty
    const newCounts = makeCounts({ draft: 1, ready: 0, in_progress: 0, done: 0 });
    rerender({ sessionId: "session-2", counts: newCounts });

    expect(result.current.isCollapsed("ready")).toBe(true);
    expect(result.current.isCollapsed("in_progress")).toBe(true);
    expect(result.current.isCollapsed("done")).toBe(true);
  });

  it("auto-expands when count transitions from 0 to N", () => {
    const initialCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result, rerender } = renderHook(
      ({ counts }) => useColumnCollapse(columns, counts, "session-1"),
      { initialProps: { counts: initialCounts } },
    );

    // "ready" starts collapsed (empty)
    expect(result.current.isCollapsed("ready")).toBe(true);

    // Tasks arrive in "ready"
    const updatedCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 0 });
    rerender({ counts: updatedCounts });

    // Should auto-expand
    expect(result.current.isCollapsed("ready")).toBe(false);
  });

  it("respects user-initiated expand within same plan (won't re-collapse)", () => {
    const emptyCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result, rerender } = renderHook(
      ({ counts, sessionId }) =>
        useColumnCollapse(columns, counts, sessionId),
      { initialProps: { counts: emptyCounts, sessionId: "session-1" } },
    );

    // "ready" starts collapsed
    expect(result.current.isCollapsed("ready")).toBe(true);

    // User manually expands "ready"
    act(() => {
      result.current.toggleCollapse("ready");
    });
    expect(result.current.isCollapsed("ready")).toBe(false);

    // Re-render with same plan (counts stay same) — user expand is respected
    rerender({ counts: emptyCounts, sessionId: "session-1" });

    // Should remain expanded because user expanded it within the same plan
    expect(result.current.isCollapsed("ready")).toBe(false);
  });

  it("toggleCollapse toggles collapsed state", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 0 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, "session-1"),
    );

    // "draft" starts expanded
    expect(result.current.isCollapsed("draft")).toBe(false);

    // Toggle to collapsed
    act(() => {
      result.current.toggleCollapse("draft");
    });
    expect(result.current.isCollapsed("draft")).toBe(true);

    // Toggle back to expanded
    act(() => {
      result.current.toggleCollapse("draft");
    });
    expect(result.current.isCollapsed("draft")).toBe(false);
  });

  it("expandColumn expands a collapsed column", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, "session-1"),
    );

    // "ready" starts collapsed
    expect(result.current.isCollapsed("ready")).toBe(true);

    // Expand it
    act(() => {
      result.current.expandColumn("ready");
    });
    expect(result.current.isCollapsed("ready")).toBe(false);
  });

  it("does not auto-collapse columns with tasks", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 5 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, "session-1"),
    );

    // All columns have tasks, none should be collapsed
    expect(result.current.isCollapsed("draft")).toBe(false);
    expect(result.current.isCollapsed("ready")).toBe(false);
    expect(result.current.isCollapsed("in_progress")).toBe(false);
    expect(result.current.isCollapsed("done")).toBe(false);
  });

  it("clears user-expanded tracking on plan change", () => {
    const emptyCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result, rerender } = renderHook(
      ({ counts, sessionId }) =>
        useColumnCollapse(columns, counts, sessionId),
      { initialProps: { counts: emptyCounts, sessionId: "session-1" } },
    );

    // User manually expands "ready"
    act(() => {
      result.current.expandColumn("ready");
    });
    expect(result.current.isCollapsed("ready")).toBe(false);

    // Plan changes (different session ID) — user-expanded tracking is reset
    // and "ready" is still empty → auto-collapses again
    rerender({ counts: emptyCounts, sessionId: "session-2" });

    // On plan change, user-expanded tracking resets and empty cols re-collapse
    expect(result.current.isCollapsed("ready")).toBe(true);
  });

  it("handles undefined ideationSessionId", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 0, in_progress: 1, done: 0 });

    const { result } = renderHook(() =>
      useColumnCollapse(columns, taskCounts, undefined),
    );

    expect(result.current.isCollapsed("ready")).toBe(true);
    expect(result.current.isCollapsed("draft")).toBe(false);
  });

  it("user-initiated collapse is tracked (won't auto-expand)", () => {
    const taskCounts = makeCounts({ draft: 3, ready: 2, in_progress: 1, done: 0 });

    const { result, rerender } = renderHook(
      ({ counts }) => useColumnCollapse(columns, counts, "session-1"),
      { initialProps: { counts: taskCounts } },
    );

    // User collapses "ready" manually
    act(() => {
      result.current.toggleCollapse("ready");
    });
    expect(result.current.isCollapsed("ready")).toBe(true);

    // Count increases further — should NOT auto-expand because user collapsed it
    const moreCounts = makeCounts({ draft: 3, ready: 5, in_progress: 1, done: 0 });
    rerender({ counts: moreCounts });

    expect(result.current.isCollapsed("ready")).toBe(true);
  });
});
