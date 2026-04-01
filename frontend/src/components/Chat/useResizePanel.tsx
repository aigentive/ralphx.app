/* eslint-disable react-refresh/only-export-components */
/**
 * useResizePanel - Hook for panel resize functionality
 */

import { useRef, useState, useCallback, useEffect } from "react";
import { MIN_WIDTH, MAX_WIDTH_PERCENT } from "./ResizeablePanel.constants";

export interface UseResizePanelOptions {
  initialWidth: number;
  onWidthChange: (width: number) => void;
}

interface ResizeHandleProps {
  isDragging: boolean;
  onMouseDown: (e: React.MouseEvent) => void;
}

function ResizeHandle({ isDragging, onMouseDown }: ResizeHandleProps) {
  return (
    <div
      data-testid="chat-panel-resize-handle"
      className="absolute top-0 bottom-0 w-1.5 cursor-ew-resize z-[41]"
      style={{ left: "-3px" }}
      onMouseDown={onMouseDown}
    >
      <div
        className={`absolute top-1/2 left-1/2 w-0.5 h-12 -translate-x-1/2 -translate-y-1/2 rounded-sm transition-all duration-150 ${
          isDragging ? "h-16" : ""
        }`}
        style={{
          backgroundColor: isDragging
            ? "var(--accent-primary)"
            : "transparent",
          boxShadow: isDragging
            ? "0 0 8px rgba(255,107,53,0.4)"
            : "none",
        }}
      />
      <style>{`
        [data-testid="chat-panel-resize-handle"]:hover > div {
          background-color: var(--border-default);
        }
      `}</style>
    </div>
  );
}

export function useResizePanel({ initialWidth, onWidthChange }: UseResizePanelOptions) {
  const [isDragging, setIsDragging] = useState(false);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);
  const cleanupRef = useRef<(() => void) | null>(null);

  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsDragging(true);
      resizeRef.current = {
        startX: e.clientX,
        startWidth: initialWidth,
      };

      const handleResizeMove = (moveEvent: MouseEvent) => {
        if (!resizeRef.current) return;
        const deltaX = resizeRef.current.startX - moveEvent.clientX;
        const newWidth = resizeRef.current.startWidth + deltaX;
        const maxWidth = window.innerWidth * (MAX_WIDTH_PERCENT / 100);
        onWidthChange(Math.max(MIN_WIDTH, Math.min(maxWidth, newWidth)));
      };

      const handleResizeEnd = () => {
        resizeRef.current = null;
        setIsDragging(false);
        document.removeEventListener("mousemove", handleResizeMove);
        document.removeEventListener("mouseup", handleResizeEnd);
        cleanupRef.current = null;
      };

      document.addEventListener("mousemove", handleResizeMove);
      document.addEventListener("mouseup", handleResizeEnd);

      // Store cleanup function for unmount scenario
      cleanupRef.current = () => {
        document.removeEventListener("mousemove", handleResizeMove);
        document.removeEventListener("mouseup", handleResizeEnd);
      };
    },
    [initialWidth, onWidthChange]
  );

  // Cleanup listeners on unmount
  useEffect(() => {
    return () => {
      if (cleanupRef.current) {
        cleanupRef.current();
      }
    };
  }, []);

  const ResizeHandleComponent = useCallback(() => (
    <ResizeHandle isDragging={isDragging} onMouseDown={handleResizeStart} />
  ), [isDragging, handleResizeStart]);

  return {
    isDragging,
    handleResizeStart,
    ResizeHandle: ResizeHandleComponent,
  };
}
