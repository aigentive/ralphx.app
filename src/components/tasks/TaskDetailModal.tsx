/**
 * TaskDetailModal - Premium modal dialog for task details
 *
 * Design spec: specs/design/refined-studio-patterns.md
 * - Refined Studio aesthetic with layered depth
 * - Glass effect header with backdrop-blur
 * - Gradient backgrounds and premium shadows
 * - Compact sizing for application UI
 */

import {
  Dialog,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { useReviewsByTaskId, useTaskStateHistory } from "@/hooks/useReviews";
import { StateHistoryTimeline } from "./StateHistoryTimeline";
import { TaskContextPanel } from "./TaskContextPanel";
import { TaskEditForm } from "./TaskEditForm";
import { StatusDropdown } from "./StatusDropdown";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import type { Task } from "@/types/task";
import { X, Loader2, FileText, Pencil, Archive, RotateCcw, Trash } from "lucide-react";
import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import {
  PriorityBadge,
  StatusBadge,
  ReviewCard,
  FixTaskIndicator,
  SectionTitle,
} from "./TaskDetailModal.components";
import { SYSTEM_CONTROLLED_STATUSES } from "./TaskDetailModal.constants";
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

interface TaskDetailModalProps {
  task: Task | null;
  isOpen: boolean;
  onClose: () => void;
  fixTaskCount?: number;
}

export function TaskDetailModal({
  task,
  isOpen,
  onClose,
  fixTaskCount,
}: TaskDetailModalProps) {
  const [showContext, setShowContext] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const { data: reviews, isLoading: reviewsLoading } = useReviewsByTaskId(
    task?.id ?? ""
  );
  useTaskStateHistory(task?.id ?? "");

  // Get mutations - use task's projectId if available
  const {
    updateMutation,
    moveMutation,
    archiveMutation,
    restoreMutation,
    cleanupTaskMutation,
    isArchiving,
    isRestoring,
    isCleaningTask,
  } = useTaskMutation(task?.projectId ?? "");

  if (!task) return null;

  const hasReviews = reviews.length > 0;
  const hasFixTasks = fixTaskCount !== undefined && fixTaskCount > 0;
  const hasContext = !!(task.sourceProposalId || task.planArtifactId);

  // Determine if task is editable
  // System-controlled statuses: executing, qa_*, pending_review, revision_needed, reviewing, review_passed, re_executing
  const isArchived = !!task.archivedAt;
  const isSystemControlled = SYSTEM_CONTROLLED_STATUSES.includes(task.internalStatus);
  const canEdit = !isArchived && !isSystemControlled;

  // Handle edit save
  const handleSave = (updateData: Parameters<typeof updateMutation.mutate>[0]['input']) => {
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
    moveMutation.mutate({ taskId: task.id, toStatus: newStatus });
  };

  // Handle archive
  const handleArchive = () => {
    archiveMutation.mutate(task.id, {
      onSuccess: () => {
        onClose();
      },
    });
  };

  // Handle restore
  const handleRestore = () => {
    restoreMutation.mutate(task.id, {
      onSuccess: () => {
        onClose();
      },
    });
  };

  // Handle permanent delete
  const handlePermanentDelete = () => {
    cleanupTaskMutation.mutate(task.id, {
      onSuccess: () => {
        setShowDeleteDialog(false);
        onClose();
      },
    });
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogPortal>
        {/* Custom overlay with blur */}
        <DialogOverlay
          className="fixed inset-0 z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
          style={{
            backgroundColor: "rgba(0, 0, 0, 0.6)",
            backdropFilter: "blur(8px)",
          }}
        />
        {/* Custom content with scale animation - Refined Studio */}
        <div
          data-testid="task-detail-modal"
          className="fixed left-[50%] top-[50%] z-50 translate-x-[-50%] translate-y-[-50%] w-full max-w-[580px] max-h-[80vh] overflow-hidden flex flex-col rounded-xl data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          style={{
            background: "linear-gradient(180deg, rgba(24,24,24,0.98) 0%, rgba(18,18,18,0.99) 100%)",
            border: "1px solid rgba(255,255,255,0.08)",
            boxShadow:
              "0 8px 16px rgba(0,0,0,0.4), 0 16px 32px rgba(0,0,0,0.3), 0 0 0 1px rgba(255,255,255,0.03)",
          }}
          data-state={isOpen ? "open" : "closed"}
        >
          {/* Header - Glass effect */}
          <div
            className="px-5 pt-5 pb-4 backdrop-blur-sm"
            style={{
              borderBottom: "1px solid rgba(255,255,255,0.06)",
              background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, transparent 100%)",
            }}
          >
            {/* Archived Badge */}
            {isArchived && (
              <div
                data-testid="archived-badge"
                className="mb-3 px-2.5 py-1.5 rounded-lg flex items-center gap-2"
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
                  data-testid="task-detail-title"
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
                    data-testid="task-detail-category"
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
            {/* Action buttons (status dropdown, edit, archive/restore, close) */}
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
              {/* Edit button - only for non-archived, non-system-controlled tasks */}
              {canEdit && (
                <button
                  onClick={() => setIsEditing(!isEditing)}
                  data-testid="task-detail-edit-button"
                  className="p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2"
                  style={{
                    color: "var(--text-muted)",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                    e.currentTarget.style.color = "var(--text-primary)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "transparent";
                    e.currentTarget.style.color = "var(--text-muted)";
                  }}
                  aria-label={isEditing ? "Cancel editing" : "Edit task"}
                  title={isEditing ? "Cancel editing" : "Edit task"}
                >
                  <Pencil className="w-4 h-4" />
                </button>
              )}
              {/* Archive button - only for non-archived tasks */}
              {!isArchived && (
                <button
                  onClick={handleArchive}
                  disabled={isArchiving}
                  data-testid="task-detail-archive-button"
                  className="p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:opacity-50"
                  style={{
                    color: "var(--text-muted)",
                  }}
                  onMouseEnter={(e) => {
                    if (!isArchiving) {
                      e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                      e.currentTarget.style.color = "var(--text-primary)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "transparent";
                    e.currentTarget.style.color = "var(--text-muted)";
                  }}
                  aria-label="Archive task"
                  title="Archive task"
                >
                  {isArchiving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Archive className="w-4 h-4" />
                  )}
                </button>
              )}
              {/* Restore button - only for archived tasks */}
              {isArchived && (
                <button
                  onClick={handleRestore}
                  disabled={isRestoring}
                  data-testid="task-detail-restore-button"
                  className="p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:opacity-50"
                  style={{
                    color: "var(--text-muted)",
                  }}
                  onMouseEnter={(e) => {
                    if (!isRestoring) {
                      e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                      e.currentTarget.style.color = "var(--text-primary)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "transparent";
                    e.currentTarget.style.color = "var(--text-muted)";
                  }}
                  aria-label="Restore task"
                  title="Restore task"
                >
                  {isRestoring ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <RotateCcw className="w-4 h-4" />
                  )}
                </button>
              )}
              {/* Delete permanently button - only for archived tasks */}
              {isArchived && (
                <button
                  onClick={() => setShowDeleteDialog(true)}
                  disabled={isCleaningTask}
                  data-testid="task-detail-delete-permanently-button"
                  className="p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:opacity-50"
                  style={{
                    color: "var(--status-error)",
                  }}
                  onMouseEnter={(e) => {
                    if (!isCleaningTask) {
                      e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "transparent";
                  }}
                  aria-label="Delete permanently"
                  title="Delete permanently"
                >
                  {isCleaningTask ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Trash className="w-4 h-4" />
                  )}
                </button>
              )}
              {/* Close button */}
              <button
                onClick={onClose}
                data-testid="task-detail-close"
                className="p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2"
                style={{
                  color: "var(--text-muted)",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                  e.currentTarget.style.color = "var(--text-primary)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = "transparent";
                  e.currentTarget.style.color = "var(--text-muted)";
                }}
                aria-label="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          </div>

          {/* Scrollable Content */}
          <ScrollArea className="flex-1">
            {isEditing ? (
              /* Edit Mode */
              <div className="px-6 py-4">
                <TaskEditForm
                  task={task}
                  onSave={handleSave}
                  onCancel={() => setIsEditing(false)}
                  isSaving={updateMutation.isPending}
                />
              </div>
            ) : (
              /* Read-only View */
              <div
                data-testid="task-detail-view"
                data-task-id={task.id}
                className="px-6 py-4 space-y-6"
              >
                {/* View Context Button */}
                {hasContext && (
                  <div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setShowContext(!showContext)}
                      data-testid="view-context-button"
                      className="w-full justify-center"
                    >
                      <FileText className="h-4 w-4 mr-2" />
                      {showContext ? "Hide Context" : "View Context"}
                    </Button>
                  </div>
                )}

                {/* Task Context Panel */}
                {showContext && hasContext && (
                  <div data-testid="task-context-section">
                    <TaskContextPanel taskId={task.id} />
                  </div>
                )}

                {/* Description Section */}
                {task.description ? (
                  <div
                    data-testid="task-detail-description"
                    className="text-[13px] text-white/60"
                    style={{
                      lineHeight: "1.6",
                      wordBreak: "break-word",
                    }}
                  >
                    <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                      {task.description}
                    </ReactMarkdown>
                  </div>
                ) : (
                  <p className="text-[13px] italic text-white/35">
                    No description provided
                  </p>
                )}

                {/* Reviews Section */}
                {reviewsLoading && (
                  <div
                    data-testid="reviews-loading"
                    className="flex justify-center py-4"
                  >
                    <Loader2
                      className="w-6 h-6 animate-spin"
                      style={{ color: "var(--text-muted)" }}
                    />
                  </div>
                )}
                {!reviewsLoading && hasReviews && (
                  <div data-testid="task-detail-reviews-section">
                    <SectionTitle>Reviews</SectionTitle>
                    <div className="space-y-2">
                      {reviews.map((review) => (
                        <ReviewCard
                          key={review.id}
                          reviewerType={review.reviewer_type}
                          status={review.status}
                        />
                      ))}
                    </div>
                    {hasFixTasks && <FixTaskIndicator count={fixTaskCount} />}
                  </div>
                )}

                {/* History Section */}
                <div data-testid="task-detail-history-section">
                  <SectionTitle>History</SectionTitle>
                  <StateHistoryTimeline taskId={task.id} />
                </div>
              </div>
            )}
          </ScrollArea>
        </div>
      </DialogPortal>

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
              disabled={isCleaningTask}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isCleaningTask ? (
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
    </Dialog>
  );
}
