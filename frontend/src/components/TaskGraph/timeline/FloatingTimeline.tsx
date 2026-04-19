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
      data-testid="floating-timeline"
      className={className}
      style={{
        height: "100%",
        padding: variant === "overlay" ? "0" : "8px",
        backgroundColor: variant === "overlay" ? "transparent" : "var(--bg-base)",
      }}
    >
      <div
        style={{
          height: "100%",
          borderRadius: "10px",
          background: "var(--bg-surface)",
          backdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid var(--overlay-weak)",
          boxShadow:
            "0 4px 16px var(--overlay-scrim), 0 12px 32px var(--overlay-scrim)",
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
