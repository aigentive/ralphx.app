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
        "fixed top-14 right-0 flex flex-col",
        isExiting ? "chat-panel-exit" : "chat-panel-enter"
      )}
      style={{
        width: `${width + 16}px`,
        minWidth: `${MIN_WIDTH + 16}px`,
        bottom: "76px",
        zIndex: 40,
      }}
    >
      {/* Floating panel inner container */}
      <div
        className="flex flex-col flex-1 rounded-[10px] overflow-hidden"
        style={{
          margin: "8px",
          background: "hsla(220 10% 10% / 0.92)",
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        <ResizeHandle />
        {children}
      </div>
    </aside>
  );
}
