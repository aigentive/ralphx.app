/**
 * TaskFullView - Full-screen task view with split layout
 *
 * Raycast-style full-screen overlay with 24px margin.
 * Split layout: left panel (TaskDetailPanel) | right panel (TaskChatPanel)
 * Resizable panels with drag handle, default 50/50 split.
 *
 * Design spec: specs/plans/task-execution-experience.md
 * - Full-screen overlay replacing modal for executing tasks
 * - Context-aware chat based on task state
 * - Integrated task details and execution monitoring
 */

import { useEffect, useState, useCallback, useMemo } from "react";
import { X, ArrowLeft, Pencil, Archive, RotateCcw, Pause, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TaskDetailPanel } from "./TaskDetailPanel";
import { TaskChatPanel } from "./TaskChatPanel";
import { useTasks } from "@/hooks/useTasks";
import { useProjectStore } from "@/stores/projectStore";
import type { InternalStatus } from "@/types/task";

interface TaskFullViewProps {
  taskId: string;
  onClose: () => void;
}

// Status badge configuration matching design spec
const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; bg: string; text: string }
> = {
  backlog: {
    label: "Backlog",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  ready: {
    label: "Ready",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  blocked: {
    label: "Blocked",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  executing: {
    label: "Executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  approved: {
    label: "Approved",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  failed: {
    label: "Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  reviewing: {
    label: "AI Review in Progress",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  review_passed: {
    label: "AI Review Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
};

const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-base)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

const DEFAULT_PRIORITY_COLOR = { bg: "var(--bg-hover)", text: "var(--text-secondary)" };

function PriorityBadge({ priority }: { priority: number }) {
  const colors = PRIORITY_COLORS[priority] ?? DEFAULT_PRIORITY_COLOR;
  return (
    <span
      data-testid="task-fullview-priority"
      className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium"
      style={{ backgroundColor: colors.bg, color: colors.text }}
    >
      P{priority}
    </span>
  );
}

function StatusBadge({ status }: { status: InternalStatus }) {
  const config = STATUS_CONFIG[status];
  return (
    <Badge
      data-testid="task-fullview-status"
      data-status={status}
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0"
      style={{ backgroundColor: config.bg, color: config.text }}
    >
      {config.label}
    </Badge>
  );
}

export function TaskFullView({ taskId, onClose }: TaskFullViewProps) {
  const { activeProjectId } = useProjectStore();

  // Fetch task from cache first, or trigger new fetch
  const { data: tasks = [] } = useTasks(activeProjectId || "");
  const task = useMemo(
    () => tasks.find((t) => t.id === taskId),
    [tasks, taskId]
  );

  // Panel width state (default 50%)
  const [panelWidth, setPanelWidth] = useState(() => {
    const saved = localStorage.getItem("taskFullView:panelWidth");
    return saved ? parseFloat(saved) : 50;
  });

  // Determine context type based on task status
  const contextType = useMemo((): "task" | "task_execution" | "review" => {
    if (!task) return "task";

    // Review states route to reviewer agent
    const reviewStatuses: InternalStatus[] = ["reviewing", "review_passed"];
    if (reviewStatuses.includes(task.internalStatus)) {
      return "review";
    }

    // Execution states route to worker agent
    const executingStatuses: InternalStatus[] = [
      "executing",
      "re_executing",
      "qa_refining",
      "qa_testing",
      "qa_passed",
      "qa_failed",
    ];
    return executingStatuses.includes(task.internalStatus)
      ? "task_execution"
      : "task";
  }, [task]);

  const isExecuting = contextType === "task_execution" || contextType === "review";

  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Handle close
  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  // Handlers for action buttons (stub for now, will be implemented when integrated)
  const handleEdit = useCallback(() => {
    // TODO: Open edit modal/form
  }, []);

  const handleArchive = useCallback(() => {
    // TODO: Archive task
  }, []);

  const handlePause = useCallback(() => {
    // TODO: Pause execution
  }, []);

  const handleStop = useCallback(() => {
    // TODO: Stop execution
  }, []);

  if (!task) {
    return (
      <div
        data-testid="task-fullview-loading"
        className="fixed inset-0 z-50 flex items-center justify-center"
        style={{ backgroundColor: "var(--bg-base)" }}
      >
        <div className="text-[13px] text-white/60">Loading task...</div>
      </div>
    );
  }

  return (
    <div
      data-testid="task-fullview"
      data-task-id={taskId}
      className="fixed z-50 flex flex-col"
      style={{
        top: "var(--space-6)", // 24px
        right: "var(--space-6)",
        bottom: "var(--space-6)",
        left: "var(--space-6)",
        backgroundColor: "var(--bg-surface)",
        borderRadius: "var(--radius-lg)",
        border: "1px solid var(--border-subtle)",
        boxShadow:
          "0 8px 32px rgba(0,0,0,0.4), 0 2px 8px rgba(0,0,0,0.2)",
      }}
    >
      {/* Header */}
      <div
        data-testid="task-fullview-header"
        className="flex items-center justify-between h-14 px-4 border-b shrink-0"
        style={{
          borderColor: "var(--border-subtle)",
          background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%)",
        }}
      >
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleClose}
            data-testid="task-fullview-back-button"
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>

          <PriorityBadge priority={task.priority} />

          <h2
            data-testid="task-fullview-title"
            className="text-sm font-semibold text-white/90 truncate"
            style={{
              letterSpacing: "-0.02em",
              lineHeight: "1.3",
            }}
          >
            {task.title}
          </h2>

          <StatusBadge status={task.internalStatus} />
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleEdit}
            data-testid="task-fullview-edit-button"
          >
            <Pencil className="h-4 w-4" />
          </Button>

          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleArchive}
            data-testid="task-fullview-archive-button"
          >
            {task.archivedAt ? (
              <RotateCcw className="h-4 w-4" />
            ) : (
              <Archive className="h-4 w-4" />
            )}
          </Button>

          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleClose}
            data-testid="task-fullview-close-button"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Split Layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Panel - Task Details */}
        <div
          data-testid="task-fullview-left-panel"
          className="overflow-y-auto border-r"
          style={{
            width: `${panelWidth}%`,
            minWidth: "360px",
            borderColor: "var(--border-subtle)",
          }}
        >
          <div className="p-6">
            <TaskDetailPanel task={task} showHistory={true} useViewRegistry={true} />
          </div>
        </div>

        {/* Drag Handle */}
        <div
          data-testid="task-fullview-drag-handle"
          className="w-1 cursor-col-resize hover:bg-accent-primary/20 transition-colors shrink-0"
          style={{
            backgroundColor: "var(--border-subtle)",
          }}
          onMouseDown={(e) => {
            e.preventDefault();
            const startX = e.clientX;
            const startWidth = panelWidth;
            const containerWidth = e.currentTarget.parentElement?.clientWidth || 1;

            const handleMouseMove = (moveEvent: MouseEvent) => {
              const deltaX = moveEvent.clientX - startX;
              const deltaPercent = (deltaX / containerWidth) * 100;
              let newWidth = startWidth + deltaPercent;

              // Clamp to ensure minimum 360px on each side
              const minPercent = (360 / containerWidth) * 100;
              const maxPercent = 100 - minPercent;
              newWidth = Math.max(minPercent, Math.min(maxPercent, newWidth));

              setPanelWidth(newWidth);
              localStorage.setItem("taskFullView:panelWidth", newWidth.toString());
            };

            const handleMouseUp = () => {
              document.removeEventListener("mousemove", handleMouseMove);
              document.removeEventListener("mouseup", handleMouseUp);
            };

            document.addEventListener("mousemove", handleMouseMove);
            document.addEventListener("mouseup", handleMouseUp);
          }}
        />

        {/* Right Panel - Chat */}
        <div
          data-testid="task-fullview-right-panel"
          className="flex-1 overflow-hidden"
          style={{
            minWidth: "360px",
          }}
        >
          <TaskChatPanel taskId={taskId} contextType={contextType} taskStatus={task.internalStatus} />
        </div>
      </div>

      {/* Footer - Execution Controls */}
      {isExecuting && (
        <div
          data-testid="task-fullview-footer"
          className="flex items-center justify-end gap-2 h-12 px-4 border-t shrink-0"
          style={{
            borderColor: "var(--border-subtle)",
            backgroundColor: "var(--bg-surface)",
          }}
        >
          <Button
            variant="outline"
            size="sm"
            onClick={handlePause}
            data-testid="task-fullview-pause-button"
          >
            <Pause className="h-4 w-4 mr-2" />
            Pause
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={handleStop}
            data-testid="task-fullview-stop-button"
          >
            <Square className="h-4 w-4 mr-2" />
            Stop
          </Button>
        </div>
      )}
    </div>
  );
}
