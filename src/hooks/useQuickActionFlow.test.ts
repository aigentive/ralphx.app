/**
 * useQuickActionFlow hook tests
 *
 * Tests the generic quick action state machine:
 * - State transitions: idle → confirming → creating → success → idle
 * - Cancel/dismiss transitions back to idle
 * - Error handling transitions back to idle with error message
 * - Entity ID capture on success
 * - isBlocking correctness (true when not idle)
 * - Edge cases (double-confirm, cancel during idle)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useQuickActionFlow } from "./useQuickActionFlow";
import type { QuickAction } from "./useQuickActionFlow";

describe("useQuickActionFlow", () => {
  let mockAction: QuickAction;

  beforeEach(() => {
    vi.clearAllMocks();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const mockIcon = vi.fn() as any;
    mockAction = {
      id: "test-action",
      label: "Test Action",
      icon: mockIcon,
      description: (query) => `"${query}"`,
      isVisible: (query) => query.trim().length > 0,
      execute: vi.fn().mockResolvedValue("test-entity-123"),
      creatingLabel: "Creating...",
      successLabel: "Created!",
      viewLabel: "View",
      navigateTo: vi.fn(),
    };
  });

  describe("initial state", () => {
    it("should start in idle state", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.flowState).toBe("idle");
      expect(result.current.createdEntityId).toBeNull();
      expect(result.current.error).toBeNull();
    });

    it("should have isBlocking=false when idle", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.isBlocking).toBe(false);
    });

    it("should provide all transition methods", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.startConfirmation).toBeInstanceOf(Function);
      expect(result.current.confirm).toBeInstanceOf(Function);
      expect(result.current.cancel).toBeInstanceOf(Function);
      expect(result.current.viewEntity).toBeInstanceOf(Function);
      expect(result.current.dismiss).toBeInstanceOf(Function);
    });
  });

  describe("state transitions", () => {
    it("should transition from idle to confirming when startConfirmation is called", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.flowState).toBe("confirming");
      expect(result.current.isBlocking).toBe(true);
    });

    it("should transition from confirming to idle when cancel is called", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

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

    it("should transition from confirming to creating when confirm is called", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      let confirmPromise: Promise<void>;
      act(() => {
        confirmPromise = result.current.confirm("test query");
      });

      // Should be in creating state while execute() is running
      expect(result.current.flowState).toBe("creating");
      expect(result.current.isBlocking).toBe(true);

      await act(async () => {
        await confirmPromise;
      });

      expect(mockAction.execute).toHaveBeenCalledWith("test query");
    });

    it("should transition from creating to success on successful execution", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("success");
      expect(result.current.createdEntityId).toBe("test-entity-123");
      expect(result.current.error).toBeNull();
      expect(result.current.isBlocking).toBe(true);
    });

    it("should transition from success to idle when dismiss is called", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("success");

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
      expect(result.current.createdEntityId).toBeNull();
    });

    it("should transition from success to idle when viewEntity is called", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("success");

      act(() => {
        result.current.viewEntity();
      });

      expect(mockAction.navigateTo).toHaveBeenCalledWith("test-entity-123");
      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });
  });

  describe("error handling", () => {
    it("should transition to idle with error message when execution fails", async () => {
      const errorAction = {
        ...mockAction,
        execute: vi.fn().mockRejectedValue(new Error("Network error")),
      };

      const { result } = renderHook(() => useQuickActionFlow(errorAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.error).toBe("Network error");
      expect(result.current.createdEntityId).toBeNull();
      expect(result.current.isBlocking).toBe(false);
    });

    it("should handle non-Error exceptions", async () => {
      const errorAction = {
        ...mockAction,
        execute: vi.fn().mockRejectedValue("String error"),
      };

      const { result } = renderHook(() => useQuickActionFlow(errorAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.error).toBe("An error occurred");
      expect(result.current.isBlocking).toBe(false);
    });

    it("should clear error when startConfirmation is called again", async () => {
      const errorAction = {
        ...mockAction,
        execute: vi.fn().mockRejectedValue(new Error("Network error")),
      };

      const { result } = renderHook(() => useQuickActionFlow(errorAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.error).toBe("Network error");

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.error).toBeNull();
      expect(result.current.flowState).toBe("confirming");
    });
  });

  describe("isBlocking behavior", () => {
    it("should be false only in idle state", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });

    it("should be true in confirming state", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.flowState).toBe("confirming");
      expect(result.current.isBlocking).toBe(true);
    });

    it("should be true in creating state", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      let confirmPromise: Promise<void>;
      act(() => {
        confirmPromise = result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("creating");
      expect(result.current.isBlocking).toBe(true);

      await act(async () => {
        await confirmPromise;
      });
    });

    it("should be true in success state", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      expect(result.current.flowState).toBe("success");
      expect(result.current.isBlocking).toBe(true);
    });

    it("should return to false after dismiss", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("test query");
      });

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });
  });

  describe("edge cases", () => {
    it("should handle double-confirm (confirm called twice rapidly)", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await Promise.all([
          result.current.confirm("query 1"),
          result.current.confirm("query 2"),
        ]);
      });

      // execute should only be called once (first call)
      expect(mockAction.execute).toHaveBeenCalledTimes(1);
      expect(mockAction.execute).toHaveBeenCalledWith("query 1");
    });

    it("should handle cancel when already in idle state (no-op)", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.flowState).toBe("idle");

      act(() => {
        result.current.cancel();
      });

      expect(result.current.flowState).toBe("idle");
      expect(result.current.isBlocking).toBe(false);
    });

    it("should handle dismiss when already in idle state (no-op)", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.flowState).toBe("idle");

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.flowState).toBe("idle");
    });

    it("should handle viewEntity when no entity ID exists (no-op)", () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      act(() => {
        result.current.viewEntity();
      });

      expect(mockAction.navigateTo).not.toHaveBeenCalled();
      expect(result.current.flowState).toBe("idle");
    });

    it("should handle confirm called from idle state (skip to creating)", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      expect(result.current.flowState).toBe("idle");

      await act(async () => {
        await result.current.confirm("direct query");
      });

      expect(mockAction.execute).toHaveBeenCalledWith("direct query");
      expect(result.current.flowState).toBe("success");
      expect(result.current.createdEntityId).toBe("test-entity-123");
    });

    it("should preserve entity ID across multiple dismiss/startConfirmation cycles", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      // First cycle
      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("query 1");
      });

      expect(result.current.createdEntityId).toBe("test-entity-123");

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.createdEntityId).toBeNull();

      // Second cycle with different entity ID
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (mockAction.execute as any).mockResolvedValue("test-entity-456");

      act(() => {
        result.current.startConfirmation();
      });

      await act(async () => {
        await result.current.confirm("query 2");
      });

      expect(result.current.createdEntityId).toBe("test-entity-456");
    });
  });

  describe("callback stability", () => {
    it("should maintain stable callback references across state changes", async () => {
      const { result } = renderHook(() => useQuickActionFlow(mockAction));

      const initialCallbacks = {
        startConfirmation: result.current.startConfirmation,
        confirm: result.current.confirm,
        cancel: result.current.cancel,
        viewEntity: result.current.viewEntity,
        dismiss: result.current.dismiss,
      };

      act(() => {
        result.current.startConfirmation();
      });

      expect(result.current.startConfirmation).toBe(
        initialCallbacks.startConfirmation
      );
      expect(result.current.confirm).toBe(initialCallbacks.confirm);
      expect(result.current.cancel).toBe(initialCallbacks.cancel);
      expect(result.current.viewEntity).toBe(initialCallbacks.viewEntity);
      expect(result.current.dismiss).toBe(initialCallbacks.dismiss);

      await act(async () => {
        await result.current.confirm("test");
      });

      expect(result.current.startConfirmation).toBe(
        initialCallbacks.startConfirmation
      );
      expect(result.current.confirm).toBe(initialCallbacks.confirm);
      expect(result.current.cancel).toBe(initialCallbacks.cancel);
      expect(result.current.viewEntity).toBe(initialCallbacks.viewEntity);
      expect(result.current.dismiss).toBe(initialCallbacks.dismiss);
    });
  });

  describe("action reference changes", () => {
    it("should handle action reference changing", () => {
      const { result, rerender } = renderHook(
        (props: { action: QuickAction }) => useQuickActionFlow(props.action),
        {
          initialProps: { action: mockAction },
        }
      );

      expect(result.current.flowState).toBe("idle");

      const newAction = {
        ...mockAction,
        id: "new-action",
      };

      rerender({ action: newAction });

      expect(result.current.flowState).toBe("idle");
    });

    it("should use the latest action when confirm is called", async () => {
      const action1 = {
        ...mockAction,
        execute: vi.fn().mockResolvedValue("entity-1"),
      };

      const action2 = {
        ...mockAction,
        execute: vi.fn().mockResolvedValue("entity-2"),
      };

      const { result, rerender } = renderHook(
        (props: { action: QuickAction }) => useQuickActionFlow(props.action),
        {
          initialProps: { action: action1 },
        }
      );

      act(() => {
        result.current.startConfirmation();
      });

      // Change action while in confirming state
      rerender({ action: action2 });

      await act(async () => {
        await result.current.confirm("test");
      });

      // Should use the latest action
      expect(action2.execute).toHaveBeenCalledWith("test");
      expect(action1.execute).not.toHaveBeenCalled();
      expect(result.current.createdEntityId).toBe("entity-2");
    });
  });
});
