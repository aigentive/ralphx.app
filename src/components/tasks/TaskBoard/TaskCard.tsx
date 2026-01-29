/**
 * TaskCard - Draggable task card for the kanban board
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass background with backdrop-blur
 * - Priority stripe on left border
 * - Subtle shadows, no heavy gradients
 * - Clean hover/drag states
 *
 * Styling utilities extracted to TaskCard.utils.ts
 */

import { useDraggable } from "@dnd-kit/core";
import { GripVertical, FileText, Lightbulb, Archive, Clock } from "lucide-react";
import { useState, useMemo } from "react";
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
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { toast } from "sonner";
import { useTaskExecutionState, formatDuration } from "@/hooks/useTaskExecutionState";
import { StepProgressBar } from "@/components/tasks/StepProgressBar";
import { ReviewStateBadge } from "./ReviewStateBadge";
import {
  getCardStyles,
  getExecutionStateClass,
  getExecutionBorderStyles,
  isDraggableStatus,
  isReviewStateStatus,
  isActivelyProcessing,
} from "./TaskCard.utils";

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
}: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform, isDragging: isBeingDragged } = useDraggable({ id: task.id });

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
    permanentlyDeleteMutation,
    moveMutation,
  } = useTaskMutation(task.projectId);

  // Confirmation dialog state for permanent delete
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Check if task is archived
  const isArchived = task.archivedAt !== null;

  // Determine if task is draggable based on internal status
  const isDraggable = useMemo(() => isDraggableStatus(task.internalStatus), [task.internalStatus]);

  // Hide when being dragged OR when transitioning after drop
  const shouldHide = isBeingDragged || isHidden;

  const dragStyle: React.CSSProperties = {
    ...(transform && { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }),
    opacity: shouldHide ? 0 : 1,
    // Pop-in animation when card appears in new column
    animation: shouldHide ? 'none' : 'card-pop-in 150ms ease-out',
  };

  // Build QA badge props conditionally to satisfy exactOptionalPropertyTypes
  const qaBadgeProps = {
    needsQA: needsQA ?? false,
    ...(prepStatus !== undefined && { prepStatus }),
    ...(testStatus !== undefined && { testStatus }),
  };

  // Computed styles using extracted utilities
  const cardStyles = useMemo(
    () => getCardStyles(task.priority, isArchived, !!isDragging, isDraggable, !!isSelected),
    [task.priority, isArchived, isDragging, isDraggable, isSelected]
  );
  const executionStateClass = getExecutionStateClass(task.internalStatus);
  const executionBorderStyles = getExecutionBorderStyles(task.internalStatus);
  const showReviewState = isReviewStateStatus(task.internalStatus);
  const isActivelyProcessingTask = isActivelyProcessing(task.internalStatus);

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
    permanentlyDeleteMutation.mutate(task.id);
    setShowDeleteConfirm(false);
  };

  const handleStatusChange = (newStatus: string) => {
    moveMutation.mutate({ taskId: task.id, toStatus: newStatus });
  };

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
        onStartIdeation={handleStartIdeation}
      >
        <div
          ref={setNodeRef}
          {...(isDraggable ? { ...attributes, ...listeners } : {})}
          data-testid={`task-card-${task.id}`}
          onClick={() => {
            handleViewDetails();
          }}
          className={`group relative p-2.5 rounded-lg hover:-translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#ff6b35]/50 ${isArchived ? "opacity-50" : ""} ${!isDraggable ? "opacity-70 cursor-default" : ""} ${executionStateClass}`}
          style={{ ...cardStyles, ...executionBorderStyles, ...dragStyle }}
          title={!isDraggable ? "This task is being processed and cannot be moved manually" : undefined}
          tabIndex={0}
        >
      {/* Archive badge overlay - only shown for archived tasks */}
      {isArchived && (
        <div className="absolute top-1.5 right-1.5 bg-white/10 rounded-full p-0.5" data-testid="archive-badge">
          <Archive className="w-2.5 h-2.5 text-white/50" />
        </div>
      )}

      {/* Review state badges - shown for review-related states */}
      {!isArchived && showReviewState && (
        <div className="absolute top-1.5 right-1.5 flex items-center gap-1" data-testid="review-state-indicator">
          {/* Activity dots for actively processing states */}
          {isActivelyProcessingTask && (
            <div className="flex gap-0.5">
              <span
                className="w-1 h-1 rounded-full"
                style={{
                  backgroundColor: task.internalStatus === "reviewing" ? "var(--status-info)" : "var(--status-warning)",
                  animation: "bounce 1.4s ease-in-out 0s infinite",
                }}
              />
              <span
                className="w-1 h-1 rounded-full"
                style={{
                  backgroundColor: task.internalStatus === "reviewing" ? "var(--status-info)" : "var(--status-warning)",
                  animation: "bounce 1.4s ease-in-out 0.2s infinite",
                }}
              />
              <span
                className="w-1 h-1 rounded-full"
                style={{
                  backgroundColor: task.internalStatus === "reviewing" ? "var(--status-info)" : "var(--status-warning)",
                  animation: "bounce 1.4s ease-in-out 0.4s infinite",
                }}
              />
            </div>
          )}
          <ReviewStateBadge status={task.internalStatus} revisionCount={revisionCount} />
        </div>
      )}

      {/* Activity indicator - shown for executing/QA tasks (not review states) */}
      {!isArchived && executionState.isActive && !showReviewState && (
        <div className="absolute top-1.5 right-1.5 flex items-center gap-1" data-testid="activity-indicator">
          {/* Three dots with staggered bounce animation */}
          <div className="flex gap-0.5">
            <span
              className="w-1 h-1 rounded-full"
              style={{
                backgroundColor: "var(--accent-primary)",
                animation: "bounce 1.4s ease-in-out 0s infinite",
              }}
            />
            <span
              className="w-1 h-1 rounded-full"
              style={{
                backgroundColor: "var(--accent-primary)",
                animation: "bounce 1.4s ease-in-out 0.2s infinite",
              }}
            />
            <span
              className="w-1 h-1 rounded-full"
              style={{
                backgroundColor: "var(--accent-primary)",
                animation: "bounce 1.4s ease-in-out 0.4s infinite",
              }}
            />
          </div>
          {/* Phase-specific badge for QA */}
          {executionState.phase === "qa" && (
            <Badge
              variant="secondary"
              className="text-[9px] px-1 py-0.5"
              style={{
                backgroundColor: "rgba(255, 107, 53, 0.15)",
                color: "var(--accent-primary)",
                border: "none",
              }}
            >
              QA
            </Badge>
          )}
        </div>
      )}

      {/* Drag handle - appears on hover (hidden if archived or executing to show indicator) */}
      {!isArchived && !executionState.isActive && (
        <div
          data-testid="drag-handle"
          className="absolute top-1.5 right-1.5 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab"
        >
          <GripVertical className="w-3.5 h-3.5 text-white/30 hover:text-white/50" />
        </div>
      )}

      {/* Card content */}
      <div className="pr-5">
        {/* Title */}
        <div
          data-testid="task-title"
          className="text-[13px] font-medium truncate text-white/90 tracking-tight leading-snug"
        >
          {task.title}
        </div>

        {/* Description - 2 line clamp */}
        {task.description && (
          <div className="text-xs mt-1 line-clamp-2 text-white/50 leading-relaxed">
            {task.description}
          </div>
        )}

        {/* Badge row */}
        <div className="flex flex-wrap items-center gap-1 mt-1.5">
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0.5 bg-white/5 text-white/60 border-white/10">
            {task.category}
          </Badge>
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
        </div>

        {/* Step progress indicator - shown when task is executing, in QA, or pending review */}
        {(task.internalStatus === "executing" ||
          task.internalStatus.startsWith("qa_") ||
          task.internalStatus === "pending_review") && (
          <div className="flex items-center gap-2 mt-2" data-testid="step-progress-footer">
            <StepProgressBar taskId={task.id} compact={true} />

            {/* Duration badge - shown when executing */}
            {task.internalStatus === "executing" && executionState.duration !== null && (
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
