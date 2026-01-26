/**
 * TaskDetailPanel - Reusable task detail content (extracted from TaskDetailModal)
 *
 * This component extracts the detail content from TaskDetailModal for reuse
 * in other contexts like TaskFullView. It renders task metadata, context,
 * steps, and history without edit buttons (parent handles that).
 *
 * Design spec: specs/design/refined-studio-patterns.md
 * - Refined Studio aesthetic with layered depth
 * - Gradient backgrounds and premium shadows
 */

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useReviewsByTaskId } from "@/hooks/useReviews";
import { StateHistoryTimeline } from "./StateHistoryTimeline";
import { TaskContextPanel } from "./TaskContextPanel";
import { StepList } from "./StepList";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import type { Task, InternalStatus } from "@/types/task";
import { Bot, User, Loader2, FileText } from "lucide-react";
import { useState } from "react";

interface TaskDetailPanelProps {
  task: Task;
  showContext?: boolean;
  showHistory?: boolean;
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
      data-testid="task-detail-status"
      data-status={status}
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0"
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

  const defaultStatusColor = { bg: "rgba(255,255,255,0.05)", text: "rgba(255,255,255,0.5)" };
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
      className="flex items-center justify-between p-2.5 rounded-lg"
      style={{
        background: "linear-gradient(180deg, rgba(28,28,28,0.9) 0%, rgba(22,22,22,0.95) 100%)",
        border: "1px solid rgba(255,255,255,0.06)",
      }}
    >
      <div className="flex items-center gap-2">
        <Icon className="w-3.5 h-3.5 text-white/50" />
        <span className="text-[13px] font-medium text-white/80">
          {label}
        </span>
      </div>
      <Badge
        className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0 capitalize"
        style={{ backgroundColor: statusColor.bg, color: statusColor.text }}
      >
        {status.replace("_", " ")}
      </Badge>
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-[13px] font-medium mb-2.5 text-white/80">
      {children}
    </h3>
  );
}

export function TaskDetailPanel({
  task,
  showContext: showContextProp = false,
  showHistory: showHistoryProp = true,
}: TaskDetailPanelProps) {
  const [showContext, setShowContext] = useState(showContextProp);

  // Fetch reviews
  const { data: reviews, isLoading: reviewsLoading } = useReviewsByTaskId(task.id);

  // Fetch steps to determine if we should show the StepList
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);

  const hasReviews = reviews.length > 0;
  const hasContext = !!(task.sourceProposalId || task.planArtifactId);
  const hasSteps = (steps?.length ?? 0) > 0;

  return (
    <div
      data-testid="task-detail-panel"
      data-task-id={task.id}
      className="space-y-6"
    >
      {/* Header with priority, title, category, status */}
      <div className="space-y-2">
        <div className="flex items-start gap-2.5">
          <PriorityBadge priority={task.priority} />
          <div className="flex-1 min-w-0">
            <h2
              data-testid="task-detail-title"
              className="text-base font-semibold text-white/90"
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
      </div>

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
        <div>
          <p
            data-testid="task-detail-description"
            className="text-[13px] text-white/60"
            style={{
              lineHeight: "1.6",
              wordBreak: "break-word",
            }}
          >
            {task.description}
          </p>
        </div>
      ) : (
        <p className="text-[13px] italic text-white/35">
          No description provided
        </p>
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div className="flex justify-center py-4">
          <Loader2
            className="w-6 h-6 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}
      {!stepsLoading && hasSteps && (
        <div data-testid="task-detail-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </div>
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
        </div>
      )}

      {/* History Section */}
      {showHistoryProp && (
        <div data-testid="task-detail-history-section">
          <SectionTitle>History</SectionTitle>
          <StateHistoryTimeline taskId={task.id} />
        </div>
      )}
    </div>
  );
}
