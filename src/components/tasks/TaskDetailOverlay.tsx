/**
 * TaskDetailOverlay - Inline task detail panel for split-screen layout
 *
 * This replaces TaskDetailModal and TaskFullView for the Kanban view.
 * It displays as an overlay within the left section of the split layout
 * with blur backdrop matching the current modal aesthetic.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useCallback, useEffect, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TaskDetailPanel } from "./TaskDetailPanel";
import { TaskEditForm } from "./TaskEditForm";
import { StatusDropdown } from "./StatusDropdown";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
import { useTasks } from "@/hooks/useTasks";
import type { Task, InternalStatus } from "@/types/task";
import {
  X,
  Pencil,
  Archive,
  RotateCcw,
  Trash,
  Loader2,
} from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

// ============================================================================
// Priority Colors
// ============================================================================

const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-base)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

const DEFAULT_PRIORITY_COLOR = { bg: "var(--bg-hover)", text: "var(--text-secondary)" };

// ============================================================================
// Status Badge Configuration
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
}

export function TaskDetailOverlay({ projectId }: TaskDetailOverlayProps) {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);

  // Try to get task from store first, fall back to fetching from API
  const taskFromStore = useTaskStore((state) =>
    selectedTaskId ? state.tasks[selectedTaskId] : undefined
  );

  // Fetch all tasks to ensure we have the latest data
  const { data: tasks = [] } = useTasks(projectId);

  // Find the task from fetched tasks if not in store
  const task: Task | undefined = taskFromStore || tasks.find((t) => t.id === selectedTaskId);

  const [isEditing, setIsEditing] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  // Get mutations
  const {
    updateMutation,
    moveMutation,
    archiveMutation,
    restoreMutation,
    permanentlyDeleteMutation,
    isArchiving,
    isRestoring,
    isPermanentlyDeleting,
  } = useTaskMutation(projectId);

  // Close overlay on Escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSelectedTaskId(null);
        setIsEditing(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setSelectedTaskId]);

  // Reset editing state when task changes
  useEffect(() => {
    setIsEditing(false);
  }, [selectedTaskId]);

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
  const handleArchive = () => {
    if (!task) return;
    archiveMutation.mutate(task.id, {
      onSuccess: () => {
        handleClose();
      },
    });
  };

  // Handle restore
  const handleRestore = () => {
    if (!task) return;
    restoreMutation.mutate(task.id, {
      onSuccess: () => {
        handleClose();
      },
    });
  };

  // Handle permanent delete
  const handlePermanentDelete = () => {
    if (!task) return;
    permanentlyDeleteMutation.mutate(task.id, {
      onSuccess: () => {
        setShowDeleteDialog(false);
        handleClose();
      },
    });
  };

  // Don't render if no task is selected
  if (!selectedTaskId || !task) {
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

  return (
    <>
      {/* Backdrop with blur */}
      <div
        data-testid="task-overlay-backdrop"
        className="absolute inset-0 z-40"
        style={{
          backgroundColor: "rgba(0, 0, 0, 0.6)",
          backdropFilter: "blur(8px)",
          WebkitBackdropFilter: "blur(8px)",
        }}
        onClick={handleBackdropClick}
      >
        {/* Overlay content */}
        <div
          data-testid="task-detail-overlay"
          data-task-id={task.id}
          className="absolute inset-6 flex flex-col rounded-xl overflow-hidden"
          style={{
            background: "linear-gradient(180deg, rgba(24,24,24,0.98) 0%, rgba(18,18,18,0.99) 100%)",
            border: "1px solid rgba(255,255,255,0.08)",
            boxShadow:
              "0 8px 16px rgba(0,0,0,0.4), 0 16px 32px rgba(0,0,0,0.3), 0 0 0 1px rgba(255,255,255,0.03)",
          }}
          onClick={(e) => e.stopPropagation()} // Prevent backdrop click
        >
          {/* Header - Glass effect */}
          <div
            className="px-5 pt-5 pb-4 shrink-0 backdrop-blur-sm"
            style={{
              borderBottom: "1px solid rgba(255,255,255,0.06)",
              background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, transparent 100%)",
            }}
          >
            {/* Archived Badge */}
            {isArchived && (
              <div
                data-testid="archived-badge"
                className="mb-3 px-2.5 py-1.5 rounded-lg flex items-center gap-2 w-fit"
                style={{
                  background: "linear-gradient(135deg, rgba(255,107,53,0.1) 0%, rgba(255,107,53,0.05) 100%)",
                  border: "1px solid rgba(255,107,53,0.2)",
                }}
              >
                <Archive className="w-3.5 h-3.5 text-[#ff6b35]" />
                <span className="text-[12px] font-medium text-[#ff6b35]">Archived</span>
              </div>
            )}
            <div className="flex items-start gap-2.5 pr-28">
              <PriorityBadge priority={task.priority} />
              <div className="flex-1 min-w-0">
                <h2
                  data-testid="task-overlay-title"
                  className="text-base font-semibold truncate text-white/90"
                  style={{
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
                      backgroundColor: "rgba(255,255,255,0.05)",
                      border: "1px solid rgba(255,255,255,0.08)",
                      color: "rgba(255,255,255,0.6)",
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
              {/* Edit button */}
              {canEdit && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => setIsEditing(!isEditing)}
                  data-testid="task-overlay-edit-button"
                  aria-label={isEditing ? "Cancel editing" : "Edit task"}
                  className="hover:bg-white/5"
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
                  className="hover:bg-white/5"
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
                  className="hover:bg-white/5"
                >
                  {isRestoring ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <RotateCcw className="w-4 h-4" />
                  )}
                </Button>
              )}
              {/* Delete permanently button */}
              {isArchived && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => setShowDeleteDialog(true)}
                  disabled={isPermanentlyDeleting}
                  data-testid="task-overlay-delete-button"
                  aria-label="Delete permanently"
                  className="text-red-400 hover:text-red-300 hover:bg-red-400/10"
                >
                  {isPermanentlyDeleting ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Trash className="w-4 h-4" />
                  )}
                </Button>
              )}
              {/* Close button */}
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleClose}
                data-testid="task-overlay-close"
                aria-label="Close"
                className="hover:bg-white/5"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          </div>

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
                <TaskDetailPanel
                  task={task}
                  showHeader={false}
                  showContext={true}
                  showHistory={true}
                  useViewRegistry={true}
                />
              </div>
            </ScrollArea>
          )}
        </div>
      </div>

      {/* Permanent Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Permanently?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete "{task.title}". This action cannot be
              undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handlePermanentDelete}
              disabled={isPermanentlyDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isPermanentlyDeleting ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Deleting...
                </>
              ) : (
                "Delete Permanently"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
