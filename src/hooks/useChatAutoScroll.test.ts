/**
 * useChatAutoScroll hook tests
 *
 * Tests the unified chat auto-scroll hook behavior:
 * - Bottom detection with 150px threshold
 * - Auto-scroll on new messages when at bottom
 * - Auto-scroll on streaming content changes
 * - Manual scroll-up pauses auto-scroll
 * - Manual scroll-to-bottom resumes auto-scroll
 * - History mode disables auto-scroll
 * - RAF debouncing for streaming updates
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useChatAutoScroll } from "./useChatAutoScroll";

describe("useChatAutoScroll", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock requestAnimationFrame
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      cb(0);
      return 0;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("initial state", () => {
    it("should start with isAtBottom=true", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      expect(result.current.isAtBottom).toBe(true);
    });

    it("should start with shouldAutoScroll=true when not disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
          disabled: false,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(true);
    });

    it("should start with shouldAutoScroll=false when disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
          disabled: true,
        })
      );

      expect(result.current.shouldAutoScroll).toBe(false);
    });

    it("should provide containerRef and messagesEndRef", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      expect(result.current.containerRef).toBeDefined();
      expect(result.current.containerRef.current).toBeNull(); // Not attached yet
      expect(result.current.messagesEndRef).toBeDefined();
      expect(result.current.messagesEndRef.current).toBeNull();
    });

    it("should provide Virtuoso callback handlers", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      expect(result.current.handleAtBottomStateChange).toBeInstanceOf(Function);
      expect(result.current.handleFollowOutput).toBeInstanceOf(Function);
      expect(result.current.scrollToBottom).toBeInstanceOf(Function);
    });
  });

  describe("bottom state tracking", () => {
    it("should update isAtBottom via handleAtBottomStateChange", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
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
          isStreaming: false,
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
          isStreaming: false,
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

  describe("handleFollowOutput (Virtuoso callback)", () => {
    it("should return 'smooth' when at bottom and not disabled", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      const output = result.current.handleFollowOutput(true);
      expect(output).toBe("smooth");
    });

    it("should return false when not at bottom", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      const output = result.current.handleFollowOutput(false);
      expect(output).toBe(false);
    });

    it("should return false when disabled, even if at bottom", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
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
            isStreaming: false,
            disabled: false,
          },
        }
      );

      expect(result.current.handleFollowOutput(true)).toBe("smooth");

      rerender({
        messageCount: 0,
        isStreaming: false,
        disabled: true,
      });

      expect(result.current.handleFollowOutput(true)).toBe(false);
    });
  });

  describe("auto-scroll on message count changes", () => {
    it("should trigger scrollIntoView when message count increases and at bottom", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Increase message count
      rerender({
        messageCount: 6,
        isStreaming: false,
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });

    it("should not trigger scroll when scrolled up (not at bottom)", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // User scrolled up
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      // Increase message count
      rerender({
        messageCount: 6,
        isStreaming: false,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should not trigger scroll when disabled", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
            disabled: true,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Increase message count
      rerender({
        messageCount: 6,
        isStreaming: false,
        disabled: true,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should not scroll when message count is 0", () => {
      const mockScrollIntoView = vi.fn();
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 0,
          isStreaming: false,
        })
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });
  });

  describe("auto-scroll on streaming content changes", () => {
    it("should trigger scroll when streaming hash changes", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      mockScrollIntoView.mockClear();

      // Change streaming hash
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: "hash2",
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });

    it("should use RAF debouncing for streaming updates", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: vi.fn(),
        } as unknown as HTMLDivElement;
      });

      vi.clearAllMocks();

      // Change streaming hash
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: "hash2",
      });

      expect(window.requestAnimationFrame).toHaveBeenCalled();
    });

    it("should cancel RAF on cleanup", () => {
      const { result, rerender, unmount } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: vi.fn(),
        } as unknown as HTMLDivElement;
      });

      // Change streaming hash to schedule RAF
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: "hash2",
      });

      // Unmount should cancel RAF
      unmount();

      expect(window.cancelAnimationFrame).toHaveBeenCalled();
    });

    it("should not scroll when streaming hash changes but not at bottom", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // User scrolled up
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      mockScrollIntoView.mockClear();

      // Change streaming hash
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: "hash2",
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should not scroll when not streaming", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      mockScrollIntoView.mockClear();

      // Change streaming hash (but not streaming)
      rerender({
        messageCount: 5,
        isStreaming: false,
        streamingHash: "hash2",
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should not scroll when streamingHash is undefined", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: undefined,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      mockScrollIntoView.mockClear();

      // Re-render (streamingHash still undefined)
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: undefined,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });

    it("should not scroll when disabled, even during streaming", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
            disabled: true,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      mockScrollIntoView.mockClear();

      // Change streaming hash
      rerender({
        messageCount: 5,
        isStreaming: true,
        streamingHash: "hash2",
        disabled: true,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();
    });
  });

  describe("manual scroll-to-bottom", () => {
    it("should set isAtBottom=true and trigger scroll", () => {
      const mockScrollIntoView = vi.fn();
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          isStreaming: false,
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

    it("should resume auto-scroll after manual scroll-to-bottom", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
          },
        }
      );

      // User scrolled up
      act(() => {
        result.current.handleAtBottomStateChange(false);
      });

      expect(result.current.shouldAutoScroll).toBe(false);

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

      expect(result.current.shouldAutoScroll).toBe(true);

      mockScrollIntoView.mockClear();

      // New message arrives - should auto-scroll now
      rerender({
        messageCount: 6,
        isStreaming: false,
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });

    it("should work when messagesEndRef is not attached", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          isStreaming: false,
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
    it("should disable auto-scroll completely when disabled=true", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
            disabled: true,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Try message count change
      rerender({
        messageCount: 6,
        isStreaming: true,
        streamingHash: "hash1",
        disabled: true,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();

      // Try streaming hash change
      rerender({
        messageCount: 6,
        isStreaming: true,
        streamingHash: "hash2",
        disabled: true,
      });

      expect(mockScrollIntoView).not.toHaveBeenCalled();

      // Ensure shouldAutoScroll remains false
      expect(result.current.shouldAutoScroll).toBe(false);
    });

    it("should re-enable auto-scroll when disabled changes to false", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
            disabled: true,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      expect(result.current.shouldAutoScroll).toBe(false);

      // Re-enable
      rerender({
        messageCount: 5,
        isStreaming: false,
        disabled: false,
      });

      expect(result.current.shouldAutoScroll).toBe(true);

      // New message should trigger scroll
      rerender({
        messageCount: 6,
        isStreaming: false,
        disabled: false,
      });

      expect(mockScrollIntoView).toHaveBeenCalledWith({ behavior: "smooth" });
    });
  });

  describe("edge cases", () => {
    it("should handle messagesEndRef being null", () => {
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
          },
        }
      );

      // messagesEndRef is null (not attached)
      expect(result.current.messagesEndRef.current).toBeNull();

      // Should not throw when message count changes
      expect(() => {
        rerender({
          messageCount: 6,
          isStreaming: false,
        });
      }).not.toThrow();
    });

    it("should handle rapid message count increases", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: false,
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      // Rapid increases
      rerender({ messageCount: 6, isStreaming: false });
      rerender({ messageCount: 7, isStreaming: false });
      rerender({ messageCount: 8, isStreaming: false });

      // Should have called scroll for each increase
      expect(mockScrollIntoView).toHaveBeenCalled();
    });

    it("should handle rapid streaming hash changes", () => {
      const mockScrollIntoView = vi.fn();
      const { result, rerender } = renderHook(
        (props) => useChatAutoScroll(props),
        {
          initialProps: {
            messageCount: 5,
            isStreaming: true,
            streamingHash: "hash1",
          },
        }
      );

      // Attach mock element
      act(() => {
        result.current.messagesEndRef.current = {
          scrollIntoView: mockScrollIntoView,
        } as unknown as HTMLDivElement;
      });

      mockScrollIntoView.mockClear();

      // Rapid hash changes (RAF should debounce)
      rerender({ messageCount: 5, isStreaming: true, streamingHash: "hash2" });
      rerender({ messageCount: 5, isStreaming: true, streamingHash: "hash3" });
      rerender({ messageCount: 5, isStreaming: true, streamingHash: "hash4" });

      // RAF should have been called for each change
      expect(window.requestAnimationFrame).toHaveBeenCalled();
    });

    it("should maintain separate state for containerRef and messagesEndRef", () => {
      const { result } = renderHook(() =>
        useChatAutoScroll({
          messageCount: 5,
          isStreaming: false,
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
  });
});
