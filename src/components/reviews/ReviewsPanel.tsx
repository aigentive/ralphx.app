/**
 * ReviewsPanel - Slide-in panel for pending reviews
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass headers with backdrop-blur
 * - Flat translucent surfaces
 * - Subtle borders and single shadows
 */

import { useState, useMemo, useCallback, useEffect } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { usePendingReviews } from "@/hooks/useReviews";
import { ReviewCard } from "./ReviewCard";
import { ReviewDetailModal } from "./ReviewDetailModal";
import {
  LoadingSpinner,
  EmptyState,
  PanelHeader,
  FilterTabs,
  ReviewDetailView,
  PANEL_STYLES,
  type FilterTab,
} from "./ReviewsPanel.utils";
import type { ReviewerType } from "@/types/review";
import type { ReviewResponse } from "@/lib/tauri";

type ViewMode = "list" | "detail";

interface ReviewsPanelProps {
  projectId: string;
  taskTitles: Record<string, string>;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string, notes?: string) => void;
  onViewDiff?: (reviewId: string) => void;
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
  taskTitles,
  onApprove,
  onRequestChanges,
  onViewDiff,
  onOpenInIDE,
  onClose,
  isClosing = false,
  isApproving = false,
  isRequestingChanges = false,
}: ReviewsPanelProps) {
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [selectedReview, setSelectedReview] = useState<ReviewResponse | null>(
    null
  );
  // State for ReviewDetailModal
  const [selectedReviewId, setSelectedReviewId] = useState<string | null>(null);
  const { data: reviews, isLoading } = usePendingReviews(projectId);

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

  // Filter reviews by tab
  const filteredReviews = useMemo(() => {
    if (activeTab === "all") return reviews;
    const reviewerType: ReviewerType = activeTab;
    return reviews.filter((r) => r.reviewer_type === reviewerType);
  }, [reviews, activeTab]);

  // Calculate counts for tabs
  const allCount = reviews.length;
  const aiCount = reviews.filter((r) => r.reviewer_type === "ai").length;
  const humanCount = reviews.filter((r) => r.reviewer_type === "human").length;

  const isEmpty = filteredReviews.length === 0;

  // Handle view diff click - switch to detail view
  const handleViewDiff = useCallback(
    (reviewId: string) => {
      const review = reviews.find((r) => r.id === reviewId);
      if (review) {
        setSelectedReview(review);
        setViewMode("detail");
      }
      onViewDiff?.(reviewId);
    },
    [reviews, onViewDiff]
  );

  // Handle back to list view
  const handleBack = useCallback(() => {
    setViewMode("list");
    setSelectedReview(null);
  }, []);

  // Handle opening ReviewDetailModal
  const handleReview = useCallback((reviewId: string) => {
    setSelectedReviewId(reviewId);
  }, []);

  // Handle closing ReviewDetailModal
  const handleCloseModal = useCallback(() => {
    setSelectedReviewId(null);
  }, []);

  // Find the task ID for the selected review (for ReviewDetailModal)
  const selectedReviewTaskId = useMemo(() => {
    if (!selectedReviewId) return null;
    const review = reviews.find((r) => r.id === selectedReviewId);
    return review?.task_id ?? null;
  }, [selectedReviewId, reviews]);

  // Panel animation class
  const animationClass = isClosing
    ? "animate-[panel-slide-out_250ms_ease-in_forwards]"
    : "animate-[panel-slide-in_300ms_ease-out]";

  // Render detail view when a review is selected
  if (viewMode === "detail" && selectedReview) {
    return (
      <>
        <style>{PANEL_STYLES}</style>
        <div
          data-testid="reviews-panel"
          role="complementary"
          aria-label="Reviews panel"
          className={cn(
            "flex flex-col h-full",
            "bg-[var(--bg-surface)]",
            "shadow-[var(--shadow-md)]",
            animationClass
          )}
        >
          <ReviewDetailView
            review={selectedReview}
            taskTitle={taskTitles[selectedReview.task_id] ?? "Unknown Task"}
            onBack={handleBack}
            {...(onApprove ? { onApprove } : {})}
            {...(onRequestChanges ? { onRequestChanges: (id: string) => onRequestChanges(id) } : {})}
            {...(onOpenInIDE ? { onOpenInIDE } : {})}
            isLoading={isApproving || isRequestingChanges}
          />
        </div>
      </>
    );
  }

  // Render list view
  return (
    <>
      <style>{PANEL_STYLES}</style>
      <div
        data-testid="reviews-panel"
        role="complementary"
        aria-label="Reviews panel"
        className={cn(
          "flex flex-col h-full",
          "bg-[var(--bg-surface)]",
          "shadow-[var(--shadow-md)]",
          animationClass
        )}
      >
        {/* Header */}
        <PanelHeader totalCount={allCount} onClose={onClose} />

        {/* Filter Tabs */}
        <FilterTabs
          activeTab={activeTab}
          onTabChange={setActiveTab}
          allCount={allCount}
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
                  {...(onRequestChanges && { onRequestChanges: (id: string) => onRequestChanges(id) })}
                  onReview={handleReview}
                  onViewDiff={handleViewDiff}
                  isLoading={isApproving || isRequestingChanges}
                />
              ))}
            </div>
          )}
        </ScrollArea>
      </div>

      {/* ReviewDetailModal - opens when 'Review' button is clicked */}
      {selectedReviewId && selectedReviewTaskId && (
        <ReviewDetailModal
          taskId={selectedReviewTaskId}
          reviewId={selectedReviewId}
          onClose={handleCloseModal}
        />
      )}
    </>
  );
}
