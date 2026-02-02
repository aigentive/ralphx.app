/**
 * GraphSplitLayout - Split-screen layout for Graph view
 *
 * Provides a split layout with:
 * - Left side: ReactFlow canvas + task detail overlay (when selected)
 * - Right side: Always visible panel that switches content:
 *   - No task selected: FloatingTimeline (execution timeline)
 *   - Task selected: IntegratedChatPanel
 *
 * Key difference from KanbanSplitLayout:
 * - Kanban: Chat toggleable (can hide completely)
 * - Graph: Right panel always visible, content switches (timeline ↔ chat)
 *
 * Resizing works like IdeationView - percentage-based with mouse drag.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useUiStore } from "@/stores/uiStore";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { TaskDetailOverlay } from "@/components/tasks/TaskDetailOverlay";
import { TaskCreationOverlay } from "@/components/tasks/TaskCreationOverlay";

// ============================================================================
// Constants
// ============================================================================

const MIN_LEFT_PERCENT = 65; // Minimum left panel width (65% left = 35% right max)
const MAX_LEFT_PERCENT = 75; // Maximum left panel width (75% left = 25% right min)
const DEFAULT_LEFT_PERCENT = 70; // Default: 70% left, 30% right
const LEFT_WIDTH_STORAGE_KEY = "ralphx-graph-split-left-width";

// ============================================================================
// Main Component
// ============================================================================

interface GraphSplitLayoutProps {
  /** ReactFlow canvas content */
  children: React.ReactNode;
  /** Project ID for context */
  projectId: string;
  /** Optional footer to render at the bottom of the left section (e.g., ExecutionControlBar) */
  footer?: React.ReactNode;
  /** Timeline content to show when no task is selected */
  timelineContent: React.ReactNode;
}

export function GraphSplitLayout({
  children,
  projectId,
  footer,
  timelineContent,
}: GraphSplitLayoutProps) {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const taskCreationContext = useUiStore((s) => s.taskCreationContext);

  // Percentage-based width for left panel
  const [leftPanelWidth, setLeftPanelWidth] = useState(() => {
    const saved = localStorage.getItem(LEFT_WIDTH_STORAGE_KEY);
    if (saved) {
      const parsed = parseFloat(saved);
      if (!isNaN(parsed) && parsed >= MIN_LEFT_PERCENT && parsed <= MAX_LEFT_PERCENT) {
        return parsed;
      }
    }
    return DEFAULT_LEFT_PERCENT;
  });

  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Persist left panel width to localStorage when it changes
  useEffect(() => {
    localStorage.setItem(LEFT_WIDTH_STORAGE_KEY, leftPanelWidth.toString());
  }, [leftPanelWidth]);

  // Handle resize start
  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  // Handle resize move/end
  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = ((e.clientX - rect.left) / rect.width) * 100;
      setLeftPanelWidth(Math.max(MIN_LEFT_PERCENT, Math.min(MAX_LEFT_PERCENT, newWidth)));
    };

    const handleMouseUp = () => setIsResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  return (
    <div
      ref={containerRef}
      data-testid="graph-split-layout"
      className="flex h-full overflow-hidden"
      style={{ backgroundColor: "hsl(220 10% 8%)" }}
    >
      {/* Left Section - Graph canvas */}
      <div
        data-testid="graph-split-left"
        className="relative flex flex-col overflow-hidden"
        style={{
          width: `${leftPanelWidth}%`,
          minWidth: "400px",
          transition: isResizing ? "none" : "width 150ms ease-out",
        }}
      >
        {/* Graph Canvas */}
        <div className="flex-1 overflow-hidden relative">
          {children}
        </div>

        {/* Footer (e.g., ExecutionControlBar) */}
        {footer && (
          <div className="flex-shrink-0">
            {footer}
          </div>
        )}

        {/* Task Detail Overlay */}
        {selectedTaskId && <TaskDetailOverlay projectId={projectId} />}

        {/* Task Creation Overlay */}
        {taskCreationContext && <TaskCreationOverlay projectId={projectId} />}
      </div>

      {/* Resize Handle - subtle separator line */}
      <div
        data-testid="graph-split-resize-handle"
        className="cursor-ew-resize relative shrink-0"
        style={{
          width: "1px",
          background: "hsla(220 20% 100% / 0.04)",
        }}
        onMouseDown={handleResizeStart}
      />

      {/* Right Section - Timeline or Chat (always visible, content switches) */}
      <div
        data-testid="graph-split-right"
        className="flex flex-col overflow-hidden shrink-0"
        style={{
          width: `${100 - leftPanelWidth}%`,
          minWidth: "320px",
          transition: isResizing ? "none" : "width 150ms ease-out",
        }}
      >
        {selectedTaskId ? (
          // Task selected: show chat panel
          <IntegratedChatPanel projectId={projectId} />
        ) : (
          // No task selected: show timeline
          timelineContent
        )}
      </div>
    </div>
  );
}
