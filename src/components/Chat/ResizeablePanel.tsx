/**
 * ResizeablePanel - Reusable panel with resize functionality
 */

import { useRef, useState, useCallback, type ReactNode } from "react";
import { cn } from "@/lib/utils";

const MIN_WIDTH = 320;
const MAX_WIDTH_PERCENT = 50;

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
        className={cn(
          "absolute top-1/2 left-1/2 w-0.5 h-12 -translate-x-1/2 -translate-y-1/2 rounded-sm transition-all duration-150",
          isDragging ? "h-16" : ""
        )}
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

export interface UseResizePanelOptions {
  initialWidth: number;
  onWidthChange: (width: number) => void;
}

export function useResizePanel({ initialWidth, onWidthChange }: UseResizePanelOptions) {
  const [isDragging, setIsDragging] = useState(false);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);

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
      };

      document.addEventListener("mousemove", handleResizeMove);
      document.addEventListener("mouseup", handleResizeEnd);
    },
    [initialWidth, onWidthChange]
  );

  const ResizeHandleComponent = useCallback(() => (
    <ResizeHandle isDragging={isDragging} onMouseDown={handleResizeStart} />
  ), [isDragging, handleResizeStart]);

  return {
    isDragging,
    handleResizeStart,
    ResizeHandle: ResizeHandleComponent,
  };
}

export interface ResizeablePanelProps {
  width: number;
  children: ReactNode;
  isExiting?: boolean;
  testId?: string;
  ariaLabel?: string;
  ResizeHandle: React.ComponentType;
}

export function ResizeablePanel({
  width,
  children,
  isExiting = false,
  testId = "resizeable-panel",
  ariaLabel = "Resizeable panel",
  ResizeHandle,
}: ResizeablePanelProps) {
  return (
    <aside
      data-testid={testId}
      role="complementary"
      aria-label={ariaLabel}
      className={cn(
        "fixed top-14 right-0 bottom-0 flex flex-col",
        isExiting ? "chat-panel-exit" : "chat-panel-enter"
      )}
      style={{
        width: `${width}px`,
        minWidth: `${MIN_WIDTH}px`,
        backgroundColor: "var(--bg-surface)",
        borderLeft: "1px solid var(--border-subtle)",
        boxShadow: "var(--shadow-md)",
        zIndex: 40,
      }}
    >
      <ResizeHandle />
      {children}
    </aside>
  );
}

export { MIN_WIDTH, MAX_WIDTH_PERCENT };
