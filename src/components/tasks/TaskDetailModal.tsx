/**
 * TaskDetailModal - Premium modal dialog for task details
 * Uses shadcn Dialog with backdrop blur, scale animation, and premium styling
 * Displays task info, reviews, QA results, and state history
 */

import {
  Dialog,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { useReviewsByTaskId, useTaskStateHistory } from "@/hooks/useReviews";
import { StateHistoryTimeline } from "./StateHistoryTimeline";
import type { Task, InternalStatus } from "@/types/task";
import { X, Bot, User, Wrench, Loader2 } from "lucide-react";

interface TaskDetailModalProps {
  task: Task | null;
  isOpen: boolean;
  onClose: () => void;
  fixTaskCount?: number;
}

// Priority colors matching design spec
const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-base)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

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
  execution_done: {
    label: "Execution Done",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
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
};

const DEFAULT_PRIORITY_COLOR = { bg: "var(--bg-hover)", text: "var(--text-secondary)" };

function PriorityBadge({ priority }: { priority: number }) {
  const colors = PRIORITY_COLORS[priority] ?? DEFAULT_PRIORITY_COLOR;
  return (
    <span
      data-testid="task-detail-priority"
      className="inline-flex items-center px-2 py-1 rounded text-xs font-mono font-medium"
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
      data-testid="task-detail-status"
      data-status={status}
      className="rounded-lg px-2 py-1 text-xs font-medium border-0"
      style={{ backgroundColor: config.bg, color: config.text }}
    >
      {config.label}
    </Badge>
  );
}

function ReviewCard({
  reviewerType,
  status,
}: {
  reviewerType: "ai" | "human";
  status: string;
}) {
  const Icon = reviewerType === "ai" ? Bot : User;
  const label = reviewerType === "ai" ? "AI Review" : "Human Review";

  const defaultStatusColor = { bg: "var(--bg-hover)", text: "var(--text-muted)" };
  const statusColors: Record<string, { bg: string; text: string }> = {
    pending: defaultStatusColor,
    approved: {
      bg: "rgba(16, 185, 129, 0.15)",
      text: "var(--status-success)",
    },
    changes_requested: {
      bg: "rgba(245, 158, 11, 0.15)",
      text: "var(--status-warning)",
    },
    rejected: { bg: "rgba(239, 68, 68, 0.15)", text: "var(--status-error)" },
  };

  const statusColor = statusColors[status] ?? defaultStatusColor;

  return (
    <div
      data-testid={`review-item-${reviewerType}`}
      className="flex items-center justify-between p-3 rounded-lg"
      style={{
        backgroundColor: "var(--bg-surface)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      <div className="flex items-center gap-2">
        <Icon className="w-4 h-4" style={{ color: "var(--text-secondary)" }} />
        <span
          className="text-sm font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          {label}
        </span>
      </div>
      <Badge
        className="rounded-lg px-2 py-0.5 text-xs font-medium border-0 capitalize"
        style={{ backgroundColor: statusColor.bg, color: statusColor.text }}
      >
        {status.replace("_", " ")}
      </Badge>
    </div>
  );
}

function FixTaskIndicator({ count }: { count: number }) {
  const label = count === 1 ? "1 fix task" : `${count} fix tasks`;
  return (
    <div
      data-testid="fix-task-indicator"
      className="flex items-center gap-2 text-sm mt-3"
      style={{ color: "var(--status-warning)" }}
    >
      <Wrench className="w-4 h-4" />
      <span>{label}</span>
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3
      className="text-sm font-medium mb-3"
      style={{ color: "var(--text-primary)" }}
    >
      {children}
    </h3>
  );
}

export function TaskDetailModal({
  task,
  isOpen,
  onClose,
  fixTaskCount,
}: TaskDetailModalProps) {
  const { data: reviews, isLoading: reviewsLoading } = useReviewsByTaskId(
    task?.id ?? ""
  );
  useTaskStateHistory(task?.id ?? "");

  if (!task) return null;

  const hasReviews = reviews.length > 0;
  const hasFixTasks = fixTaskCount !== undefined && fixTaskCount > 0;

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
        {/* Custom content with scale animation */}
        <div
          data-testid="task-detail-modal"
          className="fixed left-[50%] top-[50%] z-50 translate-x-[-50%] translate-y-[-50%] w-full max-w-[640px] max-h-[80vh] overflow-hidden flex flex-col rounded-xl data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
          style={{
            backgroundColor: "var(--bg-elevated)",
            border: "1px solid var(--border-subtle)",
            boxShadow:
              "0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25)",
          }}
          data-state={isOpen ? "open" : "closed"}
        >
          {/* Header */}
          <div
            className="px-6 pt-6 pb-4"
            style={{ borderBottom: "1px solid var(--border-subtle)" }}
          >
            <div className="flex items-start gap-3 pr-8">
              <PriorityBadge priority={task.priority} />
              <div className="flex-1 min-w-0">
                <h2
                  data-testid="task-detail-title"
                  className="text-xl font-semibold truncate"
                  style={{
                    color: "var(--text-primary)",
                    letterSpacing: "-0.02em",
                    lineHeight: "1.2",
                  }}
                >
                  {task.title}
                </h2>
                <div className="flex flex-wrap items-center gap-2 mt-2">
                  <span
                    data-testid="task-detail-category"
                    className="px-2.5 py-1 rounded text-xs font-medium"
                    style={{
                      backgroundColor: "var(--bg-base)",
                      border: "1px solid var(--border-subtle)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {task.category}
                  </span>
                  <StatusBadge status={task.internalStatus} />
                </div>
              </div>
            </div>
            {/* Close button */}
            <button
              onClick={onClose}
              data-testid="task-detail-close"
              className="absolute top-4 right-4 p-2 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2"
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

          {/* Scrollable Content */}
          <ScrollArea className="flex-1">
            <div
              data-testid="task-detail-view"
              data-task-id={task.id}
              className="px-6 py-4 space-y-6"
            >
              {/* Description Section */}
              {task.description ? (
                <div>
                  <p
                    data-testid="task-detail-description"
                    className="text-sm"
                    style={{
                      color: "var(--text-secondary)",
                      lineHeight: "1.65",
                      wordBreak: "break-word",
                    }}
                  >
                    {task.description}
                  </p>
                </div>
              ) : (
                <p
                  className="text-sm italic"
                  style={{ color: "var(--text-muted)" }}
                >
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
          </ScrollArea>
        </div>
      </DialogPortal>
    </Dialog>
  );
}
