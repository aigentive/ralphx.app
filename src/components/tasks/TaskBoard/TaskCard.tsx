/**
 * TaskCard - Draggable task card for the kanban board
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass background with backdrop-blur
 * - Priority stripe on left border
 * - Subtle shadows, no heavy gradients
 * - Clean hover/drag states
 */

import { useDraggable } from "@dnd-kit/core";
import { GripVertical, FileText, Lightbulb, Archive, Eye, AlertCircle, Clock } from "lucide-react";
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
import { useTaskExecutionState, formatDuration } from "@/hooks/useTaskExecutionState";
import { StepProgressBar } from "@/components/tasks/StepProgressBar";

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
}

/**
 * Get priority gradient for the left border stripe (Refined Studio aesthetic)
 */
function getPriorityColor(priority: number, isArchived: boolean): string {
  // Archived tasks always use gray
  if (isArchived) {
    return "#525252"; // neutral-600
  }

  switch (priority) {
    case 1: // Critical
      return "#ef4444"; // red-500
    case 2: // High
      return "#f97316"; // orange-500
    case 3: // Medium
      return "#ff6b35"; // accent-primary
    case 4: // Low
      return "#525252"; // neutral-600
    default: // None or unknown
      return "transparent";
  }
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
}: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform, isDragging: isBeingDragged } = useDraggable({ id: task.id });

  // UI Store - use selectedTaskId for split layout (TaskDetailOverlay handles rendering)
  const setSelectedTaskId = useUiStore((state) => state.setSelectedTaskId);

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
  const isDraggable = useMemo(() => {
    const nonDraggableStatuses = [
      'executing',
      'qa_refining',
      'qa_testing',
      'qa_passed',
      'qa_failed',
      'pending_review',
      'revision_needed',
      'reviewing',
      'review_passed',
      're_executing',
    ];
    return !nonDraggableStatuses.includes(task.internalStatus);
  }, [task.internalStatus]);

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

  // Card styles based on state (macOS Tahoe - Liquid Glass)
  const getCardStyles = (): React.CSSProperties => {
    const baseStyles: React.CSSProperties = {
      cursor: isDragging ? "grabbing" : (isDraggable ? "grab" : "default"),
      transition: "all 180ms ease-out",
      background: "rgba(255,255,255,0.04)",
      backdropFilter: "blur(20px)",
      WebkitBackdropFilter: "blur(20px)",
      border: "1px solid rgba(255,255,255,0.08)",
      // Priority stripe - must come AFTER border shorthand to override left border
      borderLeft: `3px solid ${getPriorityColor(task.priority, isArchived)}`,
      boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
    };

    if (isDragging) {
      return {
        ...baseStyles,
        transform: "scale(1.02)",
        boxShadow: "0 12px 32px rgba(0,0,0,0.25)",
        background: "rgba(255,255,255,0.06)",
        zIndex: 50,
      };
    }

    if (isSelected) {
      return {
        ...baseStyles,
        background: "rgba(255,107,53,0.08)",
        borderColor: "rgba(255,107,53,0.25)",
        boxShadow: "0 0 0 1px rgba(255,107,53,0.15), 0 2px 8px rgba(0,0,0,0.15)",
      };
    }

    return baseStyles;
  };

  // Execution state class names
  const getExecutionStateClass = (): string => {
    if (task.internalStatus === "executing") {
      return "task-card-executing";
    }
    if (task.internalStatus === "revision_needed") {
      return "task-card-attention";
    }
    // For QA and review states, we'll add custom borders
    return "";
  };

  // Get execution state border styles
  const getExecutionBorderStyles = (): React.CSSProperties => {
    // QA states: pulsing orange border
    if (task.internalStatus.startsWith("qa_")) {
      return {
        borderWidth: "2px",
        borderColor: "var(--accent-primary)",
        animation: "var(--animation-executing-pulse)",
      };
    }
    // Pending review: static amber border
    if (task.internalStatus === "pending_review") {
      return {
        borderWidth: "2px",
        borderColor: "var(--status-warning)",
      };
    }
    return {};
  };

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
      >
        <div
          ref={setNodeRef}
          {...(isDraggable ? { ...attributes, ...listeners } : {})}
          data-testid={`task-card-${task.id}`}
          onClick={() => {
            handleViewDetails();
          }}
          className={`group relative p-2.5 rounded-lg hover:-translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#ff6b35]/50 ${isArchived ? "opacity-50" : ""} ${!isDraggable ? "opacity-70 cursor-default" : ""} ${getExecutionStateClass()}`}
          style={{ ...getCardStyles(), ...getExecutionBorderStyles(), ...dragStyle }}
          title={!isDraggable ? "This task is being processed and cannot be moved manually" : undefined}
          tabIndex={0}
        >
      {/* Archive badge overlay - only shown for archived tasks */}
      {isArchived && (
        <div className="absolute top-1.5 right-1.5 bg-white/10 rounded-full p-0.5" data-testid="archive-badge">
          <Archive className="w-2.5 h-2.5 text-white/50" />
        </div>
      )}

      {/* Activity indicator - shown for executing tasks */}
      {!isArchived && executionState.isActive && (
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
          {/* Phase-specific icon */}
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
          {executionState.phase === "review" && (
            <Eye className="w-3 h-3" style={{ color: "var(--status-warning)" }} />
          )}
          {task.internalStatus === "revision_needed" && (
            <AlertCircle className="w-3 h-3" style={{ color: "var(--status-warning)" }} />
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
