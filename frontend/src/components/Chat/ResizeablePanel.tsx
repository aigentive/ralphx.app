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
        bottom: "76px",
        zIndex: 50,
        background: "var(--bg-elevated)",
      }}
    >
      {/* Floating panel inner container */}
      <div
        className="flex flex-col flex-1 rounded-[10px] overflow-hidden"
        style={{
          margin: "8px",
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
