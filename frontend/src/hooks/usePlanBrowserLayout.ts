import { useCallback, useEffect, useReducer, useRef } from "react";

// ============================================================================
// Breakpoint queries and width constants
// ============================================================================

const QUERY_LARGE = "(min-width: 1440px)";
const QUERY_MEDIUM = "(min-width: 1280px)";

const WIDTH_LARGE = 340;
const WIDTH_MEDIUM = 276;

const COLLAPSE_PREF_KEY = "ralphx-plan-browser-collapsed";

// ============================================================================
// Types
// ============================================================================

type Breakpoint = "large" | "medium" | "small";

interface LayoutState {
  breakpoint: Breakpoint;
  /** Manual collapse preference — only meaningful at ≥1280px */
  userCollapsed: boolean;
  /** Overlay panel open — only meaningful at <1280px */
  overlayOpen: boolean;
}

type LayoutAction =
  | { type: "BREAKPOINT_CHANGE"; breakpoint: Breakpoint; wasOverlayOpen: boolean }
  | { type: "TOGGLE_COLLAPSE" }
  | { type: "OPEN_OVERLAY" }
  | { type: "CLOSE_OVERLAY" };

export interface PlanBrowserLayoutResult {
  /** Sidebar pixel width (0 when collapsed inline) */
  sidebarWidth: number;
  /** True when sidebar is not visible inline */
  isCollapsed: boolean;
  /** True when sidebar is open as a floating overlay */
  isOverlayOpen: boolean;
  /** Toggle collapse/expand or toggle overlay at small viewport */
  toggleCollapse: () => void;
  /** Open overlay (used for external trigger, e.g. strip click) */
  openOverlay: () => void;
  /** Close overlay (used for backdrop click or overlay header) */
  closeOverlay: () => void;
  /** Ref set to true during viewport resize events to suppress CSS transition */
  suppressTransition: React.MutableRefObject<boolean>;
}

// ============================================================================
// localStorage helpers
// ============================================================================

function loadCollapsePreference(): boolean {
  try {
    const saved = localStorage.getItem(COLLAPSE_PREF_KEY);
    if (saved !== null) {
      return JSON.parse(saved) as boolean;
    }
  } catch {
    /* ignore parse errors */
  }
  return false;
}

function saveCollapsePreference(collapsed: boolean): void {
  try {
    localStorage.setItem(COLLAPSE_PREF_KEY, JSON.stringify(collapsed));
  } catch {
    /* ignore write errors */
  }
}

// ============================================================================
// Breakpoint helpers
// ============================================================================

function getBreakpoint(isLarge: boolean, isMedium: boolean): Breakpoint {
  if (isLarge) return "large";
  if (isMedium) return "medium";
  return "small";
}

// ============================================================================
// State machine reducer
// ============================================================================

function layoutReducer(state: LayoutState, action: LayoutAction): LayoutState {
  switch (action.type) {
    case "BREAKPOINT_CHANGE": {
      const { breakpoint: newBp, wasOverlayOpen } = action;

      if (newBp === "small") {
        // Viewport shrunk to <1280px: auto-collapse (overlay closes)
        return { ...state, breakpoint: newBp, overlayOpen: false };
      }

      if (state.breakpoint === "small") {
        // Viewport grew from <1280px to ≥1280px
        // If overlay was open, user wanted sidebar visible → expand inline (clear userCollapsed)
        const userCollapsed = wasOverlayOpen ? false : state.userCollapsed;
        return { breakpoint: newBp, userCollapsed, overlayOpen: false };
      }

      // Same tier or large↔medium resize
      return { ...state, breakpoint: newBp };
    }

    case "TOGGLE_COLLAPSE": {
      if (state.breakpoint === "small") {
        // At small viewport: toggle overlay open/closed
        return { ...state, overlayOpen: !state.overlayOpen };
      }
      // At ≥1280px: toggle user collapse preference
      const newCollapsed = !state.userCollapsed;
      saveCollapsePreference(newCollapsed);
      return { ...state, userCollapsed: newCollapsed };
    }

    case "OPEN_OVERLAY":
      return { ...state, overlayOpen: true };

    case "CLOSE_OVERLAY":
      return { ...state, overlayOpen: false };

    default:
      return state;
  }
}

// ============================================================================
// Hook
// ============================================================================

export function usePlanBrowserLayout(): PlanBrowserLayoutResult {
  const suppressTransition = useRef<boolean>(false);

  const [state, dispatch] = useReducer(
    layoutReducer,
    undefined,
    (): LayoutState => {
      if (typeof window === "undefined") {
        return { breakpoint: "large", userCollapsed: false, overlayOpen: false };
      }
      const isLarge = window.matchMedia(QUERY_LARGE).matches;
      const isMedium = window.matchMedia(QUERY_MEDIUM).matches;
      return {
        breakpoint: getBreakpoint(isLarge, isMedium),
        userCollapsed: loadCollapsePreference(),
        overlayOpen: false,
      };
    },
  );

  // ── matchMedia listeners ──────────────────────────────────────────────────

  // Ref to read current overlayOpen in matchMedia handler without stale closure
  const overlayOpenRef = useRef(state.overlayOpen);

  // Sync ref after each render (not during render — avoids react-hooks/refs lint error)
  useEffect(() => {
    overlayOpenRef.current = state.overlayOpen;
  });

  useEffect(() => {
    if (typeof window === "undefined") return;

    const mqLarge = window.matchMedia(QUERY_LARGE);
    const mqMedium = window.matchMedia(QUERY_MEDIUM);

    const handleChange = () => {
      const isLarge = mqLarge.matches;
      const isMedium = mqMedium.matches;
      const newBreakpoint = getBreakpoint(isLarge, isMedium);

      // Suppress CSS transition during resize (only user toggles should animate)
      suppressTransition.current = true;
      requestAnimationFrame(() => {
        suppressTransition.current = false;
      });

      dispatch({
        type: "BREAKPOINT_CHANGE",
        breakpoint: newBreakpoint,
        wasOverlayOpen: overlayOpenRef.current,
      });
    };

    if (mqLarge.addEventListener) {
      mqLarge.addEventListener("change", handleChange);
      mqMedium.addEventListener("change", handleChange);
    } else {
      // Legacy Safari fallback
      mqLarge.addListener(handleChange);
      mqMedium.addListener(handleChange);
    }

    return () => {
      if (mqLarge.removeEventListener) {
        mqLarge.removeEventListener("change", handleChange);
        mqMedium.removeEventListener("change", handleChange);
      } else {
        mqLarge.removeListener(handleChange);
        mqMedium.removeListener(handleChange);
      }
    };
  }, []);

  // ── Escape key handler (overlay dismiss) ──────────────────────────────────

  useEffect(() => {
    if (!state.overlayOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // Priority gating: yield to higher-z overlays (ProposalDetailSheet, AcceptModal, etc.)
      if (document.querySelector("[data-overlay-priority]")) return;
      dispatch({ type: "CLOSE_OVERLAY" });
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [state.overlayOpen]);

  // ── Derived values ─────────────────────────────────────────────────────────

  const isCollapsed = state.breakpoint === "small" || state.userCollapsed;
  const isOverlayOpen = state.breakpoint === "small" && state.overlayOpen;

  let sidebarWidth: number;
  if (isOverlayOpen) {
    sidebarWidth = WIDTH_LARGE; // overlay always uses full sidebar width
  } else if (isCollapsed) {
    sidebarWidth = 0;
  } else {
    sidebarWidth = state.breakpoint === "large" ? WIDTH_LARGE : WIDTH_MEDIUM;
  }

  // ── Callbacks ─────────────────────────────────────────────────────────────

  const toggleCollapse = useCallback(() => {
    dispatch({ type: "TOGGLE_COLLAPSE" });
  }, []);

  const openOverlay = useCallback(() => {
    dispatch({ type: "OPEN_OVERLAY" });
  }, []);

  const closeOverlay = useCallback(() => {
    dispatch({ type: "CLOSE_OVERLAY" });
  }, []);

  return {
    sidebarWidth,
    isCollapsed,
    isOverlayOpen,
    toggleCollapse,
    openOverlay,
    closeOverlay,
    suppressTransition,
  };
}
