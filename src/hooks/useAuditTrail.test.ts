/**
 * useAuditTrail hook tests
 *
 * Tests the hook that merges review notes (state history) and activity events
 * into a unified, chronologically-sorted audit trail timeline.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import { useAuditTrail } from "./useAuditTrail";
import { api } from "@/lib/tauri";
import { activityEventsApi } from "@/api/activity-events";
import type { ReviewNoteResponse } from "@/lib/tauri";
import type { ActivityEventResponse } from "@/api/activity-events.types";

// Mock the Tauri API (review notes)
vi.mock("@/lib/tauri", () => ({
  api: {
    reviews: {
      getTaskStateHistory: vi.fn(),
    },
  },
}));

// Mock the activity events API
vi.mock("@/api/activity-events", () => ({
  activityEventsApi: {
    task: {
      list: vi.fn(),
    },
  },
}));

const mockGetStateHistory = vi.mocked(api.reviews.getTaskStateHistory);
const mockListTaskEvents = vi.mocked(activityEventsApi.task.list);

// ============================================================================
// Test Helpers
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

function createMockReviewNote(
  overrides: Partial<ReviewNoteResponse> = {}
): ReviewNoteResponse {
  return {
    id: "note-1",
    task_id: "task-1",
    reviewer: "ai",
    outcome: "approved",
    summary: "Code review passed",
    notes: "All checks passed",
    issues: null,
    created_at: "2026-02-23T10:00:00+00:00",
    ...overrides,
  };
}

function createMockActivityEvent(
  overrides: Partial<ActivityEventResponse> = {}
): ActivityEventResponse {
  return {
    id: "evt-1",
    taskId: "task-1",
    ideationSessionId: null,
    internalStatus: "executing",
    eventType: "text",
    role: "agent",
    content: "Starting execution...",
    metadata: null,
    createdAt: "2026-02-23T09:00:00+00:00",
    ...overrides,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("useAuditTrail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns loading state initially", () => {
    mockGetStateHistory.mockReturnValue(new Promise(() => {})); // never resolves
    mockListTaskEvents.mockReturnValue(new Promise(() => {}));

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(result.current.entries).toEqual([]);
  });

  it("merges review notes and activity events into unified timeline", async () => {
    const reviewNotes = [
      createMockReviewNote({
        id: "note-1",
        notes: "Review passed",
        created_at: "2026-02-23T12:00:00+00:00",
      }),
    ];
    const activityEvents = [
      createMockActivityEvent({
        id: "evt-1",
        content: "Started execution",
        createdAt: "2026-02-23T11:00:00+00:00",
      }),
    ];

    mockGetStateHistory.mockResolvedValue(reviewNotes);
    mockListTaskEvents.mockResolvedValue({
      events: activityEvents,
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(2);
    // Both types present
    const sources = result.current.entries.map((e) => e.source);
    expect(sources).toContain("review");
    expect(sources).toContain("activity");
  });

  it("sorts entries chronologically (oldest first)", async () => {
    const reviewNotes = [
      createMockReviewNote({
        id: "note-1",
        created_at: "2026-02-23T14:00:00+00:00",
      }),
    ];
    const activityEvents = [
      createMockActivityEvent({
        id: "evt-1",
        createdAt: "2026-02-23T10:00:00+00:00",
      }),
      createMockActivityEvent({
        id: "evt-2",
        createdAt: "2026-02-23T12:00:00+00:00",
      }),
    ];

    mockGetStateHistory.mockResolvedValue(reviewNotes);
    mockListTaskEvents.mockResolvedValue({
      events: activityEvents,
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(3);
    // Oldest first: evt-1 (10:00), evt-2 (12:00), note-1 (14:00)
    expect(result.current.entries[0]!.id).toBe("activity-evt-1");
    expect(result.current.entries[1]!.id).toBe("activity-evt-2");
    expect(result.current.entries[2]!.id).toBe("review-note-1");
  });

  it("handles empty review notes (only activity events)", async () => {
    const activityEvents = [
      createMockActivityEvent({ id: "evt-1" }),
      createMockActivityEvent({ id: "evt-2", createdAt: "2026-02-23T10:00:00+00:00" }),
    ];

    mockGetStateHistory.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue({
      events: activityEvents,
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(2);
    expect(result.current.entries.every((e) => e.source === "activity")).toBe(true);
    expect(result.current.isEmpty).toBe(false);
  });

  it("handles empty activity events (only review notes)", async () => {
    const reviewNotes = [
      createMockReviewNote({ id: "note-1" }),
      createMockReviewNote({
        id: "note-2",
        created_at: "2026-02-23T11:00:00+00:00",
      }),
    ];

    mockGetStateHistory.mockResolvedValue(reviewNotes);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(2);
    expect(result.current.entries.every((e) => e.source === "review")).toBe(true);
    expect(result.current.isEmpty).toBe(false);
  });

  it("handles both empty (empty state)", async () => {
    mockGetStateHistory.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toEqual([]);
    expect(result.current.isEmpty).toBe(true);
  });

  it("properly maps review note fields to AuditEntry", async () => {
    const reviewNote = createMockReviewNote({
      id: "note-42",
      reviewer: "ai",
      outcome: "changes_requested",
      summary: "Needs fixes",
      notes: "Found 3 issues",
      created_at: "2026-02-23T15:30:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([reviewNote]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(1);
    const entry = result.current.entries[0]!;
    expect(entry.id).toBe("review-note-42");
    expect(entry.source).toBe("review");
    expect(entry.timestamp).toBe("2026-02-23T15:30:00+00:00");
    expect(entry.type).toBe("Changes Requested");
    expect(entry.actor).toBe("AI Reviewer");
    // notes takes priority over summary in description
    expect(entry.description).toBe("Found 3 issues");
  });

  it("maps review note with only summary (no notes) to description", async () => {
    const reviewNote = createMockReviewNote({
      id: "note-50",
      outcome: "approved",
      summary: "All good",
      notes: null,
      created_at: "2026-02-23T16:00:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([reviewNote]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const entry = result.current.entries[0]!;
    expect(entry.description).toBe("All good");
  });

  it("properly maps activity event fields to AuditEntry", async () => {
    const activityEvent = createMockActivityEvent({
      id: "evt-77",
      eventType: "tool_call",
      role: "agent",
      content: "Running tests...",
      internalStatus: "executing",
      createdAt: "2026-02-23T16:45:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue({
      events: [activityEvent],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(1);
    const entry = result.current.entries[0]!;
    expect(entry.id).toBe("activity-evt-77");
    expect(entry.source).toBe("activity");
    expect(entry.timestamp).toBe("2026-02-23T16:45:00+00:00");
    expect(entry.type).toBe("tool_call");
    expect(entry.actor).toBe("Agent");
    expect(entry.description).toBe("Running tests...");
    expect(entry.status).toBe("executing");
  });

  it("maps human reviewer correctly", async () => {
    const reviewNote = createMockReviewNote({
      id: "note-60",
      reviewer: "human",
      outcome: "approved",
      created_at: "2026-02-23T17:00:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([reviewNote]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries[0]!.actor).toBe("Human Reviewer");
  });

  it("includes metadata for review notes with issues", async () => {
    const reviewNote = createMockReviewNote({
      id: "note-70",
      outcome: "changes_requested",
      issues: [
        { severity: "error", description: "Bug found", file: null, line: null },
        { severity: "warning", description: "Style issue", file: null, line: null },
      ],
      created_at: "2026-02-23T18:00:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([reviewNote]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries[0]!.metadata).toBe("2 issues found");
  });

  it("includes metadata for single issue", async () => {
    const reviewNote = createMockReviewNote({
      id: "note-71",
      outcome: "changes_requested",
      issues: [
        { severity: "error", description: "Critical bug", file: null, line: null },
      ],
      created_at: "2026-02-23T18:30:00+00:00",
    });

    mockGetStateHistory.mockResolvedValue([reviewNote]);
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries[0]!.metadata).toBe("1 issue found");
  });

  it("does not fetch when disabled", () => {
    const { result } = renderHook(
      () => useAuditTrail("task-1", { enabled: false }),
      { wrapper: createWrapper() }
    );

    expect(result.current.isLoading).toBe(false);
    expect(mockGetStateHistory).not.toHaveBeenCalled();
    expect(mockListTaskEvents).not.toHaveBeenCalled();
  });

  it("does not fetch without taskId", () => {
    const { result } = renderHook(() => useAuditTrail(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockGetStateHistory).not.toHaveBeenCalled();
    expect(mockListTaskEvents).not.toHaveBeenCalled();
  });

  it("returns error from review query", async () => {
    mockGetStateHistory.mockRejectedValue(new Error("Review fetch failed"));
    mockListTaskEvents.mockResolvedValue({
      events: [],
      cursor: null,
      hasMore: false,
    });

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Review fetch failed");
    });
  });

  it("returns error from activity query", async () => {
    mockGetStateHistory.mockResolvedValue([]);
    mockListTaskEvents.mockRejectedValue(new Error("Activity fetch failed"));

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Activity fetch failed");
    });
  });
});
