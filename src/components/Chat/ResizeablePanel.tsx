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
