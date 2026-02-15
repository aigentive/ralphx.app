/**
 * usePaneResize tests — Drag-to-resize coordinator column
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { usePaneResize } from "./usePaneResize";
import { useSplitPaneStore } from "@/stores/splitPaneStore";

// ============================================================================
// RAF Mock
// ============================================================================

let rafCallback: FrameRequestCallback | null = null;
let rafId = 1;

beforeEach(() => {
  rafCallback = null;
  rafId = 1;
  vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
    rafCallback = cb;
    return rafId++;
  });
  vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {
    rafCallback = null;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  // Reset store
  useSplitPaneStore.setState({
    coordinatorWidth: 40,
    isActive: false,
    focusedPane: null,
    isPrefixKeyActive: false,
    contextKey: null,
    paneOrder: [],
    panes: {},
  });
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
});

// ============================================================================
// Helpers
// ============================================================================

function createMouseEvent(type: string, clientX: number): MouseEvent {
  return new MouseEvent(type, { clientX, bubbles: true });
}

function createReactMouseEvent(clientX: number) {
  const container = document.createElement("div");
  container.setAttribute("data-split-container", "");
  Object.defineProperty(container, "clientWidth", { value: 1000 });
  const target = document.createElement("div");
  container.appendChild(target);

  return {
    preventDefault: vi.fn(),
    target,
    clientX,
  } as unknown as React.MouseEvent;
}

// ============================================================================
// Tests
// ============================================================================

describe("usePaneResize", () => {
  it("returns dividerProps and isDragging", () => {
    const { result } = renderHook(() => usePaneResize());

    expect(result.current.isDragging).toBe(false);
    expect(result.current.dividerProps.onMouseDown).toBeDefined();
    expect(result.current.dividerProps.style.cursor).toBe("col-resize");
  });

  it("starts dragging on mouseDown", () => {
    const { result } = renderHook(() => usePaneResize());

    act(() => {
      result.current.dividerProps.onMouseDown(createReactMouseEvent(400));
    });

    expect(result.current.isDragging).toBe(true);
  });

  it("updates coordinator width on drag via RAF", () => {
    const { result } = renderHook(() => usePaneResize());

    act(() => {
      result.current.dividerProps.onMouseDown(createReactMouseEvent(400));
    });

    // Simulate mouse move to 500px (50% of 1000px container)
    act(() => {
      document.dispatchEvent(createMouseEvent("mousemove", 500));
    });

    // Execute RAF callback
    act(() => {
      if (rafCallback) rafCallback(0);
    });

    expect(useSplitPaneStore.getState().coordinatorWidth).toBe(50);
  });

  it("clamps width to 25-65% range", () => {
    const { result } = renderHook(() => usePaneResize());

    act(() => {
      result.current.dividerProps.onMouseDown(createReactMouseEvent(400));
    });

    // Move to 10% (below min 25%)
    act(() => {
      document.dispatchEvent(createMouseEvent("mousemove", 100));
    });
    act(() => {
      if (rafCallback) rafCallback(0);
    });

    // Store clamps at 25% (usePaneResize clamps to 25) but then store clamps to 20 minimum
    // usePaneResize enforces 25-65, then store enforces 20-80, so 25 passes through
    expect(useSplitPaneStore.getState().coordinatorWidth).toBe(25);

    // Move to 90% (above max 65%)
    act(() => {
      document.dispatchEvent(createMouseEvent("mousemove", 900));
    });
    act(() => {
      if (rafCallback) rafCallback(0);
    });

    expect(useSplitPaneStore.getState().coordinatorWidth).toBe(65);
  });

  it("stops dragging on mouseUp and restores cursor/userSelect", () => {
    const { result } = renderHook(() => usePaneResize());

    act(() => {
      result.current.dividerProps.onMouseDown(createReactMouseEvent(400));
    });

    expect(document.body.style.cursor).toBe("col-resize");
    expect(document.body.style.userSelect).toBe("none");

    act(() => {
      document.dispatchEvent(createMouseEvent("mouseup", 400));
    });

    expect(result.current.isDragging).toBe(false);
    expect(document.body.style.cursor).toBe("");
    expect(document.body.style.userSelect).toBe("");
  });

  it("cleans up event listeners and cursor on unmount during drag", () => {
    const { result, unmount } = renderHook(() => usePaneResize());

    act(() => {
      result.current.dividerProps.onMouseDown(createReactMouseEvent(400));
    });

    expect(document.body.style.cursor).toBe("col-resize");

    unmount();

    expect(document.body.style.cursor).toBe("");
    expect(document.body.style.userSelect).toBe("");
  });
});
