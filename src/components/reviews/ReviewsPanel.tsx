/**
 * ReviewsPanel - Lists pending reviews with filter tabs and integrated diff viewer
 *
 * Features:
 * - Filter tabs: All, AI Review, Human Review
 * - Shows empty state when no pending reviews
 * - Integrated DiffViewer for viewing task changes when "View Diff" is clicked
 * - Changes/History tabs in the diff view
 * - Loading states during diff computation
 */

import { useState, useMemo, useCallback } from "react";
import { usePendingReviews } from "@/hooks/useReviews";
import { useGitDiff } from "@/hooks/useGitDiff";
import { ReviewCard } from "./ReviewCard";
import { DiffViewer } from "@/components/diff";
import type { ReviewerType } from "@/types/review";
import type { ReviewResponse } from "@/lib/tauri";
import type { Commit } from "@/components/diff";

type FilterTab = "all" | "ai" | "human";
type ViewMode = "list" | "detail";

interface ReviewsPanelProps {
  projectId: string;
  taskTitles: Record<string, string>;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string) => void;
  onViewDiff?: (reviewId: string) => void;
  onOpenInIDE?: (filePath: string) => void;
  onClose?: () => void;
}

const TABS: { key: FilterTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "ai", label: "AI Review" },
  { key: "human", label: "Human Review" },
];

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M12 4L4 12M4 4L12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function BackIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M10 4L6 8L10 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function LoadingSpinner() {
  return (
    <div data-testid="reviews-panel-loading" className="flex items-center justify-center p-8">
      <div className="w-6 h-6 border-2 rounded-full animate-spin" style={{ borderColor: "var(--border-subtle)", borderTopColor: "var(--accent-primary)" }} />
    </div>
  );
}

function EmptyState() {
  return (
    <div data-testid="reviews-panel-empty" className="flex flex-col items-center justify-center p-8 text-center">
      <svg width="48" height="48" viewBox="0 0 48 48" fill="none" className="mb-4" style={{ color: "var(--text-tertiary)" }}>
        <circle cx="24" cy="24" r="20" stroke="currentColor" strokeWidth="2" strokeDasharray="4 4" />
        <path d="M16 24L21 29L32 18" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      <p style={{ color: "var(--text-secondary)" }}>No pending reviews</p>
    </div>
  );
}

function FilterTabs({ active, onChange }: { active: FilterTab; onChange: (tab: FilterTab) => void }) {
  const tabStyles = (isActive: boolean) => ({
    backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
    color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
    borderColor: isActive ? "var(--border-subtle)" : "transparent",
  });

  return (
    <div className="flex gap-1 p-1 rounded-lg" style={{ backgroundColor: "var(--bg-base)" }}>
      {TABS.map(({ key, label }) => (
        <button
          key={key}
          role="tab"
          data-active={active === key ? "true" : "false"}
          onClick={() => onChange(key)}
          className="px-3 py-1.5 text-sm font-medium rounded-md transition-colors border"
          style={tabStyles(active === key)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

/**
 * Review Detail Header - Shows when viewing a specific review's diff
 */
interface ReviewDetailHeaderProps {
  review: ReviewResponse;
  taskTitle: string;
  onBack: () => void;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string) => void;
}

function ReviewDetailHeader({
  review,
  taskTitle,
  onBack,
  onApprove,
  onRequestChanges,
}: ReviewDetailHeaderProps) {
  const isPending = review.status === "pending";
  const btnBase = "px-3 py-1.5 rounded text-sm font-medium transition-colors";

  return (
    <div
      className="flex items-center justify-between px-4 py-3 border-b shrink-0"
      style={{ borderColor: "var(--border-subtle)" }}
    >
      <div className="flex items-center gap-3 min-w-0">
        <button
          data-testid="review-detail-back"
          onClick={onBack}
          className="p-1.5 rounded hover:bg-white/5 transition-colors"
          style={{ color: "var(--text-secondary)" }}
          title="Back to reviews"
        >
          <BackIcon />
        </button>
        <div className="min-w-0">
          <h2
            data-testid="review-detail-title"
            className="font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {taskTitle}
          </h2>
          <p
            className="text-xs truncate"
            style={{ color: "var(--text-muted)" }}
          >
            {review.reviewer_type === "ai" ? "AI Review" : "Human Review"} •{" "}
            {review.status}
          </p>
        </div>
      </div>
      {isPending && (
        <div className="flex gap-2 shrink-0">
          {onRequestChanges && (
            <button
              data-testid="review-detail-request-changes"
              onClick={() => onRequestChanges(review.id)}
              className={btnBase}
              style={{
                backgroundColor: "var(--status-warning)",
                color: "var(--bg-base)",
              }}
            >
              Request Changes
            </button>
          )}
          {onApprove && (
            <button
              data-testid="review-detail-approve"
              onClick={() => onApprove(review.id)}
              className={btnBase}
              style={{
                backgroundColor: "var(--status-success)",
                color: "var(--bg-base)",
              }}
            >
              Approve
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * Review Detail View - Shows DiffViewer for a selected review
 */
interface ReviewDetailViewProps {
  review: ReviewResponse;
  taskTitle: string;
  onBack: () => void;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string) => void;
  onOpenInIDE?: (filePath: string) => void;
}

function ReviewDetailView({
  review,
  taskTitle,
  onBack,
  onApprove,
  onRequestChanges,
  onOpenInIDE,
}: ReviewDetailViewProps) {
  const {
    changes,
    commits,
    isLoadingChanges,
    isLoadingHistory,
    fetchDiff,
  } = useGitDiff({
    taskId: review.task_id,
    enabled: true,
  });

  const handleCommitSelect = useCallback((_commit: Commit) => {
    // In a real implementation, this would fetch files changed in the commit
    // For now, the DiffViewer handles this internally via onFetchDiff
  }, []);

  return (
    <div
      data-testid="review-detail-view"
      className="flex flex-col h-full"
    >
      <ReviewDetailHeader
        review={review}
        taskTitle={taskTitle}
        onBack={onBack}
        {...(onApprove ? { onApprove } : {})}
        {...(onRequestChanges ? { onRequestChanges } : {})}
      />
      <div className="flex-1 min-h-0">
        <DiffViewer
          changes={changes}
          commits={commits}
          onFetchDiff={fetchDiff}
          {...(onOpenInIDE ? { onOpenInIDE } : {})}
          isLoadingChanges={isLoadingChanges}
          isLoadingHistory={isLoadingHistory}
          defaultTab="changes"
          onCommitSelect={handleCommitSelect}
        />
      </div>
    </div>
  );
}

export function ReviewsPanel({
  projectId,
  taskTitles,
  onApprove,
  onRequestChanges,
  onViewDiff,
  onOpenInIDE,
  onClose,
}: ReviewsPanelProps) {
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [selectedReview, setSelectedReview] = useState<ReviewResponse | null>(null);
  const { data: reviews, isLoading } = usePendingReviews(projectId);

  const filteredReviews = useMemo(() => {
    if (activeTab === "all") return reviews;
    const reviewerType: ReviewerType = activeTab;
    return reviews.filter((r) => r.reviewer_type === reviewerType);
  }, [reviews, activeTab]);

  const isEmpty = filteredReviews.length === 0;

  // Handle view diff click - switch to detail view
  const handleViewDiff = useCallback(
    (reviewId: string) => {
      const review = reviews.find((r) => r.id === reviewId);
      if (review) {
        setSelectedReview(review);
        setViewMode("detail");
      }
      // Also call external handler if provided
      onViewDiff?.(reviewId);
    },
    [reviews, onViewDiff]
  );

  // Handle back to list view
  const handleBack = useCallback(() => {
    setViewMode("list");
    setSelectedReview(null);
  }, []);

  // Render detail view when a review is selected
  if (viewMode === "detail" && selectedReview) {
    return (
      <div
        data-testid="reviews-panel"
        className="flex flex-col h-full rounded-lg border"
        style={{
          backgroundColor: "var(--bg-surface)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <ReviewDetailView
          review={selectedReview}
          taskTitle={taskTitles[selectedReview.task_id] ?? "Unknown Task"}
          onBack={handleBack}
          {...(onApprove ? { onApprove } : {})}
          {...(onRequestChanges ? { onRequestChanges } : {})}
          {...(onOpenInIDE ? { onOpenInIDE } : {})}
        />
      </div>
    );
  }

  // Render list view
  return (
    <div
      data-testid="reviews-panel"
      className="flex flex-col h-full rounded-lg border"
      style={{
        backgroundColor: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <h2
          data-testid="reviews-panel-title"
          className="text-lg font-semibold"
          style={{ color: "var(--text-primary)" }}
        >
          Reviews
        </h2>
        {onClose && (
          <button
            data-testid="reviews-panel-close"
            onClick={onClose}
            className="p-1 rounded hover:bg-black/10"
            style={{ color: "var(--text-secondary)" }}
          >
            <CloseIcon />
          </button>
        )}
      </div>

      {/* Filter Tabs */}
      <div
        className="px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <FilterTabs active={activeTab} onChange={setActiveTab} />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {isLoading ? (
          <LoadingSpinner />
        ) : isEmpty ? (
          <EmptyState />
        ) : (
          <div className="p-4 space-y-3">
            {filteredReviews.map((review) => (
              <ReviewCard
                key={review.id}
                review={{
                  id: review.id,
                  projectId: review.project_id,
                  taskId: review.task_id,
                  reviewerType: review.reviewer_type,
                  status: review.status,
                  notes: review.notes ?? null,
                  createdAt: review.created_at,
                  completedAt: review.completed_at ?? null,
                }}
                taskTitle={taskTitles[review.task_id] ?? "Unknown Task"}
                {...(onApprove && { onApprove })}
                {...(onRequestChanges && { onRequestChanges })}
                onViewDiff={handleViewDiff}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
