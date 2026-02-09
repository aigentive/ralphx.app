/**
 * TaskCard - Draggable task card for the kanban board
 *
 * Design: macOS Tahoe (2025)
 * - Clean, flat surfaces - no gradients or glows
 * - Priority stripe on left edge
 * - Subtle selection highlight (blue tint like Finder)
 * - Minimal visual noise
 *
 * Styling utilities extracted to TaskCard.utils.ts
 */

import { useDraggable } from "@dnd-kit/core";
import { GripVertical, FileText, Lightbulb, Clock, Ban, GitBranch } from "lucide-react";
import { useState, useMemo, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import type { Task } from "@/types/task";
import { StatusBadge, type ReviewStatus } from "@/components/ui/StatusBadge";
import { TaskQABadge } from "@/components/qa/TaskQABadge";
import { Badge } from "@/components/ui/badge";
import type { QAPrepStatus } from "@/types/qa-config";
import type { QAOverallStatus } from "@/types/qa";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
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
import { TaskCardContextMenu } from "@/components/tasks/TaskCardContextMenu";
import type { GroupInfo } from "@/lib/task-actions";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { toast } from "sonner";
import { useTaskExecutionState, formatDuration } from "@/hooks/useTaskExecutionState";
import { StepProgressBar } from "@/components/tasks/StepProgressBar";
import { TaskStatusBadge } from "./TaskStatusBadge";
import { getCardStyles, isDraggableStatus } from "./TaskCard.utils";

interface TaskCardProps {
  task: Task;
  isDragging?: boolean;
  isSelected?: boolean;
  isHidden?: boolean;
  reviewStatus?: ReviewStatus;
  /** Whether this task needs QA */
  needsQA?: boolean;
  /** QA prep status */
  prepStatus?: QAPrepStatus;
  /** Overall QA test status */
  testStatus?: QAOverallStatus;
  hasCheckpoint?: boolean;
  /** Number of revision attempts (for re_executing state badge) */
  revisionCount?: number;
  /** Group context for showing group actions in task context menu */
  groupInfo?: GroupInfo;
}

function CheckpointIndicator() {
  return (
    <Badge
      data-testid="checkpoint-indicator"
      className="text-[9px] px-1.5 py-px"
      style={{
        backgroundColor: "rgba(255, 169, 77, 0.2)",
        color: "#ffa94d",
        border: "none",
      }}
    >
      Checkpoint
    </Badge>
  );
}

export function TaskCard({
  task,
  isDragging,
  isSelected,
  isHidden,
  reviewStatus,
  needsQA,
  prepStatus,
  testStatus,
  hasCheckpoint,
  revisionCount,
  groupInfo,
}: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({ id: task.id });

  // UI Store - use selectedTaskId for split layout (TaskDetailOverlay handles rendering)
  const setSelectedTaskId = useUiStore((state) => state.setSelectedTaskId);
  const setCurrentView = useUiStore((state) => state.setCurrentView);

  // Ideation Store and mutation
  const addSession = useIdeationStore((state) => state.addSession);
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);
  const createSession = useCreateIdeationSession();

  // Execution state
  const executionState = useTaskExecutionState(task.id);

  // Mutations
  const {
    archiveMutation,
    restoreMutation,
    cleanupTaskMutation,
    moveMutation,
    blockMutation,
    unblockMutation,
  } = useTaskMutation(task.projectId);

  // Confirmation dialog state for permanent delete
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Check if task is archived
  const isArchived = task.archivedAt !== null;

  // Determine if task is draggable based on internal status
  const isDraggable = useMemo(() => isDraggableStatus(task.internalStatus), [task.internalStatus]);

  // Drag styling:
  // - isHidden: briefly hide when moving to a different column (prevents duplicate)
  // - Otherwise: always fully visible (DragOverlay handles the floating preview)
  const dragStyle: React.CSSProperties = {
    ...(transform && { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }),
    opacity: isHidden ? 0 : 1,
    transition: 'opacity 150ms ease-out',
  };

  // Build QA badge props conditionally to satisfy exactOptionalPropertyTypes
  const qaBadgeProps = {
    needsQA: needsQA ?? false,
    ...(prepStatus !== undefined && { prepStatus }),
    ...(testStatus !== undefined && { testStatus }),
  };

  // Computed styles using extracted utilities - left border colored by status
  const cardStyles = useMemo(
    () => getCardStyles(task.internalStatus, isArchived, !!isDragging, isDraggable, !!isSelected),
    [task.internalStatus, isArchived, isDragging, isDraggable, isSelected]
  );

  // Context menu handlers - use selectedTaskId for split layout overlay
  const handleViewDetails = () => {
    // Set selectedTaskId to show TaskDetailOverlay in the split layout
    setSelectedTaskId(task.id);
  };

  const handleEdit = () => {
    // Open task detail (edit mode can be triggered from overlay)
    setSelectedTaskId(task.id);
  };

  const handleArchive = () => {
    archiveMutation.mutate(task.id);
  };

  const handleRestore = () => {
    restoreMutation.mutate(task.id);
  };

  const handlePermanentDelete = () => {
    setShowDeleteConfirm(true);
  };

  const confirmPermanentDelete = () => {
    cleanupTaskMutation.mutate(task.id);
    setShowDeleteConfirm(false);
  };

  const handleStatusChange = (newStatus: string) => {
    moveMutation.mutate({ taskId: task.id, toStatus: newStatus });
  };

  const handleBlockWithReason = useCallback(
    (reason?: string) => {
      blockMutation.mutate(reason ? { taskId: task.id, reason } : { taskId: task.id });
    },
    [task.id, blockMutation]
  );

  const handleUnblock = useCallback(() => {
    unblockMutation.mutate(task.id);
  }, [task.id, unblockMutation]);

  const handleStartIdeation = async () => {
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
      // Navigate to ideation view
      setCurrentView("ideation");
    } catch (error) {
      console.error("Failed to start ideation:", error);
      toast.error("Failed to start ideation session");
    }
  };

  return (
    <>
      <TaskCardContextMenu
        task={task}
        onViewDetails={handleViewDetails}
        onEdit={handleEdit}
        onArchive={handleArchive}
        onRestore={handleRestore}
        onPermanentDelete={handlePermanentDelete}
        onStatusChange={handleStatusChange}
        onBlockWithReason={handleBlockWithReason}
        onUnblock={handleUnblock}
        onStartIdeation={handleStartIdeation}
        {...(groupInfo !== undefined && { groupInfo })}
      >
        <div
          ref={setNodeRef}
          {...(isDraggable ? { ...attributes, ...listeners } : {})}
          data-testid={`task-card-${task.id}`}
          onClick={() => {
            handleViewDetails();
          }}
          className={`group relative p-2.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 ${isArchived ? "opacity-50" : ""} ${!isDraggable ? "opacity-70 cursor-default" : ""}`}
          style={{ ...cardStyles, ...dragStyle }}
          title={!isDraggable ? "This task is being processed and cannot be moved manually" : undefined}
          tabIndex={0}
        >
      {/* Status badge - shown for ALL task states */}
      <div className="absolute top-1.5 right-1.5" data-testid="status-badge-container">
        <TaskStatusBadge
          status={task.internalStatus}
          isArchived={isArchived}
          {...(revisionCount !== undefined && { revisionCount })}
        />
      </div>

      {/* Drag handle - appears on hover over the status badge (hidden if not draggable) */}
      {isDraggable && !isArchived && (
        <div
          data-testid="drag-handle"
          className="absolute top-1.5 right-1.5 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab"
        >
          <GripVertical className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 40%)" }} />
        </div>
      )}

      {/* Card content */}
      <div className="pr-7">
        {/* Title - clean, simple */}
        <div
          data-testid="task-title"
          className="truncate"
          style={{
            fontSize: "13px",
            fontWeight: 500,
            color: "hsl(220 10% 90%)",
            lineHeight: 1.4,
          }}
        >
          {task.title}
        </div>

        {/* Description - 2 line clamp with markdown */}
        {task.description && (
          <div
            className="mt-1 line-clamp-2 [&_*]:!mb-0 [&_*]:!mt-0"
            style={{
              fontSize: "12px",
              color: "hsl(220 10% 55%)",
              lineHeight: 1.45,
            }}
          >
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
              {task.description}
            </ReactMarkdown>
          </div>
        )}

        {/* Blocked reason indicator - yellow pill style */}
        {task.internalStatus === "blocked" && task.blockedReason && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  data-testid="blocked-reason-indicator"
                  className="flex items-center gap-1.5 mt-1.5 px-2 py-0.5 rounded-full text-xs max-w-full"
                  style={{
                    backgroundColor: "hsla(45, 90%, 55%, 0.15)",
                    color: "hsl(45 90% 55%)",
                  }}
                >
                  <Ban className="w-3 h-3 flex-shrink-0" />
                  <span className="truncate">{task.blockedReason}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="max-w-xs">
                <p className="text-sm">{task.blockedReason}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}

        {/* Badge row - simple, muted */}
        <div className="flex flex-wrap items-center gap-1 mt-1.5">
          <span
            style={{
              fontSize: "10px",
              fontWeight: 500,
              color: "hsl(220 10% 45%)",
              textTransform: "capitalize",
            }}
          >
            {task.category}
          </span>
          {reviewStatus && <StatusBadge type="review" status={reviewStatus} />}
          <TaskQABadge {...qaBadgeProps} />
          {hasCheckpoint && <CheckpointIndicator />}

          {/* Artifact indicators */}
          {task.planArtifactId && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div
                    data-testid="plan-artifact-indicator"
                    className="inline-flex items-center justify-center"
                  >
                    <FileText
                      className="w-3.5 h-3.5"
                      style={{ color: "var(--accent-primary)" }}
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>Has implementation plan</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}

          {task.sourceProposalId && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div
                    data-testid="source-proposal-indicator"
                    className="inline-flex items-center justify-center"
                  >
                    <Lightbulb
                      className="w-3.5 h-3.5"
                      style={{ color: "var(--accent-secondary)" }}
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>Created from proposal</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}

          {/* Branch indicator - shown for tasks with active branch */}
          {task.taskBranch && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div
                    data-testid="branch-indicator"
                    className="inline-flex items-center gap-1"
                    style={{ color: "hsl(220 10% 50%)" }}
                  >
                    <GitBranch className="w-3 h-3 flex-shrink-0" />
                    <span
                      className="text-[10px] truncate max-w-[80px]"
                      style={{
                        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
                      }}
                    >
                      {task.taskBranch.split("/").pop()}
                    </span>
                  </div>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="max-w-xs">
                  <p className="text-xs font-mono">{task.taskBranch}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>

        {/* Step progress indicator - shown for all post-execution statuses */}
        {(task.internalStatus === "executing" ||
          task.internalStatus === "re_executing" ||
          task.internalStatus.startsWith("qa_") ||
          task.internalStatus === "pending_review" ||
          task.internalStatus === "reviewing" ||
          task.internalStatus === "review_passed" ||
          task.internalStatus === "escalated" ||
          task.internalStatus === "revision_needed" ||
          task.internalStatus === "approved" ||
          task.internalStatus === "merged") && (
          <div className="flex items-center gap-2 mt-2" data-testid="step-progress-footer">
            <StepProgressBar taskId={task.id} compact={true} />

            {/* Duration badge - shown when executing */}
            {(task.internalStatus === "executing" || task.internalStatus === "re_executing") && executionState.duration !== null && (
              <div className="flex items-center gap-1 text-xs text-white/50">
                <Clock className="w-3 h-3" />
                <span>{formatDuration(executionState.duration)}</span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
      </TaskCardContextMenu>

      {/* Confirmation dialog for permanent delete */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete task permanently?</AlertDialogTitle>
            <AlertDialogDescription>
              This action cannot be undone. The task "{task.title}" will be permanently removed from the system.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmPermanentDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete Permanently
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
