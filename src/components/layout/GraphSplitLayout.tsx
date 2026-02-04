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
import { ResizeHandle, SeparatorLine, CHAT_PANEL_DEFAULT_WIDTH, CHAT_PANEL_MIN_WIDTH } from "@/components/ui/ResizeHandle";

// ============================================================================
// Constants
// ============================================================================

// Fixed timeline sidebar width (px) - non-resizable
const TIMELINE_SIDEBAR_WIDTH = 320;

// Chat panel resize constraints (pixel-based)
const MAX_CHAT_WIDTH = 600; // Maximum chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-graph-chat-width";

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
  /** Whether right panel should be visible */
  rightPanelVisible: boolean;
}

export function GraphSplitLayout({
  children,
  projectId,
  footer,
  timelineContent,
  rightPanelVisible,
}: GraphSplitLayoutProps) {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const taskCreationContext = useUiStore((s) => s.taskCreationContext);

  // Chat panel resize state (pixel-based, only used when task is selected)
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
        className="relative flex-1 flex flex-col overflow-hidden min-w-0"
        style={{
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

      {rightPanelVisible && (
        <>
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
            style={{
              width: showChat ? `${chatPanelWidth}px` : `${TIMELINE_SIDEBAR_WIDTH}px`,
              transition: isResizing ? "none" : "width 150ms ease-out",
            }}
          >
            {showChat ? (
              <IntegratedChatPanel projectId={projectId} />
            ) : (
              timelineContent
            )}
          </div>
        </>
      )}
    </div>
  );
}
