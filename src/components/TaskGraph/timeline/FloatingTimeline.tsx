/**
 * FloatingTimeline - Glass container wrapper for ExecutionTimeline
 *
 * Provides Tahoe liquid glass styling for the timeline when displayed
 * in the GraphSplitLayout right panel.
 */

import { memo } from "react";
import { ExecutionTimeline, type ExecutionTimelineProps } from "./ExecutionTimeline";

export interface FloatingTimelineProps extends Omit<ExecutionTimelineProps, "className" | "defaultCollapsed" | "embedded"> {
  /** Additional className for the outer container */
  className?: string;
  /** Presentation variant */
  variant?: "panel" | "overlay";
}

export const FloatingTimeline = memo(function FloatingTimeline({
  projectId,
  onTaskClick,
  highlightedTaskId,
  className,
  variant = "panel",
}: FloatingTimelineProps) {
  return (
    <div
      className={className}
      style={{
        height: "100%",
        padding: variant === "overlay" ? "0" : "8px",
        backgroundColor: variant === "overlay" ? "transparent" : "hsl(220 10% 8%)",
      }}
    >
      <div
        style={{
          height: "100%",
          borderRadius: "10px",
          background: "hsla(220 10% 10% / 0.92)",
          backdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          boxShadow:
            "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
          overflow: "hidden",
        }}
      >
        <ExecutionTimeline
          projectId={projectId}
          embedded
          {...(onTaskClick && { onTaskClick })}
          {...(highlightedTaskId !== undefined && { highlightedTaskId })}
        />
      </div>
    </div>
  );
});

export default FloatingTimeline;
