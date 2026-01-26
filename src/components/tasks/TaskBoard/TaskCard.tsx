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
import { GripVertical, FileText, Lightbulb, Archive } from "lucide-react";
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

interface TaskCardProps {
  task: Task;
  onSelect?: (taskId: string) => void;
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
  onSelect,
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

  // UI Store
  const openModal = useUiStore((state) => state.openModal);

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
      'execution_done',
      'qa_refining',
      'qa_testing',
      'qa_passed',
      'qa_failed',
      'pending_review',
      'revision_needed',
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
      borderLeft: `3px solid ${getPriorityColor(task.priority, isArchived)}`,
      cursor: isDragging ? "grabbing" : (isDraggable ? "grab" : "default"),
      transition: "all 180ms ease-out",
      background: "rgba(255,255,255,0.04)",
      backdropFilter: "blur(20px)",
      WebkitBackdropFilter: "blur(20px)",
      border: "1px solid rgba(255,255,255,0.08)",
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

  // Context menu handlers
  const handleViewDetails = () => {
    openModal("task-detail", { taskId: task.id });
  };

  const handleEdit = () => {
    openModal("task-detail", { taskId: task.id, startInEditMode: true });
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
          onClick={() => onSelect?.(task.id)}
          className={`group relative p-2.5 rounded-lg hover:-translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#ff6b35]/50 ${isArchived ? "opacity-50" : ""} ${!isDraggable ? "opacity-70 cursor-default" : ""}`}
          style={{ ...getCardStyles(), ...dragStyle }}
          title={!isDraggable ? "This task is being processed and cannot be moved manually" : undefined}
          tabIndex={0}
        >
      {/* Archive badge overlay - only shown for archived tasks */}
      {isArchived && (
        <div className="absolute top-1.5 right-1.5 bg-white/10 rounded-full p-0.5" data-testid="archive-badge">
          <Archive className="w-2.5 h-2.5 text-white/50" />
        </div>
      )}

      {/* Drag handle - appears on hover (hidden if archived to show archive badge) */}
      {!isArchived && (
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
