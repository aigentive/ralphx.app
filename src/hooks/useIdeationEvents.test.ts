/**
 * useIdeationEvents tests
 *
 * Tests for ideation:session_created event subscription:
 * 1. Valid payload triggers invalidateQueries with ideationKeys.sessions()
 * 2. Malformed payload (missing sessionId) rejected gracefully by Zod without crash
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

// ============================================================================
// Mock infrastructure
// ============================================================================

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
    emit: vi.fn(),
  }),
}));

const mockInvalidateQueries = vi.fn().mockResolvedValue(undefined);

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mockInvalidateQueries,
  }),
}));

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: (selector: (s: { updateSession: ReturnType<typeof vi.fn> }) => unknown) =>
    selector({ updateSession: vi.fn() }),
}));

vi.mock("@/hooks/useIdeation", () => ({
  ideationKeys: {
    sessions: () => ["sessions"],
  },
}));

vi.mock("@/hooks/useDependencyGraph", () => ({
  dependencyKeys: {
    graphs: () => ["dependency-graphs"],
  },
}));

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useIdeationEvents } from "./useIdeationEvents";

// ============================================================================
// Tests
// ============================================================================

describe("useIdeationEvents — ideation:session_created", () => {
  beforeEach(() => {
    subscriptions.clear();
    mockInvalidateQueries.mockClear();
  });

  it("(1) valid payload triggers invalidateQueries with ideationKeys.sessions()", () => {
    renderHook(() => useIdeationEvents());

    act(() => {
      fireEvent("ideation:session_created", { sessionId: "test-123", projectId: "proj-456" });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({ queryKey: ["sessions"] });
  });

  it("(2) malformed payload (missing sessionId) rejected by Zod without crash", () => {
    renderHook(() => useIdeationEvents());

    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    act(() => {
      // Missing sessionId — Zod should reject this without throwing
      fireEvent("ideation:session_created", { projectId: "proj-456" });
    });

    expect(consoleError).toHaveBeenCalledWith(
      expect.stringContaining("Invalid ideation:session_created event:"),
      expect.any(String)
    );
    // invalidateQueries must NOT have been called for the malformed payload
    expect(mockInvalidateQueries).not.toHaveBeenCalledWith({ queryKey: ["sessions"] });

    consoleError.mockRestore();
  });
});
