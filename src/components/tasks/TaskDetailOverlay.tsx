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
import { StateTimelineNav } from "./StateTimelineNav";
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
  Lightbulb,
  History,
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
import { useIdeationStore } from "@/stores/ideationStore";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useConfirmation } from "@/hooks/useConfirmation";
import { toast } from "sonner";

// ============================================================================
// Priority Colors (Tahoe HSL palette)
// ============================================================================

const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "hsl(0 70% 55%)", text: "white" },
  2: { bg: "hsl(14 100% 60%)", text: "white" },
  3: { bg: "hsl(45 90% 55%)", text: "hsl(220 10% 10%)" },
  4: { bg: "hsl(220 10% 20%)", text: "hsl(220 10% 65%)" },
};

const DEFAULT_PRIORITY_COLOR = { bg: "hsl(220 10% 20%)", text: "hsl(220 10% 65%)" };

// ============================================================================
// Status Badge Configuration (Tahoe HSL palette)
// ============================================================================

const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; bg: string; text: string }
> = {
  backlog: {
    label: "Backlog",
    bg: "hsl(220 10% 20%)",
    text: "hsl(220 10% 50%)",
  },
  ready: {
    label: "Ready",
    bg: "hsla(220 80% 60% / 0.15)",
    text: "hsl(220 80% 65%)",
  },
  blocked: {
    label: "Blocked",
    bg: "hsla(45 90% 55% / 0.15)",
    text: "hsl(45 90% 55%)",
  },
  executing: {
    label: "Executing",
    bg: "hsla(14 100% 60% / 0.15)",
    text: "hsl(14 100% 60%)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "hsla(14 100% 60% / 0.15)",
    text: "hsl(14 100% 60%)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "hsla(14 100% 60% / 0.15)",
    text: "hsl(14 100% 60%)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "hsla(145 60% 45% / 0.15)",
    text: "hsl(145 60% 50%)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "hsla(0 70% 55% / 0.15)",
    text: "hsl(0 70% 60%)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "hsla(45 90% 55% / 0.15)",
    text: "hsl(45 90% 55%)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "hsla(45 90% 55% / 0.15)",
    text: "hsl(45 90% 55%)",
  },
  approved: {
    label: "Approved",
    bg: "hsla(145 60% 45% / 0.15)",
    text: "hsl(145 60% 50%)",
  },
  failed: {
    label: "Failed",
    bg: "hsla(0 70% 55% / 0.15)",
    text: "hsl(0 70% 60%)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "hsl(220 10% 20%)",
    text: "hsl(220 10% 50%)",
  },
  reviewing: {
    label: "AI Review in Progress",
    bg: "hsla(220 80% 60% / 0.15)",
    text: "hsl(220 80% 65%)",
  },
  review_passed: {
    label: "AI Review Passed",
    bg: "hsla(145 60% 45% / 0.15)",
    text: "hsl(145 60% 50%)",
  },
  escalated: {
    label: "Escalated",
    bg: "hsla(45 90% 55% / 0.15)",
    text: "hsl(45 90% 55%)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "hsla(14 100% 60% / 0.15)",
    text: "hsl(14 100% 60%)",
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
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  // History state from store - shared with IntegratedChatPanel
  const historyState = useUiStore((s) => s.taskHistoryState);
  const setHistoryState = useUiStore((s) => s.setTaskHistoryState);

  // Debug logging for history state
  console.log('[TaskDetailOverlay] History state from store:', historyState);

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

  // Find the task from fetched tasks if not in store
  const task: Task | undefined = taskFromStore || tasks.find((t) => t.id === selectedTaskId);

  const [isEditing, setIsEditing] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  // Derived values for history mode (historyState from store)
  const isHistoryMode = historyState !== null;
  const viewStatus = (historyState?.status as InternalStatus | undefined) ?? task?.internalStatus;

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
          backgroundColor: "hsl(220 10% 8%)",
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
              borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            {/* Archived Badge */}
            {isArchived && (
              <div
                data-testid="archived-badge"
                className="mb-3 px-2.5 py-1.5 rounded-lg flex items-center gap-2 w-fit"
                style={{
                  backgroundColor: "hsla(14 100% 60% / 0.1)",
                  border: "1px solid hsla(14 100% 60% / 0.2)",
                }}
              >
                <Archive className="w-3.5 h-3.5" style={{ color: "hsl(14 100% 60%)" }} />
                <span className="text-[12px] font-medium" style={{ color: "hsl(14 100% 60%)" }}>Archived</span>
              </div>
            )}
            <div className="flex items-start gap-2.5 pr-28">
              <PriorityBadge priority={task.priority} />
              <div className="flex-1 min-w-0">
                <h2
                  data-testid="task-overlay-title"
                  className="text-base font-semibold truncate"
                  style={{
                    color: "hsl(220 10% 90%)",
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
                      backgroundColor: "hsla(220 10% 100% / 0.05)",
                      border: "1px solid hsla(220 10% 100% / 0.08)",
                      color: "hsl(220 10% 60%)",
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
                  style={{ color: "hsl(220 10% 50%)" }}
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
                  style={{ color: "hsl(220 10% 50%)" }}
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
                  style={{ color: "hsl(220 10% 50%)" }}
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
                  style={{ color: "hsl(220 10% 50%)" }}
                  className="hover:bg-[hsla(220_10%_100%/0.05)]"
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
                  style={{ color: "hsl(0 70% 60%)" }}
                  className="hover:bg-[hsla(0_70%_55%/0.1)]"
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
                style={{ color: "hsl(220 10% 50%)" }}
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
              <History className="w-3 h-3" style={{ color: "hsl(220 10% 40%)" }} />
              <span className="text-[11px]" style={{ color: "hsl(220 10% 45%)" }}>
                Viewing: {STATUS_CONFIG[historyState.status]?.label ?? historyState.status}
              </span>
              <span className="text-[10px]" style={{ color: "hsl(220 10% 35%)" }}>
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
                <TaskDetailPanel
                  task={task}
                  showHeader={false}
                  showContext={true}
                  showHistory={true}
                  useViewRegistry={true}
                  {...(isHistoryMode && viewStatus ? { viewAsStatus: viewStatus } : {})}
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

      {/* Archive/Restore Confirmation Dialog */}
      <ConfirmationDialog {...confirmationDialogProps} />
    </>
  );
}
