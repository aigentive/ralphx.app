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
import { ResizeHandle, CHAT_PANEL_DEFAULT_WIDTH, CHAT_PANEL_MIN_WIDTH } from "@/components/ui/ResizeHandle";

// ============================================================================
// Constants
// ============================================================================

const MAX_CHAT_WIDTH = 600; // Maximum chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-kanban-chat-width";

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

  // Chat panel width (pixel-based)
  const [chatPanelWidth, setChatPanelWidth] = useState(() => {
    const saved = localStorage.getItem(CHAT_WIDTH_STORAGE_KEY);
    if (saved) {
      const parsed = parseInt(saved, 10);
      if (!isNaN(parsed) && parsed >= CHAT_PANEL_MIN_WIDTH && parsed <= MAX_CHAT_WIDTH) {
        return parsed;
      }
    }
    return CHAT_PANEL_DEFAULT_WIDTH;
  });

  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Persist chat panel width
  useEffect(() => {
    localStorage.setItem(CHAT_WIDTH_STORAGE_KEY, chatPanelWidth.toString());
  }, [chatPanelWidth]);

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
      // Chat panel is on the right, so width = container right edge - mouse position
      const newWidth = rect.right - e.clientX;
      setChatPanelWidth(Math.max(CHAT_PANEL_MIN_WIDTH, Math.min(MAX_CHAT_WIDTH, newWidth)));
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
        className="relative flex-1 flex flex-col overflow-hidden min-w-0"
        style={{
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
            width: `${chatPanelWidth}px`,
            transition: isResizing ? "none" : "width 150ms ease-out",
          }}
        >
          <IntegratedChatPanel projectId={projectId} onClose={() => toggleChatVisible("kanban")} />
        </div>
      )}
    </div>
  );
}
