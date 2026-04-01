/**
 * useQuickActionFlow hook tests
 *
 * Tests for the generic quick action state machine hook
 * that manages: idle → confirming → creating → success flow
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { Sparkles } from "lucide-react";
import { useQuickActionFlow } from "./useQuickActionFlow";
import type { QuickAction } from "./useQuickActionFlow";

describe("useQuickActionFlow", () => {
  const mockNavigate = vi.fn();
  const mockExecute = vi.fn();

  const createMockAction = (overrides?: Partial<QuickAction>): QuickAction => ({
    id: "test-action",
    label: "Test Action",
    icon: Sparkles,
    description: (query: string) => `Create test for "${query}"`,
    isVisible: (query: string) => query.length > 0,
    execute: mockExecute,
    creatingLabel: "Creating test...",
    successLabel: "Test created!",
    viewLabel: "View Test",
    navigateTo: mockNavigate,
    ...overrides,
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("should start in idle state", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      expect(result.current.flowState).toBe("idle");
      expect(result.current.createdEntityId).toBeNull();
      expect(result.current.error).toBeNull();
      expect(result.current.isBlocking).toBe(false);
    });
  });

  describe("state transitions", () => {
    it("should transition from idle to confirming", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.flowState).toBe("confirming");
      expect(result.current.isBlocking).toBe(true);
    });

    it("should transition from confirming back to idle on cancel", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });
      expect(result.current.flowState).toBe("confirming");

      act(() => {
        result.current.cancel();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });

    it("should transition from confirming → creating → success on successful confirm", async () => {
      mockExecute.mockResolvedValueOnce("entity-123");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      expect(result.current.createdEntityId).toBe("entity-123");
      expect(result.current.error).toBeNull();
      expect(result.current.isBlocking).toBe(true);
      expect(mockExecute).toHaveBeenCalledWith("test query");
    });

    it("should transition from creating back to idle on error", async () => {
      const error = new Error("Creation failed");
      mockExecute.mockRejectedValueOnce(error);
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("idle");
      });

      expect(result.current.error).toBe("Creation failed");
      expect(result.current.createdEntityId).toBeNull();
      expect(result.current.isBlocking).toBe(false);
    });

    it("should transition from success to idle on dismiss", async () => {
      mockExecute.mockResolvedValueOnce("entity-123");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.createdEntityId).toBeNull();
      expect(result.current.isBlocking).toBe(false);
    });

    it("should transition from success to idle on cancel", async () => {
      mockExecute.mockResolvedValueOnce("entity-123");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      act(() => {
        result.current.cancel();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });

    it("should call navigateTo and transition to idle on viewEntity", async () => {
      mockExecute.mockResolvedValueOnce("entity-123");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      act(() => {
        result.current.viewEntity();
      });

      expect(mockNavigate).toHaveBeenCalledWith("entity-123");
      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });
  });

  describe("isBlocking derived state", () => {
    it("should be false when idle", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      expect(result.current.isBlocking).toBe(false);
    });

    it("should be true when confirming", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.isBlocking).toBe(true);
    });

    it("should be true when creating", async () => {
      let resolveExecute: (value: string) => void;
      const executePromise = new Promise<string>((resolve) => {
        resolveExecute = resolve;
      });
      mockExecute.mockReturnValueOnce(executePromise);

      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      act(() => {
        void result.current.confirm("test query");
      });

      // During async execution (creating state)
      await waitFor(() => {
        expect(result.current.flowState).toBe("creating");
      });
      expect(result.current.isBlocking).toBe(true);

      // Resolve the promise
      act(() => {
        resolveExecute("entity-123");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });
    });

    it("should be true when in success state", async () => {
      mockExecute.mockResolvedValueOnce("entity-123");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      expect(result.current.isBlocking).toBe(true);
    });
  });

  describe("error handling", () => {
    it("should handle string errors from execute", async () => {
      mockExecute.mockRejectedValueOnce("String error");
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.error).toBe("String error");
      });
    });

    it("should handle Error objects from execute", async () => {
      const error = new Error("Detailed error");
      mockExecute.mockRejectedValueOnce(error);
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.error).toBe("Detailed error");
      });
    });

    it("should handle unknown error types from execute", async () => {
      mockExecute.mockRejectedValueOnce({ code: 42 });
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.error).toBe("An unknown error occurred");
      });
    });

    it("should clear error when transitioning back to success", async () => {
      // First attempt fails
      mockExecute.mockRejectedValueOnce(new Error("First error"));
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.error).toBe("First error");
      });

      // Second attempt succeeds
      mockExecute.mockResolvedValueOnce("entity-123");

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      await waitFor(() => {
        expect(result.current.flowState).toBe("success");
      });

      expect(result.current.error).toBeNull();
    });
  });

  describe("edge cases", () => {
    it("should not call navigateTo if no entity was created", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.viewEntity();
      });

      expect(mockNavigate).not.toHaveBeenCalled();
    });

    it("should handle multiple startConfirmation calls", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      act(() => {
        result.current.startConfirmation();
        result.current.startConfirmation();
        result.current.startConfirmation();
      });

      expect(result.current.flowState).toBe("confirming");
    });

    it("should ignore cancel when in idle state", () => {
      const action = createMockAction();
      const { result } = renderHook(() => useQuickActionFlow(action));

      expect(result.current.flowState).toBe("idle");

      act(() => {
        result.current.cancel();
      });

      expect(result.current.flowState).toBe("idle");
    });
  });
});
