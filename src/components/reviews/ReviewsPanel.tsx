/**
 * ReviewsPanel - Lists pending reviews with filter tabs
 * Shows empty state when no pending reviews
 * Filter tabs: All, AI Review, Human Review
 */

import { useState, useMemo } from "react";
import { usePendingReviews } from "@/hooks/useReviews";
import { ReviewCard } from "./ReviewCard";
import type { ReviewerType } from "@/types/review";

type FilterTab = "all" | "ai" | "human";

interface ReviewsPanelProps {
  projectId: string;
  taskTitles: Record<string, string>;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string) => void;
  onViewDiff?: (reviewId: string) => void;
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

export function ReviewsPanel({ projectId, taskTitles, onApprove, onRequestChanges, onViewDiff, onClose }: ReviewsPanelProps) {
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const { data: reviews, isLoading } = usePendingReviews(projectId);

  const filteredReviews = useMemo(() => {
    if (activeTab === "all") return reviews;
    const reviewerType: ReviewerType = activeTab;
    return reviews.filter((r) => r.reviewer_type === reviewerType);
  }, [reviews, activeTab]);

  const isEmpty = filteredReviews.length === 0;

  return (
    <div data-testid="reviews-panel" className="flex flex-col h-full rounded-lg border" style={{ backgroundColor: "var(--bg-surface)", borderColor: "var(--border-subtle)" }}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b" style={{ borderColor: "var(--border-subtle)" }}>
        <h2 data-testid="reviews-panel-title" className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
          Reviews
        </h2>
        {onClose && (
          <button data-testid="reviews-panel-close" onClick={onClose} className="p-1 rounded hover:bg-black/10" style={{ color: "var(--text-secondary)" }}>
            <CloseIcon />
          </button>
        )}
      </div>

      {/* Filter Tabs */}
      <div className="px-4 py-3 border-b" style={{ borderColor: "var(--border-subtle)" }}>
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
                {...(onViewDiff && { onViewDiff })}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
