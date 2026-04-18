/**
 * TaskDetailOverlay - Inline task detail panel for split-screen layout
 *
 * This replaces TaskDetailModal and TaskFullView for the Kanban view.
 * It displays as an overlay within the left section of the split layout
 * with blur backdrop matching the current modal aesthetic.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import React, { useCallback, useEffect, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { TaskDetailPanel } from "./TaskDetailPanel";
import { TaskEditForm } from "./TaskEditForm";
import { StatusDropdown } from "./StatusDropdown";
import { StateTimelineNav } from "./StateTimelineNav";
import { useQuery } from "@tanstack/react-query";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
import { useTasks, taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import type { Task, InternalStatus } from "@/types/task";
import {
  X,
  Pencil,
  Archive,
  RotateCcw,
  Loader2,
  Lightbulb,
  History,
  ScrollText,
} from "lucide-react";
import { useIdeationStore } from "@/stores/ideationStore";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useConfirmation } from "@/hooks/useConfirmation";
import { logger } from "@/lib/logger";
import { toast } from "sonner";
import { AuditTrailDialog } from "@/components/tasks/AuditTrailDialog";

// ============================================================================
// Priority Colors (Tahoe HSL palette)
// ============================================================================

const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-surface)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

const DEFAULT_PRIORITY_COLOR = { bg: "var(--bg-hover)", text: "var(--text-secondary)" };

// ============================================================================
// Status Badge Configuration (Tahoe HSL palette)
// ============================================================================

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
    bg: "var(--status-info-muted)",
    text: "var(--status-info)",
  },
  blocked: {
    label: "Blocked",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  executing: {
    label: "Executing",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "var(--status-success-muted)",
    text: "var(--status-success)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "var(--status-error-muted)",
    text: "var(--status-error)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  approved: {
    label: "Approved",
    bg: "var(--status-success-muted)",
    text: "var(--status-success)",
  },
  failed: {
    label: "Failed",
    bg: "var(--status-error-muted)",
    text: "var(--status-error)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  reviewing: {
    label: "AI Review in Progress",
    bg: "var(--status-info-muted)",
    text: "var(--status-info)",
  },
  review_passed: {
    label: "AI Review Passed",
    bg: "var(--status-success-muted)",
    text: "var(--status-success)",
  },
  escalated: {
    label: "Escalated",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  pending_merge: {
    label: "Pending Merge",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  merging: {
    label: "Merging",
    bg: "var(--accent-muted)",
    text: "var(--accent-primary)",
  },
  merge_incomplete: {
    label: "Merge Incomplete",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  merge_conflict: {
    label: "Merge Conflict",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  merged: {
    label: "Merged",
    bg: "var(--status-success-muted)",
    text: "var(--status-success)",
  },
  paused: {
    label: "Paused",
    bg: "var(--status-warning-muted)",
    text: "var(--status-warning)",
  },
  stopped: {
    label: "Stopped",
    bg: "var(--status-error-muted)",
    text: "var(--status-error)",
  },
};

// ============================================================================
// Sub-components
// ============================================================================

function PriorityBadge({ priority }: { priority: number }) {
  const colors = PRIORITY_COLORS[priority] ?? DEFAULT_PRIORITY_COLOR;
  return (
    <span
      data-testid="task-overlay-priority"
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
      data-testid="task-overlay-status"
      data-status={status}
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0"
      style={{ backgroundColor: config.bg, color: config.text }}
    >
      {config.label}
    </Badge>
  );
}

// ============================================================================
// Main Component
// ============================================================================

interface TaskDetailOverlayProps {
  projectId: string;
  /** Optional footer to render at the bottom of the overlay (e.g., ExecutionControlBar) */
  footer?: React.ReactNode;
}

export function TaskDetailOverlay({ projectId, footer }: TaskDetailOverlayProps) {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  // History state from store - shared with IntegratedChatPanel
  const historyState = useUiStore((s) => s.taskHistoryState);
  const setHistoryState = useUiStore((s) => s.setTaskHistoryState);

  // Debug logging for history state
  logger.debug('[TaskDetailOverlay] History state from store:', historyState);

  // Ideation hooks
  const addSession = useIdeationStore((state) => state.addSession);
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);
  const createSession = useCreateIdeationSession();

  // Try to get task from store first, fall back to fetching from API
  const taskFromStore = useTaskStore((state) =>
    selectedTaskId ? state.tasks[selectedTaskId] : undefined
  );

  // Fetch all tasks to ensure we have the latest data
  const { data: tasks = [] } = useTasks(projectId);

  // Find the task from the list query
  const taskFromList = tasks.find((t) => t.id === selectedTaskId);

  // Fallback: fetch the specific task by ID when not found in store or paginated list
  const { data: taskFromDetail } = useQuery<Task, Error>({
    queryKey: taskKeys.detail(selectedTaskId ?? ""),
    queryFn: () => api.tasks.get(selectedTaskId!),
    enabled: Boolean(selectedTaskId) && !taskFromStore && !taskFromList,
  });

  const task: Task | undefined = taskFromStore || taskFromList || taskFromDetail;

  const [isEditing, setIsEditing] = useState(false);
  const [showAuditTrail, setShowAuditTrail] = useState(false);

  // Derived values for history mode (historyState from store)
  const isHistoryMode = historyState !== null;
  const viewStatus = (historyState?.status as InternalStatus | undefined) ?? task?.internalStatus;

  // Get mutations
  const {
    updateMutation,
    moveMutation,
    archiveMutation,
    restoreMutation,
    isArchiving,
    isRestoring,
  } = useTaskMutation(projectId);

  // Confirmation dialog for archive/restore
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

  // Close overlay on Escape key (exit edit mode first, then close overlay)
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        if (isEditing) {
          // If editing, just exit edit mode (go back to view)
          setIsEditing(false);
        } else {
          // If viewing, close the overlay
          setSelectedTaskId(null);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setSelectedTaskId, isEditing]);

  // Reset editing and history state when task changes
  useEffect(() => {
    setIsEditing(false);
    setHistoryState(null);
  }, [selectedTaskId, setHistoryState]);

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      // Only close if clicking the backdrop itself, not its children
      if (event.target === event.currentTarget) {
        setSelectedTaskId(null);
        setIsEditing(false);
      }
    },
    [setSelectedTaskId]
  );

  // Handle close
  const handleClose = useCallback(() => {
    setSelectedTaskId(null);
    setIsEditing(false);
  }, [setSelectedTaskId]);

  // Handle edit save
  const handleSave = (updateData: Parameters<typeof updateMutation.mutate>[0]['input']) => {
    if (!task) return;
    updateMutation.mutate(
      { taskId: task.id, input: updateData },
      {
        onSuccess: () => {
          setIsEditing(false);
        },
      }
    );
  };

  // Handle status change
  const handleStatusChange = (newStatus: string) => {
    if (!task) return;
    moveMutation.mutate({ taskId: task.id, toStatus: newStatus });
  };

  // Handle archive
  const handleArchive = async () => {
    if (!task) return;
    const confirmed = await confirm({
      title: "Archive this task?",
      description: "The task will be moved to the archive.",
      confirmText: "Archive",
    });
    if (!confirmed) return;
    archiveMutation.mutate(task.id, {
      onSuccess: () => {
        handleClose();
      },
    });
  };

  // Handle restore
  const handleRestore = async () => {
    if (!task) return;
    const confirmed = await confirm({
      title: "Restore this task?",
      description: "The task will be restored to the backlog.",
      confirmText: "Restore",
    });
    if (!confirmed) return;
    restoreMutation.mutate(task.id, {
      onSuccess: () => {
        handleClose();
      },
    });
  };

  // Handle start ideation
  const handleStartIdeation = async () => {
    if (!task) return;
    try {
      // Create session with seedTaskId
      const session = await createSession.mutateAsync({
        projectId: task.projectId,
        title: `Ideation: ${task.title}`,
        seedTaskId: task.id,
      });
      // Add session to store and set as active
      addSession(session);
      setActiveSession(session.id);
      // Close overlay and navigate to ideation view
      handleClose();
      setCurrentView("ideation");
    } catch (error) {
      console.error("Failed to start ideation:", error);
      toast.error("Failed to start ideation session");
    }
  };

  // Don't render if no task is selected
  if (!selectedTaskId || !task) {
    if (selectedTaskId && !task) {
      console.warn('[TaskDetailOverlay] Task not found for selectedTaskId:', selectedTaskId);
    }
    return null;
  }

  // Determine if task is editable
  const systemControlledStatuses: InternalStatus[] = [
    "executing",
    "qa_refining",
    "qa_testing",
    "qa_passed",
    "qa_failed",
    "pending_review",
    "revision_needed",
    "reviewing",
    "review_passed",
    "re_executing",
  ];

  const isArchived = !!task.archivedAt;
  const isSystemControlled = systemControlledStatuses.includes(task.internalStatus);
  const canEdit = !isArchived && !isSystemControlled;
  // "Backlog" is the equivalent of "draft" - tasks that haven't started execution yet
  const isBacklog = task.internalStatus === "backlog";

  return (
    <>
      {/* Full-page container - same bg as Kanban */}
      <div
        data-testid="task-overlay-backdrop"
        className="absolute inset-0 z-40 flex"
        style={{
          backgroundColor: "var(--bg-base)",
        }}
        onClick={handleBackdropClick}
      >
        {/* Content area - full width, no boxing */}
        <div
          data-testid="task-detail-overlay"
          data-task-id={task.id}
          className="flex-1 flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header - flat Tahoe styling */}
          <div
            className="px-6 pt-5 pb-4 shrink-0"
            style={{
              borderBottom: "1px solid var(--overlay-weak)",
            }}
          >
            {/* Archived Badge */}
            {isArchived && (
              <div
                data-testid="archived-badge"
                className="mb-3 px-2.5 py-1.5 rounded-lg flex items-center gap-2 w-fit"
                style={{
                  backgroundColor: "var(--accent-muted)",
                  border: "1px solid var(--accent-border)",
                }}
              >
                <Archive className="w-3.5 h-3.5" style={{ color: "var(--accent-primary)" }} />
                <span className="text-[12px] font-medium" style={{ color: "var(--accent-primary)" }}>Archived</span>
              </div>
            )}
            <div className="flex items-start gap-2.5 pr-28">
              <PriorityBadge priority={task.priority} />
              <div className="flex-1 min-w-0">
                <h2
                  data-testid="task-overlay-title"
                  className="text-base font-semibold truncate"
                  style={{
                    color: "var(--text-primary)",
                    letterSpacing: "-0.02em",
                    lineHeight: "1.3",
                  }}
                >
                  {task.title}
                </h2>
                <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
                  <span
                    data-testid="task-overlay-category"
                    className="px-1.5 py-0.5 rounded text-[10px] font-medium"
                    style={{
                      backgroundColor: "var(--overlay-weak)",
                      border: "1px solid var(--overlay-moderate)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {task.category}
                  </span>
                  <StatusBadge status={task.internalStatus} />
                </div>
              </div>
            </div>

            {/* Action buttons */}
            <div className="absolute top-4 right-4 flex items-center gap-2">
              {/* StatusDropdown - only for user-controlled statuses */}
              {canEdit && (
                <StatusDropdown
                  taskId={task.id}
                  currentStatus={task.internalStatus}
                  onTransition={handleStatusChange}
                  disabled={moveMutation.isPending}
                />
              )}
              {/* Start Ideation button - only for backlog (draft) tasks */}
              {isBacklog && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleStartIdeation}
                  disabled={createSession.isPending}
                  data-testid="task-overlay-ideation-button"
                  aria-label="Start Ideation"
                  style={{ color: "var(--text-muted)" }}
                  className="hover:bg-[hsla(220_10%_100%/0.05)]"
                >
                  {createSession.isPending ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Lightbulb className="w-4 h-4" />
                  )}
                </Button>
              )}
              {/* Edit button */}
              {canEdit && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => setIsEditing(!isEditing)}
                  data-testid="task-overlay-edit-button"
                  aria-label={isEditing ? "Cancel editing" : "Edit task"}
                  style={{ color: "var(--text-muted)" }}
                  className="hover:bg-[hsla(220_10%_100%/0.05)]"
                >
                  <Pencil className="w-4 h-4" />
                </Button>
              )}
              {/* Archive button */}
              {!isArchived && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleArchive}
                  disabled={isArchiving}
                  data-testid="task-overlay-archive-button"
                  aria-label="Archive task"
                  style={{ color: "var(--text-muted)" }}
                  className="hover:bg-[hsla(220_10%_100%/0.05)]"
                >
                  {isArchiving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Archive className="w-4 h-4" />
                  )}
                </Button>
              )}
              {/* Restore button */}
              {isArchived && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleRestore}
                  disabled={isRestoring}
                  data-testid="task-overlay-restore-button"
                  aria-label="Restore task"
                  style={{ color: "var(--text-muted)" }}
                  className="hover:bg-[hsla(220_10%_100%/0.05)]"
                >
                  {isRestoring ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <RotateCcw className="w-4 h-4" />
                  )}
                </Button>
              )}
              {/* Audit Trail button */}
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setShowAuditTrail(true)}
                data-testid="task-overlay-audit-trail-button"
                aria-label="Audit Trail"
                title="Audit Trail"
                style={{ color: "var(--text-muted)" }}
                className="hover:bg-[hsla(220_10%_100%/0.05)]"
              >
                <ScrollText className="w-4 h-4" />
              </Button>
              {/* Close button */}
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleClose}
                data-testid="task-overlay-close"
                aria-label="Close"
                style={{ color: "var(--text-muted)" }}
                className="hover:bg-[hsla(220_10%_100%/0.05)]"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          </div>

          {/* State Timeline Navigation - for viewing historical states (hidden in edit mode) */}
          {!isEditing && (
            <StateTimelineNav
              taskId={task.id}
              currentStatus={task.internalStatus}
              onStateSelect={setHistoryState}
              selectedState={historyState}
            />
          )}

          {/* History Mode Banner */}
          {isHistoryMode && (
            <div
              data-testid="history-mode-banner"
              className="px-4 py-1.5 flex items-center gap-2 shrink-0"
            >
              <History className="w-3 h-3" style={{ color: "var(--text-muted)" }} />
              <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>
                Viewing: {STATUS_CONFIG[historyState.status]?.label ?? historyState.status}
              </span>
              <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>
                {new Date(historyState.timestamp).toLocaleString()}
              </span>
            </div>
          )}

          {/* Scrollable Content */}
          {isEditing ? (
            /* Edit Mode - No ScrollArea, form handles its own layout */
            <div className="flex-1 flex flex-col overflow-auto px-6 py-4">
              <TaskEditForm
                task={task}
                onSave={handleSave}
                onCancel={() => setIsEditing(false)}
                isSaving={updateMutation.isPending}
              />
            </div>
          ) : (
            /* Read-only View */
            <ScrollArea className="flex-1">
              <div className="px-6 py-4">
                <ErrorBoundary>
                  <TaskDetailPanel
                    task={task}
                    showHeader={false}
                    showContext={true}
                    showHistory={true}
                    useViewRegistry={true}
                    {...(isHistoryMode && viewStatus ? { viewAsStatus: viewStatus } : {})}
                    {...(isHistoryMode && historyState?.timestamp
                      ? { viewTimestamp: historyState.timestamp }
                      : {})}
                  />
                </ErrorBoundary>
              </div>
            </ScrollArea>
          )}

          {/* Execution Control Bar - always visible at bottom of overlay */}
          {footer && (
            <div className="flex-shrink-0">
              {footer}
            </div>
          )}
        </div>
      </div>

      {/* Archive/Restore Confirmation Dialog */}
      <ConfirmationDialog {...confirmationDialogProps} />

      {/* Audit Trail Dialog */}
      <AuditTrailDialog
        taskId={task.id}
        isOpen={showAuditTrail}
        onClose={() => setShowAuditTrail(false)}
      />
    </>
  );
}
