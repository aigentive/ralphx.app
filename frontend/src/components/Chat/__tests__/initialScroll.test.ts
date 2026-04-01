/**
 * B1 fix: ResizeObserver-based initial scroll logic
 *
 * Tests the one-shot ResizeObserver approach used for initial conversation scroll.
 * Isolated unit tests that exercise the scroll logic directly (without rendering
 * the full ChatMessageList component, which requires Virtuoso internals).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// ============================================================================
// ResizeObserver mock infrastructure
// ============================================================================

class MockResizeObserver {
  private callback: ResizeObserverCallback;
  public observedElements: Element[] = [];
  public disconnected = false;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }

  observe(el: Element) {
    this.observedElements.push(el);
  }

  unobserve(_el: Element) {}

  disconnect() {
    this.disconnected = true;
  }

  /** Allow manual trigger for testing */
  trigger(entries: ResizeObserverEntry[] = []) {
    this.callback(entries, this as unknown as ResizeObserver);
  }
}

let mockObserverInstances: MockResizeObserver[] = [];

// ============================================================================
// Helpers — replicate the logic from the useEffect in ChatMessageList
// ============================================================================

const MARKDOWN_RENDER_DELAY_MS = 300;

/**
 * Extracted logic from the B1 initial scroll useEffect.
 * Returns a cleanup function, just like useEffect does.
 */
function runInitialScrollEffect(opts: {
  conversationId: string | null;
  timelineLength: number;
  hasScrolledRef: { current: string | null };
  scrollToIndex: (args: { index: number; align: string; behavior: string }) => void;
  scrollerEl: HTMLElement | null;
}): () => void {
  const { conversationId, timelineLength, hasScrolledRef, scrollToIndex, scrollerEl } = opts;

  if (!conversationId || timelineLength === 0 || hasScrolledRef.current === conversationId) {
    return () => {};
  }

  const targetConversationId = conversationId;

  const doScroll = () => {
    if (hasScrolledRef.current === targetConversationId) return;
    scrollToIndex({ index: timelineLength - 1, align: "end", behavior: "auto" });
    hasScrolledRef.current = targetConversationId;
  };

  const scroller = scrollerEl;
  if (!scroller) {
    // Fallback: scroller not yet mounted, use fixed delay
    const timer = setTimeout(doScroll, MARKDOWN_RENDER_DELAY_MS);
    return () => clearTimeout(timer);
  }

  let debounceTimer: ReturnType<typeof setTimeout>;
  const observer = new ResizeObserver(() => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      doScroll();
      observer.disconnect();
    }, 200);
  });

  observer.observe(scroller);

  // Safety timeout: 3s max — disconnect + force scroll if debounce never settles
  const safetyTimer = setTimeout(() => {
    observer.disconnect();
    doScroll();
  }, 3000);

  return () => {
    observer.disconnect();
    clearTimeout(debounceTimer);
    clearTimeout(safetyTimer);
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("B1 initial scroll — ResizeObserver approach", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockObserverInstances = [];

    // Use a real class constructor (not vi.fn().mockImplementation) so `new ResizeObserver(...)` works.
    // vi.stubGlobal replaces the global while MockResizeObserver IS a proper constructor.
    const OriginalMock = MockResizeObserver;
    const trackingConstructor = class TrackingResizeObserver extends OriginalMock {
      constructor(cb: ResizeObserverCallback) {
        super(cb);
        mockObserverInstances.push(this);
      }
    };
    vi.stubGlobal("ResizeObserver", trackingConstructor);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  // --------------------------------------------------------------------------
  // Guard conditions
  // --------------------------------------------------------------------------

  describe("guard conditions — early return", () => {
    it("does nothing when conversationId is null", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      const cleanup = runInitialScrollEffect({
        conversationId: null,
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.runAllTimers();
      cleanup();

      expect(scrollToIndex).not.toHaveBeenCalled();
      expect(mockObserverInstances).toHaveLength(0);
    });

    it("does nothing when timeline is empty", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 0,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.runAllTimers();
      cleanup();

      expect(scrollToIndex).not.toHaveBeenCalled();
    });

    it("does nothing when already scrolled to this conversation", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: "conv-1" };

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.runAllTimers();
      cleanup();

      expect(scrollToIndex).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // Fallback: no scroller (test env or early mount)
  // --------------------------------------------------------------------------

  describe("fallback setTimeout when scrollerEl is null", () => {
    it("uses MARKDOWN_RENDER_DELAY_MS timeout when scroller is not available", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 10,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      // Not called yet — timeout hasn't fired
      expect(scrollToIndex).not.toHaveBeenCalled();

      // Advance by MARKDOWN_RENDER_DELAY_MS (300ms)
      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);

      expect(scrollToIndex).toHaveBeenCalledWith({
        index: 9,
        align: "end",
        behavior: "auto",
      });
    });

    it("scrolls to the last timeline item index (length - 1)", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 7,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);

      expect(scrollToIndex).toHaveBeenCalledWith(
        expect.objectContaining({ index: 6 }),
      );
    });

    it("sets hasScrolledRef to conversationId after scroll", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      runInitialScrollEffect({
        conversationId: "conv-abc",
        timelineLength: 3,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);

      expect(hasScrolledRef.current).toBe("conv-abc");
    });

    it("cleanup clears the fallback timeout before it fires", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      // Clean up before timeout fires
      cleanup();
      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS + 100);

      expect(scrollToIndex).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // ResizeObserver path: scroller IS available
  // --------------------------------------------------------------------------

  describe("ResizeObserver created when scrollerEl is available", () => {
    it("creates a ResizeObserver and observes the scroller element", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      expect(mockObserverInstances).toHaveLength(1);
      expect(mockObserverInstances[0].observedElements).toContain(scroller);

      cleanup();
    });

    it("does NOT call ResizeObserver when scroller is null (fallback path)", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      expect(mockObserverInstances).toHaveLength(0);
      cleanup();
    });

    it("calls scrollToIndex after ResizeObserver fires and 200ms debounce settles", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 8,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      // Trigger the ResizeObserver
      mockObserverInstances[0].trigger();

      // Debounce hasn't settled yet
      expect(scrollToIndex).not.toHaveBeenCalled();

      // Advance 200ms debounce
      vi.advanceTimersByTime(200);

      expect(scrollToIndex).toHaveBeenCalledWith({
        index: 7,
        align: "end",
        behavior: "auto",
      });

      cleanup();
    });

    it("disconnects observer after scroll (one-shot behavior)", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];
      expect(observer.disconnected).toBe(false);

      observer.trigger();
      vi.advanceTimersByTime(200);

      expect(observer.disconnected).toBe(true);

      cleanup();
    });

    it("debounces rapid ResizeObserver firings — only scrolls once", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];

      // Fire multiple times rapidly
      observer.trigger();
      vi.advanceTimersByTime(50);
      observer.trigger();
      vi.advanceTimersByTime(50);
      observer.trigger();

      // Not yet settled
      expect(scrollToIndex).not.toHaveBeenCalled();

      // Wait for debounce to settle after last trigger
      vi.advanceTimersByTime(200);

      // scrollToIndex called exactly once
      expect(scrollToIndex).toHaveBeenCalledTimes(1);

      cleanup();
    });

    it("safety timeout forces scroll after 3s if observer never fires", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      // Don't trigger the observer — simulate stuck/no-resize case
      expect(scrollToIndex).not.toHaveBeenCalled();

      // Advance 3s safety timeout
      vi.advanceTimersByTime(3000);

      expect(scrollToIndex).toHaveBeenCalledOnce();
      expect(scrollToIndex).toHaveBeenCalledWith({
        index: 4,
        align: "end",
        behavior: "auto",
      });

      cleanup();
    });

    it("safety timeout disconnects observer before forcing scroll", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];

      vi.advanceTimersByTime(3000);

      expect(observer.disconnected).toBe(true);

      cleanup();
    });
  });

  // --------------------------------------------------------------------------
  // No double scroll guard
  // --------------------------------------------------------------------------

  describe("no double scroll — hasScrolledRef prevents duplicate scrolls", () => {
    it("does not scroll again when hasScrolledRef already matches (same conversation re-render)", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };

      // First call — scrolls and sets ref
      runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);
      expect(scrollToIndex).toHaveBeenCalledTimes(1);
      expect(hasScrolledRef.current).toBe("conv-1");

      // Second call (re-render with same conversation) — should be a no-op
      runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 6, // even more items
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);
      // Still only called once
      expect(scrollToIndex).toHaveBeenCalledTimes(1);
    });

    it("allows scroll for a new conversation after already scrolled to a previous one", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: "conv-1" }; // Already scrolled conv-1

      // New conversation — should scroll
      runInitialScrollEffect({
        conversationId: "conv-2",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: null,
      });

      vi.advanceTimersByTime(MARKDOWN_RENDER_DELAY_MS);

      expect(scrollToIndex).toHaveBeenCalledOnce();
      expect(hasScrolledRef.current).toBe("conv-2");
    });

    it("doScroll guard: if hasScrolledRef set between effect and async completion, does not double-scroll", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      // Start effect for conv-1
      runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];

      // Simulate: between effect setup and observer callback, something else set the ref
      hasScrolledRef.current = "conv-1";

      // Now observer fires
      observer.trigger();
      vi.advanceTimersByTime(200);

      // doScroll guard should prevent double-scroll
      expect(scrollToIndex).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // Cleanup
  // --------------------------------------------------------------------------

  describe("cleanup disconnects observer and clears timers", () => {
    it("cleanup disconnects observer when called before debounce settles", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];
      observer.trigger();

      // Clean up before debounce settles
      cleanup();
      vi.advanceTimersByTime(200);

      expect(observer.disconnected).toBe(true);
      // scrollToIndex should NOT be called — cleanup cancelled it
      expect(scrollToIndex).not.toHaveBeenCalled();
    });

    it("cleanup disconnects observer and clears safety timer", () => {
      const scrollToIndex = vi.fn();
      const hasScrolledRef = { current: null };
      const scroller = document.createElement("div");

      const cleanup = runInitialScrollEffect({
        conversationId: "conv-1",
        timelineLength: 5,
        hasScrolledRef,
        scrollToIndex,
        scrollerEl: scroller,
      });

      const observer = mockObserverInstances[0];

      // Clean up immediately (before any timers fire)
      cleanup();

      // Advance past both debounce and safety timeout
      vi.advanceTimersByTime(3100);

      expect(observer.disconnected).toBe(true);
      // Safety timeout was cleared — no scroll
      expect(scrollToIndex).not.toHaveBeenCalled();
    });
  });
});
