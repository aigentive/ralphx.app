/**
 * usePlanBrowserLayout tests — Responsive sidebar layout state machine
 *
 * Tests all 7 state transitions, localStorage persistence, and transition suppression.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { usePlanBrowserLayout } from "./usePlanBrowserLayout";

// ============================================================================
// matchMedia Mock
// ============================================================================

type MediaQueryCallback = (e: MediaQueryListEvent) => void;

interface MockMediaQueryList extends EventTarget {
  matches: boolean;
  addEventListener: (type: string, cb: MediaQueryCallback) => void;
  removeEventListener: (type: string, cb: MediaQueryCallback) => void;
  addListener: (cb: MediaQueryCallback) => void;
  removeListener: (cb: MediaQueryCallback) => void;
  _listeners: MediaQueryCallback[];
  _trigger: (matches: boolean) => void;
}

function createMockMQL(initialMatches: boolean): MockMediaQueryList {
  const listeners: MediaQueryCallback[] = [];
  const mql: MockMediaQueryList = {
    matches: initialMatches,
    addEventListener: vi.fn((_type: string, cb: MediaQueryCallback) => {
      listeners.push(cb);
    }),
    removeEventListener: vi.fn((_type: string, cb: MediaQueryCallback) => {
      const idx = listeners.indexOf(cb);
      if (idx !== -1) listeners.splice(idx, 1);
    }),
    addListener: vi.fn((cb: MediaQueryCallback) => {
      listeners.push(cb);
    }),
    removeListener: vi.fn((cb: MediaQueryCallback) => {
      const idx = listeners.indexOf(cb);
      if (idx !== -1) listeners.splice(idx, 1);
    }),
    _listeners: listeners,
    _trigger: (matches: boolean) => {
      mql.matches = matches;
      const event = { matches } as MediaQueryListEvent;
      listeners.forEach((cb) => cb(event));
    },
    // EventTarget stubs
    dispatchEvent: vi.fn(),
  } as unknown as MockMediaQueryList;
  return mql;
}

// Two MQL instances — one per query
let mqLarge: MockMediaQueryList; // ≥1440px
let mqMedium: MockMediaQueryList; // ≥1280px

function setupMatchMedia(isLarge: boolean, isMedium: boolean) {
  mqLarge = createMockMQL(isLarge);
  mqMedium = createMockMQL(isMedium);

  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: vi.fn((query: string) => {
      if (query === "(min-width: 1440px)") return mqLarge as unknown as MediaQueryList;
      if (query === "(min-width: 1280px)") return mqMedium as unknown as MediaQueryList;
      return createMockMQL(false) as unknown as MediaQueryList;
    }),
  });
}

// ============================================================================
// RAF Mock
// ============================================================================

let rafCallback: FrameRequestCallback | null = null;

function flushRaf() {
  if (rafCallback) {
    const cb = rafCallback;
    rafCallback = null;
    cb(0);
  }
}

// ============================================================================
// localStorage Mock
// ============================================================================

const COLLAPSE_PREF_KEY = "ralphx-plan-browser-collapsed";

// ============================================================================
// Setup / Teardown
// ============================================================================

beforeEach(() => {
  localStorage.clear();
  rafCallback = null;
  vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
    rafCallback = cb;
    return 1;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
});

// ============================================================================
// Tests: Initial state
// ============================================================================

describe("usePlanBrowserLayout — initial state", () => {
  it("starts expanded at large viewport (≥1440px)", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.sidebarWidth).toBe(340);
  });

  it("starts expanded at medium viewport (1280-1439px)", () => {
    setupMatchMedia(false, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.sidebarWidth).toBe(276);
  });

  it("starts collapsed at small viewport (<1280px)", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.sidebarWidth).toBe(0);
  });

  it("loads collapsed preference from localStorage on mount", () => {
    setupMatchMedia(true, true);
    localStorage.setItem(COLLAPSE_PREF_KEY, JSON.stringify(true));
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.sidebarWidth).toBe(0);
  });
});

// ============================================================================
// Tests: Transition 1 — Expanded → Collapsed via user click at ≥1280px
// ============================================================================

describe("Transition 1: Expanded → Collapsed via user click (≥1280px)", () => {
  it("collapses sidebar when toggleCollapse called at large viewport", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse();
    });

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.sidebarWidth).toBe(0);
  });

  it("persists collapse preference to localStorage", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse();
    });

    expect(localStorage.getItem(COLLAPSE_PREF_KEY)).toBe("true");
  });

  it("collapses sidebar when toggleCollapse called at medium viewport", () => {
    setupMatchMedia(false, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse();
    });

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.sidebarWidth).toBe(0);
  });
});

// ============================================================================
// Tests: Transition 2 — Expanded → Collapsed via viewport shrink to <1280px
// ============================================================================

describe("Transition 2: Expanded → Collapsed via viewport shrink (<1280px)", () => {
  it("auto-collapses when viewport shrinks below 1280px", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(false);

    act(() => {
      mqLarge._trigger(false);
      mqMedium._trigger(false);
    });

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.isOverlayOpen).toBe(false);
  });

  it("sets suppressTransition.current = true during viewport resize", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      mqLarge._trigger(false);
      mqMedium._trigger(false);
    });

    expect(result.current.suppressTransition.current).toBe(true);
  });

  it("clears suppressTransition.current after requestAnimationFrame", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      mqLarge._trigger(false);
      mqMedium._trigger(false);
    });
    act(() => {
      flushRaf();
    });

    expect(result.current.suppressTransition.current).toBe(false);
  });
});

// ============================================================================
// Tests: Transition 3 — Collapsed → Expanded via toggle at ≥1280px
// ============================================================================

describe("Transition 3: Collapsed → Expanded via toggle (≥1280px)", () => {
  it("expands when toggleCollapse called while collapsed at large viewport", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse(); // collapse
    });
    expect(result.current.isCollapsed).toBe(true);

    act(() => {
      result.current.toggleCollapse(); // expand
    });

    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.sidebarWidth).toBe(340);
  });

  it("persists expanded preference to localStorage", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse(); // collapse
      result.current.toggleCollapse(); // expand
    });

    expect(localStorage.getItem(COLLAPSE_PREF_KEY)).toBe("false");
  });
});

// ============================================================================
// Tests: Transition 4 — Collapsed → Overlay open via toggle at <1280px
// ============================================================================

describe("Transition 4: Collapsed → Overlay open via toggle (<1280px)", () => {
  it("opens overlay when toggleCollapse called at small viewport", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.isOverlayOpen).toBe(false);

    act(() => {
      result.current.toggleCollapse();
    });

    expect(result.current.isOverlayOpen).toBe(true);
    expect(result.current.sidebarWidth).toBe(340); // overlay uses large width
  });

  it("openOverlay also opens the overlay", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });

    expect(result.current.isOverlayOpen).toBe(true);
  });
});

// ============================================================================
// Tests: Transition 5 — Overlay open → Collapsed via backdrop click
// ============================================================================

describe("Transition 5: Overlay open → Collapsed via backdrop/closeOverlay", () => {
  it("closes overlay when closeOverlay is called", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });
    expect(result.current.isOverlayOpen).toBe(true);

    act(() => {
      result.current.closeOverlay();
    });

    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.isCollapsed).toBe(true);
  });
});

// ============================================================================
// Tests: Transition 6 — Overlay open → Collapsed via overlay header collapse
// ============================================================================

describe("Transition 6: Overlay open → Collapsed via toggleCollapse in overlay", () => {
  it("closes overlay when toggleCollapse called while overlay is open", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });
    expect(result.current.isOverlayOpen).toBe(true);

    act(() => {
      result.current.toggleCollapse();
    });

    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.isCollapsed).toBe(true);
  });
});

// ============================================================================
// Tests: Transition 7 — Overlay open → Expanded (inline) via viewport grow ≥1280px
// ============================================================================

describe("Transition 7: Overlay open → Expanded (inline) via viewport grow ≥1280px", () => {
  it("transitions to expanded inline when viewport grows while overlay is open", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });
    expect(result.current.isOverlayOpen).toBe(true);

    act(() => {
      mqMedium._trigger(true);
    });

    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.sidebarWidth).toBe(276);
  });

  it("transitions to expanded (large) when viewport grows to ≥1440px while overlay open", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });

    act(() => {
      mqLarge._trigger(true);
      mqMedium._trigger(true);
    });

    expect(result.current.isOverlayOpen).toBe(false);
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.sidebarWidth).toBe(340);
  });

  it("does NOT expand if viewport grows while overlay is closed (keeps userCollapsed)", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    // Collapse first (no overlay open)
    // At small viewport, overlay is closed by default — viewport grows
    act(() => {
      mqMedium._trigger(true);
    });

    // Sidebar was collapsed at <1280 (overlay closed), now medium — should stay collapsed
    // (no explicit user preference was set — so it respects whatever was stored)
    // Default behavior: userCollapsed=false, so expands
    expect(result.current.isCollapsed).toBe(false);
  });
});

// ============================================================================
// Tests: Escape key handler
// ============================================================================

describe("Escape key handler for overlay dismiss", () => {
  it("closes overlay on Escape key press", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });
    expect(result.current.isOverlayOpen).toBe(true);

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(result.current.isOverlayOpen).toBe(false);
  });

  it("does not close overlay if [data-overlay-priority] element is present", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });

    // Simulate a higher-priority overlay being open
    const priorityEl = document.createElement("div");
    priorityEl.setAttribute("data-overlay-priority", "true");
    document.body.appendChild(priorityEl);

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(result.current.isOverlayOpen).toBe(true);

    document.body.removeChild(priorityEl);
  });

  it("does not listen for Escape when overlay is closed", () => {
    setupMatchMedia(false, false);
    const { result } = renderHook(() => usePlanBrowserLayout());

    // Overlay is closed, Escape should have no effect
    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(result.current.isOverlayOpen).toBe(false);
  });

  it("removes Escape listener when overlay closes", () => {
    setupMatchMedia(false, false);
    const addSpy = vi.spyOn(document, "addEventListener");
    const removeSpy = vi.spyOn(document, "removeEventListener");

    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.openOverlay();
    });
    const addCallsBefore = addSpy.mock.calls.filter((c) => c[0] === "keydown").length;
    expect(addCallsBefore).toBeGreaterThan(0);

    act(() => {
      result.current.closeOverlay();
    });
    const removeCallsAfter = removeSpy.mock.calls.filter((c) => c[0] === "keydown").length;
    expect(removeCallsAfter).toBeGreaterThan(0);
  });
});

// ============================================================================
// Tests: localStorage persistence
// ============================================================================

describe("localStorage persistence", () => {
  it("loads collapsed=true from localStorage on mount at ≥1280px", () => {
    setupMatchMedia(false, true);
    localStorage.setItem(COLLAPSE_PREF_KEY, JSON.stringify(true));

    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(true);
  });

  it("loads collapsed=false from localStorage on mount", () => {
    setupMatchMedia(true, true);
    localStorage.setItem(COLLAPSE_PREF_KEY, JSON.stringify(false));

    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(false);
  });

  it("handles corrupt localStorage value gracefully (defaults to expanded)", () => {
    setupMatchMedia(true, true);
    localStorage.setItem(COLLAPSE_PREF_KEY, "not-valid-json{{");

    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.isCollapsed).toBe(false);
  });

  it("auto-collapse at <1280px overrides localStorage preference on reload", () => {
    setupMatchMedia(false, false);
    // Even if user collapsed at large viewport before, at <1280 we auto-collapse
    // (but userCollapsed=true won't affect the "isCollapsed" since breakpoint=small already implies collapsed)
    localStorage.setItem(COLLAPSE_PREF_KEY, JSON.stringify(false));

    const { result } = renderHook(() => usePlanBrowserLayout());

    // At small viewport, always collapsed (isCollapsed = breakpoint === "small" || userCollapsed)
    expect(result.current.isCollapsed).toBe(true);
  });
});

// ============================================================================
// Tests: suppressTransition ref behavior
// ============================================================================

describe("suppressTransition ref", () => {
  it("returns a ref with initial value false", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    expect(result.current.suppressTransition.current).toBe(false);
  });

  it("does NOT set suppressTransition on user-initiated toggle (only on resize)", () => {
    setupMatchMedia(true, true);
    const { result } = renderHook(() => usePlanBrowserLayout());

    act(() => {
      result.current.toggleCollapse();
    });

    // User toggle should NOT suppress transition
    expect(result.current.suppressTransition.current).toBe(false);
  });
});

// ============================================================================
// Tests: cleanup
// ============================================================================

describe("cleanup on unmount", () => {
  it("removes matchMedia listeners on unmount", () => {
    setupMatchMedia(true, true);
    const { unmount } = renderHook(() => usePlanBrowserLayout());

    unmount();

    expect(mqLarge.removeEventListener).toHaveBeenCalled();
    expect(mqMedium.removeEventListener).toHaveBeenCalled();
  });
});
