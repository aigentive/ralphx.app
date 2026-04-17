/**
 * useVerificationBootstrap tests
 *
 * Covers:
 * 1. Calls getPendingVerificationConfirmations with the active project ID on mount
 * 2. Calls hydrateVerificationQueue with the returned session IDs on success
 * 3. console.warn on API failure (not throw)
 * 4. Re-fetches when activeProjectId changes
 * 5. No-op when activeProjectId is null
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

// ============================================================================
// Hoisted mocks (must be declared before vi.mock factories)
// ============================================================================

const { mockActiveProjectId, mockHydrateVerificationQueue } = vi.hoisted(() => ({
  mockActiveProjectId: { value: "project-1" as string | null },
  mockHydrateVerificationQueue: vi.fn(),
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: (selector: (s: { activeProjectId: string | null }) => unknown) =>
    selector({ activeProjectId: mockActiveProjectId.value }),
}));

vi.mock("@/stores/uiStore", () => ({
  useUiStore: (selector: (s: { hydrateVerificationQueue: typeof mockHydrateVerificationQueue }) => unknown) =>
    selector({ hydrateVerificationQueue: mockHydrateVerificationQueue }),
}));

const mockGetPendingVerificationConfirmations = vi.fn();

vi.mock("@/api/verification", () => ({
  verificationApi: {
    getPendingVerificationConfirmations: (...args: unknown[]) =>
      mockGetPendingVerificationConfirmations(...args),
  },
}));

vi.mock("@/lib/logger", () => ({
  logger: { debug: vi.fn() },
}));

// ============================================================================
// Import under test (after mocks)
// ============================================================================

import { useVerificationBootstrap } from "./useVerificationBootstrap";

// ============================================================================
// Helpers
// ============================================================================

const PENDING_ITEMS = {
  sessions: [
    { session_id: "session-1", session_title: "Test Session 1", plan_artifact_id: "plan-1", available_specialists: [] },
    { session_id: "session-2", session_title: "Test Session 2", plan_artifact_id: "plan-2", available_specialists: [] },
  ],
};

// ============================================================================
// Tests
// ============================================================================

describe("useVerificationBootstrap", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockActiveProjectId.value = "project-1";
    mockGetPendingVerificationConfirmations.mockResolvedValue(PENDING_ITEMS);
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
  });

  it("calls getPendingVerificationConfirmations with the active project ID on mount", async () => {
    renderHook(() => useVerificationBootstrap());

    await act(async () => {
      await Promise.resolve();
    });

    expect(mockGetPendingVerificationConfirmations).toHaveBeenCalledWith("project-1");
  });

  it("calls hydrateVerificationQueue with session IDs from the API response", async () => {
    renderHook(() => useVerificationBootstrap());

    await act(async () => {
      await Promise.resolve();
    });

    expect(mockHydrateVerificationQueue).toHaveBeenCalledWith(["session-1", "session-2"]);
  });

  it("console.warn on API failure — does not throw", async () => {
    const apiError = new Error("Network error");
    mockGetPendingVerificationConfirmations.mockRejectedValue(apiError);

    const { result } = renderHook(() => useVerificationBootstrap());
    // Ensure no render error
    expect(result.error).toBeUndefined();

    await act(async () => {
      await Promise.resolve();
    });

    expect(console.warn).toHaveBeenCalledWith(
      "[VerificationBootstrap] Failed to hydrate verification queue:",
      "Network error"
    );
    expect(mockHydrateVerificationQueue).not.toHaveBeenCalled();
  });

  it("does not call the API when activeProjectId is null", async () => {
    mockActiveProjectId.value = null;

    renderHook(() => useVerificationBootstrap());

    await act(async () => {
      await Promise.resolve();
    });

    expect(mockGetPendingVerificationConfirmations).not.toHaveBeenCalled();
    expect(mockHydrateVerificationQueue).not.toHaveBeenCalled();
  });

  it("re-fetches when activeProjectId changes", async () => {
    const { rerender } = renderHook(() => useVerificationBootstrap());

    await act(async () => {
      await Promise.resolve();
    });

    expect(mockGetPendingVerificationConfirmations).toHaveBeenCalledTimes(1);
    expect(mockGetPendingVerificationConfirmations).toHaveBeenCalledWith("project-1");

    // Switch to a different project
    mockActiveProjectId.value = "project-2";
    rerender();

    await act(async () => {
      await Promise.resolve();
    });

    expect(mockGetPendingVerificationConfirmations).toHaveBeenCalledTimes(2);
    expect(mockGetPendingVerificationConfirmations).toHaveBeenCalledWith("project-2");
  });
});
