/**
 * ReviewsPanel - Slide-in panel for tasks awaiting review
 *
 * Displays tasks grouped by review phase:
 * - AI tab: Tasks in pending_review, reviewing (AI is reviewing)
 * - Human tab: Tasks in review_passed, escalated (awaiting human decision)
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass headers with backdrop-blur
 * - Flat translucent surfaces
 * - Subtle borders and single shadows
 */

import { useState, useMemo, useCallback, useEffect } from "react";
import { Bot, User, Eye, Loader2 } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useTasksAwaitingReview } from "@/hooks/useReviews";
import { ReviewDetailModal } from "./ReviewDetailModal";
import {
  LoadingSpinner,
  EmptyState,
  PanelHeader,
  FilterTabs,
  type FilterTab,
} from "./ReviewsPanel.utils";
import type { Task } from "@/types/task";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// TaskReviewCard - Card component for displaying tasks in review panel
// ============================================================================

/** Map task status to display label */
function getStatusLabel(status: InternalStatus): string {
  const labels: Partial<Record<InternalStatus, string>> = {
    pending_review: "Pending Review",
    reviewing: "Reviewing",
    review_passed: "Review Passed",
    escalated: "Escalated",
  };
  return labels[status] ?? status.replace(/_/g, " ");
}

/** Map task status to badge variant styling */
function getStatusBadgeClass(status: InternalStatus): string {
  switch (status) {
    case "pending_review":
      return "bg-[var(--status-warning)]/20 text-[var(--status-warning)] border-[var(--status-warning)]/30";
    case "reviewing":
      return "bg-blue-500/20 text-blue-400 border-blue-500/30";
    case "review_passed":
      return "bg-[var(--status-success)]/20 text-[var(--status-success)] border-[var(--status-success)]/30";
    case "escalated":
      return "bg-[var(--status-error)]/20 text-[var(--status-error)] border-[var(--status-error)]/30";
    default:
      return "bg-[var(--bg-hover)] text-[var(--text-secondary)] border-[var(--border-subtle)]";
  }
}

/** Check if task is in AI review phase */
function isAiReviewPhase(status: InternalStatus): boolean {
  return status === "pending_review" || status === "reviewing";
}

interface TaskReviewCardProps {
  task: Task;
  onReview?: (taskId: string) => void;
  isLoading?: boolean;
}

function TaskReviewCard({ task, onReview, isLoading = false }: TaskReviewCardProps) {
  const [isHovered, setIsHovered] = useState(false);
  const isAiPhase = isAiReviewPhase(task.internalStatus);

  return (
    <Card
      data-testid={`task-review-card-${task.id}`}
      data-status={task.internalStatus}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={cn(
        "p-4 border transition-all duration-150 ease-out",
        "bg-[var(--bg-elevated)] border-[var(--border-subtle)]",
        "rounded-[var(--radius-md)]",
        isHovered && "shadow-[var(--shadow-xs)]",
        isHovered && "-translate-y-[1px]",
        isHovered && "border-white/10"
      )}
    >
      {/* Task Title */}
      <div
        data-testid="task-review-title"
        className="font-semibold text-sm truncate text-[var(--text-primary)] leading-tight"
      >
        {task.title}
      </div>

      {/* Status Row */}
      <div className="flex flex-wrap items-center gap-2 mt-2">
        <Badge
          variant="outline"
          className={cn(
            "text-xs font-medium border",
            getStatusBadgeClass(task.internalStatus)
          )}
        >
          {getStatusLabel(task.internalStatus)}
        </Badge>
        <span
          data-testid="review-type-indicator"
          className="inline-flex items-center gap-1 text-xs text-[var(--text-secondary)]"
        >
          {isAiPhase ? (
            <>
              <Bot className="w-4 h-4" />
              AI Review
            </>
          ) : (
            <>
              <User className="w-4 h-4" />
              Human Review
            </>
          )}
        </span>
      </div>

      {/* Description Preview */}
      {task.description && (
        <div className="mt-3">
          <div
            className={cn(
              "p-2 rounded-[var(--radius-sm)]",
              "bg-[var(--bg-base)]"
            )}
          >
            <p
              data-testid="task-review-description"
              className="text-sm text-[var(--text-secondary)] italic line-clamp-2 leading-normal"
            >
              {task.description}
            </p>
          </div>
        </div>
      )}

      {/* Action Buttons */}
      {onReview && (
        <div className="flex flex-wrap gap-2 mt-4">
          <Button
            data-testid={`review-button-${task.id}`}
            variant="ghost"
            size="sm"
            onClick={() => onReview(task.id)}
            disabled={isLoading}
            className="bg-[var(--accent-muted)] hover:bg-[var(--accent-primary)] hover:text-white text-[var(--accent-primary)]"
          >
            {isLoading ? (
              <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />
            ) : (
              <Eye className="w-4 h-4 mr-1.5" />
            )}
            Review
          </Button>
        </div>
      )}
    </Card>
  );
}

// ============================================================================
// ReviewsPanel Component
// ============================================================================

interface ReviewsPanelProps {
  projectId: string;
  /** @deprecated No longer needed - tasks include their own title */
  taskTitles?: Record<string, string>;
  /** @deprecated Use task-based approval in detail modal */
  onApprove?: (taskId: string) => void;
  /** @deprecated Use task-based request changes in detail modal */
  onRequestChanges?: (taskId: string, notes?: string) => void;
  onViewDiff?: (taskId: string) => void;
  onOpenInIDE?: (filePath: string) => void;
  onClose?: () => void;
  isClosing?: boolean;
  /** Whether an approve operation is in progress */
  isApproving?: boolean;
  /** Whether a request changes operation is in progress */
  isRequestingChanges?: boolean;
}

export function ReviewsPanel({
  projectId,
  onClose,
  isApproving = false,
  isRequestingChanges = false,
}: ReviewsPanelProps) {
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  // State for ReviewDetailModal
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const { allTasks, aiTasks, humanTasks, isLoading, aiCount, humanCount, totalCount } =
    useTasksAwaitingReview(projectId);

  // Expose helper for visual tests (web mode only)
  useEffect(() => {
    if (window.__TAURI_INTERNALS__) {
      return;
    }
    window.__openReviewDetailModal = (taskId: string) => {
      setSelectedTaskId(taskId);
    };
    return () => {
      delete window.__openReviewDetailModal;
    };
  }, []);

  // Keyboard navigation - Escape to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && onClose) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Filter tasks by tab
  const filteredTasks = useMemo(() => {
    switch (activeTab) {
      case "ai":
        return aiTasks;
      case "human":
        return humanTasks;
      default:
        return allTasks;
    }
  }, [activeTab, allTasks, aiTasks, humanTasks]);

  const isEmpty = filteredTasks.length === 0;

  // Handle opening ReviewDetailModal
  const handleReview = useCallback((taskId: string) => {
    setSelectedTaskId(taskId);
  }, []);

  // Handle closing ReviewDetailModal
  const handleCloseModal = useCallback(() => {
    setSelectedTaskId(null);
  }, []);

  // Render list view
  return (
    <>
      <div
        data-testid="reviews-panel"
        role="complementary"
        aria-label="Reviews panel"
        className="flex flex-col h-full"
      >
        {/* Header */}
        <PanelHeader totalCount={totalCount} onClose={onClose} />

        {/* Filter Tabs */}
        <FilterTabs
          activeTab={activeTab}
          onTabChange={setActiveTab}
          allCount={totalCount}
          aiCount={aiCount}
          humanCount={humanCount}
        />

        {/* Content */}
        <ScrollArea className="flex-1">
          {isLoading ? (
            <LoadingSpinner />
          ) : isEmpty ? (
            <EmptyState />
          ) : (
            <div className="p-4 space-y-3">
              {filteredTasks.map((task) => (
                <TaskReviewCard
                  key={task.id}
                  task={task}
                  onReview={handleReview}
                  isLoading={isApproving || isRequestingChanges}
                />
              ))}
            </div>
          )}
        </ScrollArea>
      </div>

      {/* ReviewDetailModal - opens when 'Review' button is clicked */}
      {selectedTaskId && (
        <ReviewDetailModal
          taskId={selectedTaskId}
          onClose={handleCloseModal}
        />
      )}
    </>
  );
}
