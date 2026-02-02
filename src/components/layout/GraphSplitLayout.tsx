/**
 * GraphSplitLayout - Split-screen layout for Graph view
 *
 * Provides a split layout with:
 * - Left side: ReactFlow canvas + task detail overlay (takes remaining space)
 * - Right side: Panel that switches content and sizing:
 *   - No task selected: FloatingTimeline at fixed 320px
 *   - Task selected: IntegratedChatPanel with resizable width
 *
 * Key difference from KanbanSplitLayout:
 * - Kanban: Chat toggleable (can hide completely)
 * - Graph: Right panel always visible, content switches (timeline ↔ chat)
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useUiStore } from "@/stores/uiStore";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { TaskDetailOverlay } from "@/components/tasks/TaskDetailOverlay";
import { TaskCreationOverlay } from "@/components/tasks/TaskCreationOverlay";
import { ResizeHandle, SeparatorLine } from "@/components/ui/ResizeHandle";

// ============================================================================
// Constants
// ============================================================================

// Fixed timeline sidebar width (px) - non-resizable
const TIMELINE_SIDEBAR_WIDTH = 320;

// Chat panel resize constraints (percentage-based)
const MIN_LEFT_PERCENT = 60; // Minimum left panel width (60% left = 40% right max)
const MAX_LEFT_PERCENT = 80; // Maximum left panel width (80% left = 20% right min)
const DEFAULT_LEFT_PERCENT = 70; // Default: 70% left, 30% right
const LEFT_WIDTH_STORAGE_KEY = "ralphx-graph-chat-left-width";

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

  // Chat panel resize state (only used when task is selected)
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

  // Persist chat panel width
  useEffect(() => {
    localStorage.setItem(LEFT_WIDTH_STORAGE_KEY, leftPanelWidth.toString());
  }, [leftPanelWidth]);

  // Handle resize
  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

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

  // Show chat (resizable) when task selected, timeline (fixed) otherwise
  const showChat = !!selectedTaskId;

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
        className="relative flex flex-col overflow-hidden min-w-0"
        style={showChat ? {
          width: `${leftPanelWidth}%`,
          minWidth: "400px",
          transition: isResizing ? "none" : "width 150ms ease-out",
        } : {
          flex: 1,
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

      {/* Resize Handle - interactive when chat is shown, static separator for timeline */}
      {showChat ? (
        <ResizeHandle
          isResizing={isResizing}
          onMouseDown={handleResizeStart}
          testId="graph-split-resize-handle"
        />
      ) : (
        <SeparatorLine />
      )}

      {/* Right Section - Timeline (fixed 320px) or Chat (resizable) */}
      <div
        data-testid="graph-split-right"
        className="flex flex-col overflow-hidden shrink-0"
        style={showChat ? {
          width: `${100 - leftPanelWidth}%`,
          minWidth: "280px",
          transition: isResizing ? "none" : "width 150ms ease-out",
        } : {
          width: `${TIMELINE_SIDEBAR_WIDTH}px`,
        }}
      >
        {showChat ? (
          <IntegratedChatPanel projectId={projectId} />
        ) : (
          timelineContent
        )}
      </div>
    </div>
  );
}
