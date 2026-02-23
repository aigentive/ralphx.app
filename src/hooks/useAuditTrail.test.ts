/**
 * useAuditTrail hook tests
 *
 * Tests the hook that merges state transitions, review notes, and activity events
 * into a unified, chronologically-sorted audit trail with phase derivation.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import { useAuditTrail, derivePhases } from "./useAuditTrail";
import { api } from "@/lib/tauri";
import { activityEventsApi } from "@/api/activity-events";
import type { ReviewNoteResponse } from "@/lib/tauri";
import type { ActivityEventResponse } from "@/api/activity-events.types";
import type { StateTransition } from "@/api/tasks";

// Mock the Tauri API (review notes + state transitions)
vi.mock("@/lib/tauri", () => ({
  api: {
    reviews: {
      getTaskStateHistory: vi.fn(),
    },
    tasks: {
      getStateTransitions: vi.fn(),
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
const mockGetStateTransitions = vi.mocked(api.tasks.getStateTransitions);
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

function createMockTransition(
  overrides: Partial<StateTransition> = {}
): StateTransition {
  return {
    fromStatus: null,
    toStatus: "executing",
    trigger: "system",
    timestamp: "2026-02-23T09:00:00+00:00",
    ...overrides,
  };
}

const emptyPage = { events: [] as ActivityEventResponse[], cursor: null, hasMore: false };

// ============================================================================
// Hook Tests
// ============================================================================

describe("useAuditTrail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns loading state initially", () => {
    mockGetStateHistory.mockReturnValue(new Promise(() => {}));
    mockGetStateTransitions.mockReturnValue(new Promise(() => {}));
    mockListTaskEvents.mockReturnValue(new Promise(() => {}));

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(result.current.entries).toEqual([]);
    expect(result.current.phases).toEqual([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toEqual([]);
    expect(result.current.isEmpty).toBe(true);
    expect(result.current.phases).toEqual([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    mockGetStateTransitions.mockResolvedValue([]);
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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

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
    expect(mockGetStateTransitions).not.toHaveBeenCalled();
    expect(mockListTaskEvents).not.toHaveBeenCalled();
  });

  it("does not fetch without taskId", () => {
    const { result } = renderHook(() => useAuditTrail(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockGetStateHistory).not.toHaveBeenCalled();
    expect(mockGetStateTransitions).not.toHaveBeenCalled();
    expect(mockListTaskEvents).not.toHaveBeenCalled();
  });

  it("returns error from review query", async () => {
    mockGetStateHistory.mockRejectedValue(new Error("Review fetch failed"));
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Review fetch failed");
    });
  });

  it("returns error from activity query", async () => {
    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockRejectedValue(new Error("Activity fetch failed"));

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Activity fetch failed");
    });
  });

  it("returns error from transitions query", async () => {
    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockRejectedValue(new Error("Transitions fetch failed"));
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Transitions fetch failed");
    });
  });

  // ============================================================================
  // State Transition Mapping
  // ============================================================================

  it("maps state transitions to AuditEntry with source='transition'", async () => {
    const transitions: StateTransition[] = [
      createMockTransition({
        fromStatus: "backlog",
        toStatus: "executing",
        trigger: "system",
        timestamp: "2026-02-23T09:00:00+00:00",
      }),
    ];

    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue(transitions);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.entries).toHaveLength(1);
    const entry = result.current.entries[0]!;
    expect(entry.source).toBe("transition");
    expect(entry.type).toBe("State Change");
    expect(entry.actor).toBe("System");
    expect(entry.description).toBe("Backlog \u2192 Executing");
    expect(entry.fromStatus).toBe("backlog");
    expect(entry.toStatus).toBe("executing");
  });

  it("maps initial transition (null fromStatus) as 'Created'", async () => {
    const transitions: StateTransition[] = [
      createMockTransition({
        fromStatus: null,
        toStatus: "backlog",
        trigger: "user",
        timestamp: "2026-02-23T08:00:00+00:00",
      }),
    ];

    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue(transitions);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const entry = result.current.entries[0]!;
    expect(entry.description).toBe("Created \u2192 Backlog");
    expect(entry.actor).toBe("User");
  });

  // ============================================================================
  // Three-source Merge
  // ============================================================================

  it("merges all three sources chronologically", async () => {
    const transitions: StateTransition[] = [
      createMockTransition({
        fromStatus: "backlog",
        toStatus: "executing",
        timestamp: "2026-02-23T09:00:00+00:00",
      }),
    ];
    const reviewNotes = [
      createMockReviewNote({
        id: "note-1",
        created_at: "2026-02-23T11:00:00+00:00",
      }),
    ];
    const activityEvents = [
      createMockActivityEvent({
        id: "evt-1",
        createdAt: "2026-02-23T10:00:00+00:00",
      }),
    ];

    mockGetStateTransitions.mockResolvedValue(transitions);
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
    expect(result.current.entries[0]!.source).toBe("transition");
    expect(result.current.entries[1]!.source).toBe("activity");
    expect(result.current.entries[2]!.source).toBe("review");
  });

  // ============================================================================
  // Phase derivation through hook
  // ============================================================================

  it("returns phases derived from state transitions", async () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: "backlog", toStatus: "executing", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "executing", toStatus: "pending_review", timestamp: "2026-02-23T10:00:00+00:00" }),
      createMockTransition({ fromStatus: "reviewing", toStatus: "approved", timestamp: "2026-02-23T11:00:00+00:00" }),
      createMockTransition({ fromStatus: "approved", toStatus: "pending_merge", timestamp: "2026-02-23T12:00:00+00:00" }),
      createMockTransition({ fromStatus: "merging", toStatus: "merged", timestamp: "2026-02-23T13:00:00+00:00" }),
    ];

    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue(transitions);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.phases.map((p) => p.label)).toEqual([
      "Execution #1",
      "Review #1",
      "Merge",
    ]);
  });

  it("returns empty phases when no transitions", async () => {
    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue([]);
    mockListTaskEvents.mockResolvedValue(emptyPage);

    const { result } = renderHook(() => useAuditTrail("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.phases).toEqual([]);
  });

  // ============================================================================
  // Phase ID Assignment
  // ============================================================================

  it("tags entries with correct phaseId based on timestamp", async () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: "backlog", toStatus: "executing", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "executing", toStatus: "pending_review", timestamp: "2026-02-23T11:00:00+00:00" }),
    ];
    const activityEvents = [
      createMockActivityEvent({ id: "evt-1", createdAt: "2026-02-23T10:00:00+00:00" }),
      createMockActivityEvent({ id: "evt-2", createdAt: "2026-02-23T12:00:00+00:00" }),
    ];

    mockGetStateHistory.mockResolvedValue([]);
    mockGetStateTransitions.mockResolvedValue(transitions);
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

    // evt-1 at 10:00 falls within Execution #1 (09:00-11:00)
    const evt1 = result.current.entries.find((e) => e.id === "activity-evt-1");
    expect(evt1!.phaseId).toBe("phase-execution-1");

    // evt-2 at 12:00 falls within Review #1 (11:00-null)
    const evt2 = result.current.entries.find((e) => e.id === "activity-evt-2");
    expect(evt2!.phaseId).toBe("phase-review-1");
  });
});

// ============================================================================
// Pure Function: derivePhases
// ============================================================================

describe("derivePhases", () => {
  it("derives phases from linear transition sequence", () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: "backlog", toStatus: "executing", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "executing", toStatus: "pending_review", timestamp: "2026-02-23T10:00:00+00:00" }),
      createMockTransition({ fromStatus: "reviewing", toStatus: "approved", timestamp: "2026-02-23T11:00:00+00:00" }),
      createMockTransition({ fromStatus: "approved", toStatus: "pending_merge", timestamp: "2026-02-23T12:00:00+00:00" }),
      createMockTransition({ fromStatus: "merging", toStatus: "merged", timestamp: "2026-02-23T13:00:00+00:00" }),
    ];

    const phases = derivePhases(transitions);

    expect(phases.map((p) => p.label)).toEqual(["Execution #1", "Review #1", "Merge"]);
    expect(phases[0]!.type).toBe("execution");
    expect(phases[1]!.type).toBe("review");
    expect(phases[2]!.type).toBe("merge");
  });

  it("derives phases for revision cycle", () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: null, toStatus: "executing", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "executing", toStatus: "revision_needed", timestamp: "2026-02-23T10:00:00+00:00" }),
      createMockTransition({ fromStatus: "revision_needed", toStatus: "re_executing", timestamp: "2026-02-23T11:00:00+00:00" }),
      createMockTransition({ fromStatus: "re_executing", toStatus: "reviewing", timestamp: "2026-02-23T12:00:00+00:00" }),
      createMockTransition({ fromStatus: "reviewing", toStatus: "approved", timestamp: "2026-02-23T13:00:00+00:00" }),
    ];

    const phases = derivePhases(transitions);

    expect(phases.map((p) => p.label)).toEqual([
      "Execution #1",
      "Review #1",
      "Execution #2",
      "Review #2",
    ]);
  });

  it("returns empty phases for empty transitions", () => {
    expect(derivePhases([])).toEqual([]);
  });

  it("returns empty phases when only idle transitions", () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: null, toStatus: "backlog", timestamp: "2026-02-23T08:00:00+00:00" }),
      createMockTransition({ fromStatus: "backlog", toStatus: "ready", timestamp: "2026-02-23T09:00:00+00:00" }),
    ];

    expect(derivePhases(transitions)).toEqual([]);
  });

  it("sets phase startTime and endTime correctly", () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: "backlog", toStatus: "executing", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "executing", toStatus: "pending_review", timestamp: "2026-02-23T10:00:00+00:00" }),
    ];

    const phases = derivePhases(transitions);

    expect(phases[0]!.startTime).toBe(new Date("2026-02-23T09:00:00+00:00").getTime());
    expect(phases[0]!.endTime).toBe(new Date("2026-02-23T10:00:00+00:00").getTime());
    expect(phases[1]!.startTime).toBe(new Date("2026-02-23T10:00:00+00:00").getTime());
    expect(phases[1]!.endTime).toBeNull(); // last phase is open-ended
  });

  it("preserves conversationId and agentRunId on phases", () => {
    const transitions: StateTransition[] = [
      createMockTransition({
        fromStatus: "backlog",
        toStatus: "executing",
        timestamp: "2026-02-23T09:00:00+00:00",
        conversationId: "conv-1",
        agentRunId: "run-1",
      }),
    ];

    const phases = derivePhases(transitions);

    expect(phases[0]!.conversationId).toBe("conv-1");
    expect(phases[0]!.agentRunId).toBe("run-1");
  });

  it("updates phase status to latest transition in same group", () => {
    const transitions: StateTransition[] = [
      createMockTransition({ fromStatus: "approved", toStatus: "pending_merge", timestamp: "2026-02-23T09:00:00+00:00" }),
      createMockTransition({ fromStatus: "pending_merge", toStatus: "merging", timestamp: "2026-02-23T09:30:00+00:00" }),
      createMockTransition({ fromStatus: "merging", toStatus: "merged", timestamp: "2026-02-23T10:00:00+00:00" }),
    ];

    const phases = derivePhases(transitions);

    expect(phases).toHaveLength(1);
    expect(phases[0]!.label).toBe("Merge");
    expect(phases[0]!.status).toBe("merged");
  });
});
