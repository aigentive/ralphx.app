/**
 * usePaneResize — Drag-to-resize coordinator column in team split view
 *
 * Returns dividerProps (onMouseDown) and isDragging state.
 * Tracks mouse movement to update splitPaneStore.coordinatorWidth.
 * Constrains width to 25-65% range.
 * Uses requestAnimationFrame for smooth updates.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { useSplitPaneStore } from "@/stores/splitPaneStore";

const MIN_WIDTH_PERCENT = 25;
const MAX_WIDTH_PERCENT = 65;

interface UsePaneResizeReturn {
  /** Props to spread on the divider element */
  dividerProps: {
    onMouseDown: (e: React.MouseEvent) => void;
    style: { cursor: string };
  };
  /** Whether a drag is in progress */
  isDragging: boolean;
}

export function usePaneResize(): UsePaneResizeReturn {
  const setCoordinatorWidth = useSplitPaneStore((s) => s.setCoordinatorWidth);
  const [isDragging, setIsDragging] = useState(false);
  const containerWidthRef = useRef(0);
  const rafIdRef = useRef<number | null>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    // Capture the container width at drag start
    const container = (e.target as HTMLElement).closest("[data-split-container]");
    containerWidthRef.current = container?.clientWidth ?? window.innerWidth;
    setIsDragging(true);
  }, []);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
      }
      rafIdRef.current = requestAnimationFrame(() => {
        const containerWidth = containerWidthRef.current || window.innerWidth;
        const newWidth = (e.clientX / containerWidth) * 100;
        const clamped = Math.max(MIN_WIDTH_PERCENT, Math.min(MAX_WIDTH_PERCENT, newWidth));
        setCoordinatorWidth(clamped);
        rafIdRef.current = null;
      });
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
    };

    // Set cursor on document during drag
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
    };
  }, [isDragging, setCoordinatorWidth]);

  return {
    dividerProps: {
      onMouseDown: handleMouseDown,
      style: { cursor: "col-resize" },
    },
    isDragging,
  };
}
