/**
 * KanbanSplitLayout - Split-screen layout for Kanban view
 *
 * Provides a split layout with:
 * - Left side: Kanban board + task detail overlay (when selected)
 * - Right side: Integrated chat panel (toggleable via header button, resizable)
 *
 * This layout is specific to the Kanban view. Other views continue to use
 * the floating ChatPanel.
 *
 * Resizing works like IdeationView - percentage-based with mouse drag.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useUiStore } from "@/stores/uiStore";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { TaskDetailOverlay } from "@/components/tasks/TaskDetailOverlay";
import { TaskCreationOverlay } from "@/components/tasks/TaskCreationOverlay";
import { ResizeHandle } from "@/components/ui/ResizeHandle";

// ============================================================================
// Constants
// ============================================================================

const MIN_LEFT_PERCENT = 65; // 65% left = 35% right (max chat width)
const MAX_LEFT_PERCENT = 75; // 75% left = 25% right (min chat width)
const DEFAULT_LEFT_PERCENT = 70; // Start with ~30% chat
const LEFT_WIDTH_STORAGE_KEY = "ralphx-kanban-split-left-width";

// ============================================================================
// Main Component
// ============================================================================

interface KanbanSplitLayoutProps {
  children: React.ReactNode;
  projectId: string;
  /** Optional footer to render at the bottom of the left section (e.g., ExecutionControlBar) */
  footer?: React.ReactNode;
}

export function KanbanSplitLayout({ children, projectId, footer }: KanbanSplitLayoutProps) {
  const chatVisible = useUiStore((s) => s.chatVisibleByView.kanban);
  const toggleChatVisible = useUiStore((s) => s.toggleChatVisible);
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const taskCreationContext = useUiStore((s) => s.taskCreationContext);

  // Percentage-based width for left panel (like IdeationView)
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

  // Handle resize move/end (like IdeationView)
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
      data-testid="kanban-split-layout"
      className="flex h-full overflow-hidden"
      style={{ backgroundColor: "hsl(220 10% 8%)" }}
    >
      {/* Left Section - Kanban board */}
      <div
        data-testid="kanban-split-left"
        className="relative flex flex-col overflow-hidden"
        style={{
          width: chatVisible ? `${leftPanelWidth}%` : "100%",
          minWidth: "400px",
          transition: isResizing ? "none" : "width 150ms ease-out",
        }}
      >
        {/* Kanban Board */}
        <div className="flex-1 overflow-hidden">
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

      {/* Resize Handle */}
      {chatVisible && (
        <ResizeHandle
          isResizing={isResizing}
          onMouseDown={handleResizeStart}
          testId="kanban-split-resize-handle"
        />
      )}

      {/* Right Section - Chat Panel with floating glass container */}
      {chatVisible && (
        <div
          data-testid="kanban-split-right"
          className="flex flex-col overflow-hidden shrink-0"
          style={{
            width: `${100 - leftPanelWidth}%`,
            minWidth: "320px",
            transition: isResizing ? "none" : "width 150ms ease-out",
          }}
        >
          <IntegratedChatPanel projectId={projectId} onClose={() => toggleChatVisible("kanban")} />
        </div>
      )}
    </div>
  );
}
