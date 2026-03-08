/**
 * useVerificationEvents tests
 *
 * Tests for the verificationUpdateSeq feature (B1 fix):
 * 1. Tauri event increments verificationUpdateSeq in store
 * 2. seq > 0 → store overrides stale React Query data in resolvedSession merge logic
 * 3. seq === 0 → React Query data used as-is (no override)
 * 4. Session switch → seq resets (new session starts at 0)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { IdeationSession } from "@/types/ideation";

// ============================================================================
// Mock infrastructure
// ============================================================================

// Capture event bus subscriptions so tests can fire events manually
const subscriptions = new Map<string, ((...args: unknown[]) => void)[]>();

function fireEvent<T>(event: string, payload: T) {
  const handlers = subscriptions.get(event);
  if (handlers) {
    for (const handler of handlers) {
      handler(payload);
    }
  }
}

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: (event: string, handler: (...args: unknown[]) => void) => {
      if (!subscriptions.has(event)) subscriptions.set(event, []);
      subscriptions.get(event)!.push(handler);
      return () => {
        const handlers = subscriptions.get(event);
        if (handlers) {
          const idx = handlers.indexOf(handler);
          if (idx >= 0) handlers.splice(idx, 1);
        }
      };
    },
  }),
}));

const mockInvalidateQueries = vi.fn();
const mockSetQueryData = vi.fn();

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mockInvalidateQueries,
    setQueryData: mockSetQueryData,
  }),
}));

vi.mock("@/hooks/useIdeation", () => ({
  ideationKeys: {
    sessions: () => ["sessions"],
    sessionWithData: (id: string) => ["session", id],
  },
}));

// ============================================================================
// Import hook and store under test (after mocks)
// ============================================================================

import { useVerificationEvents } from "./useVerificationEvents";
import { useIdeationStore } from "@/stores/ideationStore";

// ============================================================================
// Helpers
// ============================================================================

const SESSION_ID = "session-abc";

const createTestSession = (overrides: Partial<IdeationSession> = {}): IdeationSession => ({
  id: SESSION_ID,
  projectId: "project-1",
  title: "Test Session",
  status: "active",
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  archivedAt: null,
  convertedAt: null,
  verificationStatus: "unverified",
  verificationInProgress: false,
  verificationUpdateSeq: 0,
  ...overrides,
});

const makeVerificationEvent = (overrides: Record<string, unknown> = {}) => ({
  session_id: SESSION_ID,
  status: "reviewing",
  in_progress: true,
  gap_score: 42,
  ...overrides,
});

/**
 * Mirrors the resolvedSession merge logic from App.tsx.
 * Used to test the merge semantics without rendering App.
 */
function resolveSession(
  fetchedSession: IdeationSession | undefined,
  activeSession: IdeationSession | null
): IdeationSession | null {
  const isFetchedSessionCurrent = fetchedSession?.id === activeSession?.id;
  const base = isFetchedSessionCurrent && fetchedSession ? fetchedSession : activeSession;

  if (
    base &&
    activeSession &&
    activeSession.id === base.id &&
    (activeSession.verificationUpdateSeq ?? 0) > 0
  ) {
    return {
      ...base,
      verificationStatus: activeSession.verificationStatus ?? base.verificationStatus,
      verificationInProgress: activeSession.verificationInProgress ?? base.verificationInProgress,
      gapScore: activeSession.gapScore !== undefined ? activeSession.gapScore : base.gapScore,
    };
  }

  return base;
}

// ============================================================================
// Tests
// ============================================================================

describe("useVerificationEvents — verificationUpdateSeq", () => {
  beforeEach(() => {
    subscriptions.clear();
    mockInvalidateQueries.mockClear();
    mockSetQueryData.mockClear();
    useIdeationStore.setState({
      sessions: { [SESSION_ID]: createTestSession() },
      activeSessionId: SESSION_ID,
      isLoading: false,
      error: null,
    });
  });

  it("(1) increments verificationUpdateSeq in store when Tauri event fires", () => {
    renderHook(() => useVerificationEvents());

    expect(useIdeationStore.getState().sessions[SESSION_ID]?.verificationUpdateSeq).toBe(0);

    act(() => {
      fireEvent("plan_verification:status_changed", makeVerificationEvent());
    });

    expect(useIdeationStore.getState().sessions[SESSION_ID]?.verificationUpdateSeq).toBe(1);

    // Second event increments again
    act(() => {
      fireEvent("plan_verification:status_changed", makeVerificationEvent({ status: "needs_revision", in_progress: false }));
    });

    expect(useIdeationStore.getState().sessions[SESSION_ID]?.verificationUpdateSeq).toBe(2);
  });
});

describe("resolvedSession merge logic — verificationUpdateSeq guard", () => {
  const staleQuerySession = createTestSession({
    verificationStatus: "unverified",
    verificationInProgress: false,
    gapScore: null,
    verificationUpdateSeq: 0,
  });

  it("(2) seq > 0 → store fields override stale React Query data", () => {
    const storeSession = createTestSession({
      verificationStatus: "needs_revision",
      verificationInProgress: false,
      gapScore: 75,
      verificationUpdateSeq: 2,
    });

    const resolved = resolveSession(staleQuerySession, storeSession);

    expect(resolved?.verificationStatus).toBe("needs_revision");
    expect(resolved?.verificationInProgress).toBe(false);
    expect(resolved?.gapScore).toBe(75);
  });

  it("(3) seq === 0 → React Query data used as-is (no store override)", () => {
    const freshQuerySession = createTestSession({
      verificationStatus: "verified",
      verificationInProgress: false,
      gapScore: 10,
      verificationUpdateSeq: 0,
    });
    const storeSessionNoEvents = createTestSession({
      verificationStatus: "unverified",
      verificationInProgress: false,
      verificationUpdateSeq: 0,
    });

    const resolved = resolveSession(freshQuerySession, storeSessionNoEvents);

    // No seq override → fetchedSession is used, so verificationStatus from query
    expect(resolved?.verificationStatus).toBe("verified");
    expect(resolved?.gapScore).toBe(10);
  });

  it("(4) session switch → new session has seq === 0, React Query data used as-is", () => {
    const newSessionId = "session-new";
    const newSessionFromQuery = createTestSession({
      id: newSessionId,
      verificationStatus: "verified",
      verificationInProgress: false,
      gapScore: 5,
      verificationUpdateSeq: 0,
    });
    // After session switch, the new session in store has no events yet (seq=0)
    const newSessionInStore = createTestSession({
      id: newSessionId,
      verificationStatus: "unverified",
      verificationInProgress: false,
      verificationUpdateSeq: 0,
    });

    const resolved = resolveSession(newSessionFromQuery, newSessionInStore);

    // seq === 0 → no merge, base (fetchedSession) is returned as-is
    expect(resolved?.id).toBe(newSessionId);
    expect(resolved?.verificationStatus).toBe("verified");
    expect(resolved?.gapScore).toBe(5);
  });
});
