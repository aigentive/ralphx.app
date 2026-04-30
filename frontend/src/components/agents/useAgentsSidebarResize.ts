import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type RefObject,
} from "react";

const AGENTS_SIDEBAR_WIDTH_STORAGE_KEY = "ralphx-agents-sidebar-width";

export const AGENTS_SIDEBAR_MIN_WIDTH = 220;
export const AGENTS_SIDEBAR_MAX_WIDTH = 520;

export function useAgentsSidebarResize(
  sidebarRef: RefObject<HTMLDivElement | null>,
) {
  const [userSidebarWidth, setUserSidebarWidth] = useState<number | null>(() => {
    if (typeof window === "undefined") {
      return null;
    }
    const saved = window.localStorage.getItem(AGENTS_SIDEBAR_WIDTH_STORAGE_KEY);
    if (!saved) {
      return null;
    }
    const parsed = Number.parseInt(saved, 10);
    if (!Number.isFinite(parsed)) {
      return null;
    }
    return Math.max(AGENTS_SIDEBAR_MIN_WIDTH, Math.min(AGENTS_SIDEBAR_MAX_WIDTH, parsed));
  });
  const [isSidebarResizing, setIsSidebarResizing] = useState(false);
  const sidebarResizeFrameRef = useRef<number | null>(null);
  const pendingSidebarWidthRef = useRef<number | null>(null);
  const sidebarResizeBoundsRef = useRef<{ left: number } | null>(null);

  const handleSidebarResizeStart = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    const sidebar = sidebarRef.current;
    if (sidebar) {
      const rect = sidebar.getBoundingClientRect();
      sidebarResizeBoundsRef.current = { left: rect.left };
    } else {
      sidebarResizeBoundsRef.current = null;
    }
    pendingSidebarWidthRef.current = null;
    setIsSidebarResizing(true);
  }, [sidebarRef]);

  const handleSidebarResizeReset = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    if (sidebarResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(sidebarResizeFrameRef.current);
      sidebarResizeFrameRef.current = null;
    }
    pendingSidebarWidthRef.current = null;
    sidebarResizeBoundsRef.current = null;
    setUserSidebarWidth(null);
  }, []);

  const flushPendingSidebarWidth = useCallback(() => {
    if (sidebarResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(sidebarResizeFrameRef.current);
      sidebarResizeFrameRef.current = null;
    }
    const pending = pendingSidebarWidthRef.current;
    pendingSidebarWidthRef.current = null;
    if (pending !== null) {
      setUserSidebarWidth(pending);
    }
  }, []);

  const scheduleSidebarWidth = useCallback((nextWidth: number) => {
    pendingSidebarWidthRef.current = nextWidth;
    if (sidebarResizeFrameRef.current !== null) {
      return;
    }
    sidebarResizeFrameRef.current = window.requestAnimationFrame(() => {
      sidebarResizeFrameRef.current = null;
      const pending = pendingSidebarWidthRef.current;
      pendingSidebarWidthRef.current = null;
      if (pending !== null) {
        setUserSidebarWidth(pending);
      }
    });
  }, []);

  useEffect(
    () => () => {
      if (sidebarResizeFrameRef.current !== null) {
        window.cancelAnimationFrame(sidebarResizeFrameRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (!isSidebarResizing) {
      return;
    }

    const handleMouseMove = (event: MouseEvent) => {
      const sidebar = sidebarRef.current;
      if (!sidebar) {
        return;
      }
      const bounds =
        sidebarResizeBoundsRef.current ??
        (() => {
          const rect = sidebar.getBoundingClientRect();
          const next = { left: rect.left };
          sidebarResizeBoundsRef.current = next;
          return next;
        })();
      const nextWidth = event.clientX - bounds.left;
      scheduleSidebarWidth(
        Math.max(AGENTS_SIDEBAR_MIN_WIDTH, Math.min(AGENTS_SIDEBAR_MAX_WIDTH, nextWidth)),
      );
    };

    const handleMouseUp = () => {
      flushPendingSidebarWidth();
      sidebarResizeBoundsRef.current = null;
      setIsSidebarResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [flushPendingSidebarWidth, isSidebarResizing, scheduleSidebarWidth, sidebarRef]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    if (userSidebarWidth !== null) {
      window.localStorage.setItem(
        AGENTS_SIDEBAR_WIDTH_STORAGE_KEY,
        String(userSidebarWidth),
      );
      return;
    }
    window.localStorage.removeItem(AGENTS_SIDEBAR_WIDTH_STORAGE_KEY);
  }, [userSidebarWidth]);

  return {
    handleSidebarResizeReset,
    handleSidebarResizeStart,
    isSidebarResizing,
    userSidebarWidth,
  };
}
