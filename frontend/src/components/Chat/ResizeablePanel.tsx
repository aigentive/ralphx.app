/**
 * ResizeablePanel - Reusable panel with resize functionality
 */

import { type ReactNode } from "react";
import { cn } from "@/lib/utils";
import { MIN_WIDTH } from "./ResizeablePanel.constants";

export interface ResizeablePanelProps {
  width: number;
  children: ReactNode;
  isExiting?: boolean;
  testId?: string;
  ariaLabel?: string;
  ResizeHandle: React.ComponentType;
  /**
   * Bottom offset in px. Set to 76 when the ExecutionControlBar renders below
   * this panel (Kanban/Graph), 0 elsewhere (Activity/Extensibility/Reviews/
   * Settings). Hardcoding 76 everywhere leaves ~84px of wasted empty space
   * below the composer on views that don't render the execution bar.
   */
  bottomOffset?: number;
}

export function ResizeablePanel({
  width,
  children,
  isExiting = false,
  testId = "resizeable-panel",
  ariaLabel = "Resizeable panel",
  ResizeHandle,
  bottomOffset = 0,
}: ResizeablePanelProps) {
  return (
    <aside
      data-testid={testId}
      role="complementary"
      aria-label={ariaLabel}
      // Note: no slide-in transform animation on this wrapper — a transform on
      // a fixed element creates a new stacking context that lets same-document
      // buttons with z-auto paint on top. Use opacity-only transitions if a
      // slide-in is desired.
      className={cn(
        "fixed top-14 right-0 flex flex-col",
        isExiting && "chat-panel-exit"
      )}
      style={{
        width: `${width + 16}px`,
        minWidth: `${MIN_WIDTH + 16}px`,
        bottom: `${bottomOffset}px`,
        zIndex: 50,
        background: "var(--bg-elevated)",
      }}
    >
      {/* Inner rounded container — borders + shadow do the framing. */}
      <div
        className="flex flex-col flex-1 rounded-[10px] overflow-hidden m-2"
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid var(--border-subtle)",
          boxShadow: "var(--shadow-md)",
        }}
      >
        <ResizeHandle />
        {children}
      </div>
    </aside>
  );
}
