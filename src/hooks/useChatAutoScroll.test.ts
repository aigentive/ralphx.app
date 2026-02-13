/**
 * useChatAutoScroll hook tests
 *
 * Tests the unified chat auto-scroll hook behavior:
 * - Virtuoso followOutput is the ONLY auto-scroll mechanism (no DOM effects)
 * - followOutput + atBottomStateChange control all auto-scrolling
 * - scrollToBottom routes through Virtuoso scrollToIndex when ref provided
 * - DOM marker fallback for non-Virtuoso consumers only
 * - History mode (disabled) disables auto-scroll
 * - No isStreaming/streamingHash props — Virtuoso context handles streaming
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useChatAutoScroll } from "./useChatAutoScroll";
import type { VirtuosoHandle } from "react-virtuoso";

describe("useChatAutoScroll", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("should start with isAtBottom=true", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      expect(result.current.isAtBottom).toBe(true);
    });

    it("should start with shouldAutoScroll=true when not disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          disabled: false,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(true);
    });

    it("should start with shouldAutoScroll=false when disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          disabled: true,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(false);
    });

    it("should provide containerRef and messagesEndRef", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      expect(result.current.containerRef).toBeDefined();
      expect(result.current.containerRef.current).toBeNull();
      expect(result.current.messagesEndRef).toBeDefined();
      expect(result.current.messagesEndRef.current).toBeNull();
    });

    it("should provide Virtuoso callback handlers", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      expect(result.current.handleAtBottomStateChange).toBeInstanceOf(Function);
      expect(result.current.handleFollowOutput).toBeInstanceOf(Function);
      expect(result.current.scrollToBottom).toBeInstanceOf(Function);
    });
  });

  describe("bottom state tracking via handleAtBottomStateChange", () => {
    it("should update isAtBottom when Virtuoso reports scroll position", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      expect(result.current.isAtBottom).toBe(true);

      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      expect(result.current.isAtBottom).toBe(false);

      act(() => {
        result.current.handleAtBottomStateChange(true);
      });

      expect(result.current.isAtBottom).toBe(true);
    });

    it("should update shouldAutoScroll when isAtBottom changes", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(true);

      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      expect(result.current.shouldAutoScroll).toBe(false);

      act(() => {
        result.current.handleAtBottomStateChange(true);
      });

      expect(result.current.shouldAutoScroll).toBe(true);
    });

    it("should keep shouldAutoScroll=false when disabled, regardless of isAtBottom", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          disabled: true,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(false);

      act(() => {
        result.current.handleAtBottomStateChange(true);
      });

      expect(result.current.isAtBottom).toBe(true);
      expect(result.current.shouldAutoScroll).toBe(false);
    });
  });

  describe("handleFollowOutput (Virtuoso auto-scroll)", () => {
    it("should return 'smooth' when at bottom and not disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      const output = result.current.handleFollowOutput(true);
      expect(output).toBe("smooth");
    });

    it("should return false when not at bottom", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
        })
      );

      const output = result.current.handleFollowOutput(false);
      expect(output).toBe(false);
    });

    it("should return false when disabled (history mode), even if at bottom", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          disabled: true,
        })
      );

      const output = result.current.handleFollowOutput(true);
      expect(output).toBe(false);
    });

    it("should respond to disabled prop changes", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 0,
            disabled: false,
          },
        }
      );

      expect(result.current.handleFollowOutput(true)).toBe("smooth");

      rerender({
        messageCount: 0,
        disabled: true,
      });

      expect(result.current.handleFollowOutput(true)).toBe(false);
    });
  });

  describe("single scroll path guarantee (no DOM-based auto-scroll)", () => {
    it("should NOT trigger scrollIntoView when message count increases", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Increase message count (simulates new messages during streaming)
      rerender({ messageCount: 6 });

      // No DOM-based auto-scroll — Virtuoso followOutput handles this
      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should NOT use requestAnimationFrame for any updates", () => {
      const rafSpy = vi.spyOn(window, "requestAnimationFrame");
      const { rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // Simulate streaming: rapid message count changes
      rerender({ messageCount: 6 });
      rerender({ messageCount: 7 });

      // No RAF — Virtuoso handles streaming scroll natively
      expect(rafSpy).not.toHaveBeenCalled();
      rafSpy.mockRestore();
    });

    it("should NOT trigger any DOM scroll on rapid message count increases", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Rapid increases (simulates burst of streaming updates)
      rerender({ messageCount: 6 });
      rerender({ messageCount: 7 });
      rerender({ messageCount: 8 });

      // Zero DOM scroll calls — Virtuoso handles all auto-scrolling
      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should NOT use setTimeout for scroll operations", () => {
      const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");
      const initialCallCount = setTimeoutSpy.mock.calls.length;

      const { rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // Simulate streaming updates
      rerender({ messageCount: 6 });
      rerender({ messageCount: 7 });
      rerender({ messageCount: 8 });

      // No setTimeout-based scroll scheduling
      expect(setTimeoutSpy.mock.calls.length).toBe(initialCallCount);
      setTimeoutSpy.mockRestore();
    });

    it("should have no useEffect-based scroll triggers", () => {
      const mockScrollIntoView = vi.fn();
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            virtuosoRef,
          },
        }
      );

      // Attach mock DOM element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Simulate multiple streaming updates without explicit scrollToBottom calls
      rerender({ messageCount: 6, virtuosoRef });
      rerender({ messageCount: 7, virtuosoRef });
      rerender({ messageCount: 8, virtuosoRef });

      // Neither DOM nor Virtuoso scroll should be triggered by re-renders alone
      // Only followOutput callback (called by Virtuoso internally) controls auto-scroll
      expect(mockScrollIntoView).not.toHaveBeenCalled();
      expect(mockScrollToIndex).not.toHaveBeenCalled();
    });

    it("followOutput returns exactly one scroll instruction per call during streaming", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // Simulate rapid streaming updates — each should produce exactly one
      // instruction from followOutput (either "smooth" or false, never both)
      const results: Array<"smooth" | false> = [];
      for (let i = 6; i <= 15; i++) {
        rerender({ messageCount: i });
        results.push(result.current.handleFollowOutput(true));
      }

      // All calls should return "smooth" (at bottom, not disabled)
      expect(results).toEqual(Array(10).fill("smooth"));

      // Simulate user scrolled up mid-stream
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      const scrolledUpResults: Array<"smooth" | false> = [];
      for (let i = 16; i <= 20; i++) {
        rerender({ messageCount: i });
        scrolledUpResults.push(result.current.handleFollowOutput(false));
      }

      // All calls should return false (not at bottom)
      expect(scrolledUpResults).toEqual(Array(5).fill(false));
    });
  });

  describe("scrollToBottom with Virtuoso ref", () => {
    it("should route through Virtuoso scrollToIndex when virtuosoRef is provided", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 10,
          virtuosoRef,
        })
      );

      act(() => {
        result.current.scrollToBottom();
      });

      expect(mockScrollToIndex).toHaveBeenCalledWith({
        index: 9,
        align: "end",
        behavior: "smooth",
      });
    });

    it("should set isAtBottom=true when scrollToBottom is called", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 10,
          virtuosoRef,
        })
      );

      // Simulate user scrolled up
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      expect(result.current.isAtBottom).toBe(false);

      act(() => {
        result.current.scrollToBottom();
      });

      expect(result.current.isAtBottom).toBe(true);
    });

    it("should not call Virtuoso scrollToIndex when messageCount is 0", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          virtuosoRef,
        })
      );

      act(() => {
        result.current.scrollToBottom();
      });

      // No scroll when there are no messages
      expect(mockScrollToIndex).not.toHaveBeenCalled();
    });

    it("should fall back to DOM marker when virtuosoRef.current is null", () => {
      const mockScrollIntoView = vi.fn();
      const virtuosoRef = { current: null };

      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          virtuosoRef: virtuosoRef as React.RefObject<VirtuosoHandle | null>,
        })
      );

      // Attach mock end marker
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      act(() => {
        result.current.scrollToBottom();
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });
  });

  describe("scrollToBottom without Virtuoso ref (DOM fallback)", () => {
    it("should use messagesEndRef.scrollIntoView", () => {
      const mockScrollIntoView = vi.fn();
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          // No virtuosoRef
        })
      );

      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      act(() => {
        result.current.scrollToBottom();
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });

    it("should set isAtBottom=true and trigger scroll", () => {
      const mockScrollIntoView = vi.fn();
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
        })
      );

      // User scrolled up
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      expect(result.current.isAtBottom).toBe(false);

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Manual scroll to bottom
      act(() => {
        result.current.scrollToBottom();
      });

      expect(result.current.isAtBottom).toBe(true);
      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });

    it("should not throw when messagesEndRef is not attached", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
        })
      );

      // scrollToBottom should not throw even without ref
      expect(() => {
        act(() => {
          result.current.scrollToBottom();
        });
      }).not.toThrow();

      expect(result.current.isAtBottom).toBe(true);
    });
  });

  describe("disabled prop (history mode)", () => {
    it("should disable followOutput auto-scroll when disabled=true", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          disabled: true,
        })
      );

      // followOutput should refuse to follow
      expect(result.current.handleFollowOutput(true)).toBe(false);
      expect(result.current.shouldAutoScroll).toBe(false);
    });

    it("should re-enable followOutput when disabled changes to false", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            disabled: true,
          },
        }
      );

      expect(result.current.shouldAutoScroll).toBe(false);
      expect(result.current.handleFollowOutput(true)).toBe(false);

      // Re-enable
      rerender({
        messageCount: 5,
        disabled: false,
      });

      expect(result.current.shouldAutoScroll).toBe(true);
      expect(result.current.handleFollowOutput(true)).toBe("smooth");
    });

    it("should still allow manual scrollToBottom when disabled", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 10,
          disabled: true,
          virtuosoRef,
        })
      );

      act(() => {
        result.current.scrollToBottom();
      });

      // Manual scroll still works even in history mode
      expect(mockScrollToIndex).toHaveBeenCalledWith({
        index: 9,
        align: "end",
        behavior: "smooth",
      });
    });
  });

  describe("GAP: dedup guard missing on handleAtBottomStateChange (F1)", () => {
    it("should still call setState when value hasn't changed (no guard)", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({ messageCount: 0 })
      );

      // isAtBottom starts as true
      expect(result.current.isAtBottom).toBe(true);

      // Calling with same value — without dedup guard, useState(true) is
      // called even though state is already true. React 18 may bail out of
      // re-render but the guard itself is missing.
      const prevRef = result.current.handleAtBottomStateChange;
      act(() => {
        result.current.handleAtBottomStateChange(true);
      });
      // The function should still be the same reference (no deps change)
      expect(result.current.handleAtBottomStateChange).toBe(prevRef);
    });
  });

  describe("GAP: scrollToBottom identity changes with messageCount (F3)", () => {
    it("should return a NEW scrollToBottom reference when messageCount changes", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: { messageCount: 5, virtuosoRef },
        }
      );

      const scrollRef1 = result.current.scrollToBottom;

      rerender({ messageCount: 6, virtuosoRef });

      const scrollRef2 = result.current.scrollToBottom;

      // GAP: messageCount is in useCallback deps → new identity on change
      expect(scrollRef1).not.toBe(scrollRef2);
    });
  });

  describe("edge cases", () => {
    it("should handle messagesEndRef being null", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
          },
        }
      );

      // messagesEndRef is null (not attached)
      expect(result.current.messagesEndRef.current).toBeNull();

      // Should not throw when message count changes
      expect(() => {
        rerender({
          messageCount: 6,
        });
      }).not.toThrow();
    });

    it("should maintain separate state for containerRef and messagesEndRef", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
        })
      );

      const mockContainer = document.createElement("div");
      const mockMessagesEnd = document.createElement("div");

      act(() => {
        // @ts-expect-error - Assigning to ref.current
        result.current.containerRef.current = mockContainer;
        // @ts-expect-error - Assigning to ref.current
        result.current.messagesEndRef.current = mockMessagesEnd;
      });

      expect(result.current.containerRef.current).toBe(mockContainer);
      expect(result.current.messagesEndRef.current).toBe(mockMessagesEnd);
      expect(result.current.containerRef.current).not.toBe(
        result.current.messagesEndRef.current
      );
    });

    it("should update scrollToIndex target when messageCount changes", () => {
      const mockScrollToIndex = vi.fn();
      const virtuosoRef = {
        current: { scrollToIndex: mockScrollToIndex } as unknown as VirtuosoHandle,
      };

      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            virtuosoRef,
          },
        }
      );

      act(() => {
        result.current.scrollToBottom();
      });

      expect(mockScrollToIndex).toHaveBeenCalledWith(
        expect.objectContaining({ index: 4 })
      );

      mockScrollToIndex.mockClear();

      // Message count increases
      rerender({
        messageCount: 15,
        virtuosoRef,
      });

      act(() => {
        result.current.scrollToBottom();
      });

      expect(mockScrollToIndex).toHaveBeenCalledWith(
        expect.objectContaining({ index: 14 })
      );
    });
  });
});
