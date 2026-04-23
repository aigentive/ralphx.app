import { useCallback, useEffect, useReducer, useRef } from "react";

const DEFAULT_QUERY_LARGE = "(min-width: 1440px)";
const DEFAULT_QUERY_MEDIUM = "(min-width: 1280px)";
const DEFAULT_WIDTH_LARGE = 340;
const DEFAULT_WIDTH_MEDIUM = 276;

type Breakpoint = "large" | "medium" | "small";

interface LayoutState {
  breakpoint: Breakpoint;
  userCollapsed: boolean;
  overlayOpen: boolean;
}

type LayoutAction =
  | { type: "BREAKPOINT_CHANGE"; breakpoint: Breakpoint; wasOverlayOpen: boolean }
  | { type: "TOGGLE_COLLAPSE" }
  | { type: "OPEN_OVERLAY" }
  | { type: "CLOSE_OVERLAY" };

export interface ResponsiveSidebarLayoutOptions {
  storageKey: string;
  largeQuery?: string;
  mediumQuery?: string;
  largeWidth?: number;
  mediumWidth?: number;
}

export interface ResponsiveSidebarLayoutResult {
  sidebarWidth: number;
  isCollapsed: boolean;
  isOverlayOpen: boolean;
  toggleCollapse: () => void;
  openOverlay: () => void;
  closeOverlay: () => void;
  suppressTransition: React.MutableRefObject<boolean>;
}

function loadCollapsePreference(storageKey: string): boolean {
  try {
    const saved = localStorage.getItem(storageKey);
    if (saved !== null) {
      return JSON.parse(saved) as boolean;
    }
  } catch {
    // Ignore malformed or unavailable storage.
  }
  return false;
}

function saveCollapsePreference(storageKey: string, collapsed: boolean): void {
  try {
    localStorage.setItem(storageKey, JSON.stringify(collapsed));
  } catch {
    // Ignore unavailable storage.
  }
}

function getBreakpoint(isLarge: boolean, isMedium: boolean): Breakpoint {
  if (isLarge) return "large";
  if (isMedium) return "medium";
  return "small";
}

function layoutReducer(
  state: LayoutState,
  action: LayoutAction,
  storageKey: string
): LayoutState {
  switch (action.type) {
    case "BREAKPOINT_CHANGE": {
      const { breakpoint: newBreakpoint, wasOverlayOpen } = action;

      if (newBreakpoint === "small") {
        return { ...state, breakpoint: newBreakpoint, overlayOpen: false };
      }

      if (state.breakpoint === "small") {
        const userCollapsed = wasOverlayOpen ? false : state.userCollapsed;
        return { breakpoint: newBreakpoint, userCollapsed, overlayOpen: false };
      }

      return { ...state, breakpoint: newBreakpoint };
    }

    case "TOGGLE_COLLAPSE": {
      if (state.breakpoint === "small") {
        return { ...state, overlayOpen: !state.overlayOpen };
      }

      const nextCollapsed = !state.userCollapsed;
      saveCollapsePreference(storageKey, nextCollapsed);
      return { ...state, userCollapsed: nextCollapsed };
    }

    case "OPEN_OVERLAY":
      return { ...state, overlayOpen: true };

    case "CLOSE_OVERLAY":
      return { ...state, overlayOpen: false };

    default:
      return state;
  }
}

export function useResponsiveSidebarLayout({
  storageKey,
  largeQuery = DEFAULT_QUERY_LARGE,
  mediumQuery = DEFAULT_QUERY_MEDIUM,
  largeWidth = DEFAULT_WIDTH_LARGE,
  mediumWidth = DEFAULT_WIDTH_MEDIUM,
}: ResponsiveSidebarLayoutOptions): ResponsiveSidebarLayoutResult {
  const suppressTransition = useRef<boolean>(false);

  const [state, dispatch] = useReducer(
    (currentState: LayoutState, action: LayoutAction) =>
      layoutReducer(currentState, action, storageKey),
    undefined,
    (): LayoutState => {
      if (typeof window === "undefined") {
        return { breakpoint: "large", userCollapsed: false, overlayOpen: false };
      }

      const isLarge = window.matchMedia(largeQuery).matches;
      const isMedium = window.matchMedia(mediumQuery).matches;
      return {
        breakpoint: getBreakpoint(isLarge, isMedium),
        userCollapsed: loadCollapsePreference(storageKey),
        overlayOpen: false,
      };
    }
  );

  const overlayOpenRef = useRef(state.overlayOpen);

  useEffect(() => {
    overlayOpenRef.current = state.overlayOpen;
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const mqLarge = window.matchMedia(largeQuery);
    const mqMedium = window.matchMedia(mediumQuery);

    const handleChange = () => {
      const newBreakpoint = getBreakpoint(mqLarge.matches, mqMedium.matches);

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
  }, [largeQuery, mediumQuery]);

  useEffect(() => {
    if (!state.overlayOpen) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      if (document.querySelector("[data-overlay-priority]")) {
        return;
      }
      dispatch({ type: "CLOSE_OVERLAY" });
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [state.overlayOpen]);

  const isCollapsed = state.breakpoint === "small" || state.userCollapsed;
  const isOverlayOpen = state.breakpoint === "small" && state.overlayOpen;

  let sidebarWidth = 0;
  if (isOverlayOpen) {
    sidebarWidth = largeWidth;
  } else if (!isCollapsed) {
    sidebarWidth = state.breakpoint === "large" ? largeWidth : mediumWidth;
  }

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
