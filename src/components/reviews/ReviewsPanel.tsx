/**
 * ReviewsPanel - Slide-in panel for pending reviews
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass headers with backdrop-blur
 * - Flat translucent surfaces
 * - Subtle borders and single shadows
 */

import { useState, useMemo, useCallback, useEffect } from "react";
import {
  X,
  ChevronLeft,
  Bot,
  User,
  Loader2,
  CheckCircle2,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
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

// Panel slide animation styles
const PANEL_STYLES = `
@keyframes panel-slide-in {
  from {
    transform: translateX(100%);
    opacity: 0.8;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes panel-slide-out {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(100%);
    opacity: 0.8;
  }
}
`;

/**
 * Loading Spinner - accent colored with animation
 */
function LoadingSpinner() {
  return (
    <div
      data-testid="reviews-panel-loading"
      className="flex items-center justify-center p-12"
    >
      <Loader2
        className="w-6 h-6 animate-spin"
        style={{ color: "var(--accent-primary)" }}
      />
    </div>
  );
}

/**
 * Empty State - dashed circle with check and message
 */
function EmptyState() {
  return (
    <div
      data-testid="reviews-panel-empty"
      className="flex flex-col items-center justify-center p-12 text-center"
    >
      <CheckCircle2
        className="w-12 h-12 mb-3 opacity-50"
        style={{ color: "var(--text-muted)" }}
        strokeDasharray="4 4"
      />
      <p className="text-sm font-medium text-[var(--text-secondary)]">
        No pending reviews
      </p>
      <p className="text-xs text-[var(--text-muted)] mt-1">
        All reviews have been handled
      </p>
    </div>
  );
}

/**
 * Count Badge - pill-shaped badge showing count
 */
function CountBadge({ count }: { count: number }) {
  return (
    <Badge
      variant="outline"
      className={cn(
        "inline-flex items-center justify-center min-w-[24px] px-2 py-0.5",
        "text-xs font-medium rounded-full border-0",
        "bg-[var(--accent-muted)] text-[var(--accent-primary)]"
      )}
    >
      {count}
    </Badge>
  );
}

/**
 * Panel Header - title, count badge, close button
 */
interface PanelHeaderProps {
  totalCount: number;
  onClose?: (() => void) | undefined;
}

function PanelHeader({ totalCount, onClose }: PanelHeaderProps) {
  return (
    <div
      className="flex items-center justify-between px-4 py-3 border-b shrink-0"
      style={{
        borderColor: "rgba(255,255,255,0.06)",
        height: "52px",
        background: "rgba(18,18,18,0.85)",
        backdropFilter: "blur(20px)",
        WebkitBackdropFilter: "blur(20px)",
      }}
    >
      <h2
        data-testid="reviews-panel-title"
        className="text-lg font-semibold text-[var(--text-primary)]"
        style={{ letterSpacing: "-0.02em" }}
      >
        Reviews
      </h2>
      <div className="flex items-center gap-3">
        <CountBadge count={totalCount} />
        {onClose && (
          <Button
            data-testid="reviews-panel-close"
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close reviews panel"
            className={cn(
              "w-8 h-8 rounded-[var(--radius-md)]",
              "text-[var(--text-muted)] hover:text-[var(--text-primary)]",
              "hover:bg-[var(--bg-hover)]",
              "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)]",
              "transition-colors duration-150"
            )}
          >
            <X className="w-4 h-4" />
          </Button>
        )}
      </div>
    </div>
  );
}

/**
 * Filter Tabs - All, AI, Human with counts
 */
interface FilterTabsProps {
  activeTab: FilterTab;
  onTabChange: (tab: FilterTab) => void;
  allCount: number;
  aiCount: number;
  humanCount: number;
}

function FilterTabs({
  activeTab,
  onTabChange,
  allCount,
  aiCount,
  humanCount,
}: FilterTabsProps) {
  return (
    <div
      className="px-4 py-3 border-b"
      style={{
        borderColor: "rgba(255,255,255,0.06)",
        background: "rgba(255,255,255,0.02)",
      }}
    >
      <Tabs
        value={activeTab}
        onValueChange={(v) => onTabChange(v as FilterTab)}
      >
        <TabsList
          className={cn(
            "inline-flex h-auto p-1 gap-1",
            "bg-transparent rounded-[var(--radius-md)]"
          )}
        >
          <TabsTrigger
            value="all"
            className={cn(
              "px-3 py-1.5 text-sm font-medium rounded-[var(--radius-md)]",
              "data-[state=inactive]:bg-transparent data-[state=inactive]:text-[var(--text-secondary)]",
              "data-[state=active]:bg-[var(--bg-elevated)] data-[state=active]:text-[var(--text-primary)]",
              "data-[state=active]:border data-[state=active]:border-[var(--border-subtle)]",
              "hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]",
              "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)]",
              "transition-all duration-150 min-w-[64px]"
            )}
          >
            All{" "}
            <span className="ml-1 opacity-80 text-xs">({allCount})</span>
          </TabsTrigger>
          <TabsTrigger
            value="ai"
            className={cn(
              "px-3 py-1.5 text-sm font-medium rounded-[var(--radius-md)]",
              "data-[state=inactive]:bg-transparent data-[state=inactive]:text-[var(--text-secondary)]",
              "data-[state=active]:bg-[var(--bg-elevated)] data-[state=active]:text-[var(--text-primary)]",
              "data-[state=active]:border data-[state=active]:border-[var(--border-subtle)]",
              "hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]",
              "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)]",
              "transition-all duration-150 min-w-[64px]",
              "inline-flex items-center gap-1"
            )}
          >
            <Bot className="w-3.5 h-3.5" />
            AI <span className="opacity-80 text-xs">({aiCount})</span>
          </TabsTrigger>
          <TabsTrigger
            value="human"
            className={cn(
              "px-3 py-1.5 text-sm font-medium rounded-[var(--radius-md)]",
              "data-[state=inactive]:bg-transparent data-[state=inactive]:text-[var(--text-secondary)]",
              "data-[state=active]:bg-[var(--bg-elevated)] data-[state=active]:text-[var(--text-primary)]",
              "data-[state=active]:border data-[state=active]:border-[var(--border-subtle)]",
              "hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]",
              "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)]",
              "transition-all duration-150 min-w-[64px]",
              "inline-flex items-center gap-1"
            )}
          >
            <User className="w-3.5 h-3.5" />
            Human <span className="opacity-80 text-xs">({humanCount})</span>
          </TabsTrigger>
        </TabsList>
      </Tabs>
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
  isLoading?: boolean;
}

function ReviewDetailHeader({
  review,
  taskTitle,
  onBack,
  onApprove,
  onRequestChanges,
  isLoading = false,
}: ReviewDetailHeaderProps) {
  const isPending = review.status === "pending";

  return (
    <div
      className="flex items-center justify-between px-4 py-3 border-b shrink-0"
      style={{ borderColor: "var(--border-subtle)" }}
    >
      <div className="flex items-center gap-3 min-w-0">
        <Button
          data-testid="review-detail-back"
          variant="ghost"
          size="icon"
          onClick={onBack}
          className="w-8 h-8 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          title="Back to reviews"
        >
          <ChevronLeft className="w-4 h-4" />
        </Button>
        <div className="min-w-0">
          <h2
            data-testid="review-detail-title"
            className="font-semibold truncate text-[var(--text-primary)]"
          >
            {taskTitle}
          </h2>
          <p className="text-xs truncate text-[var(--text-muted)]">
            {review.reviewer_type === "ai" ? "AI Review" : "Human Review"} •{" "}
            {review.status}
          </p>
        </div>
      </div>
      {isPending && (
        <div className="flex gap-2 shrink-0">
          {onRequestChanges && (
            <Button
              data-testid="review-detail-request-changes"
              size="sm"
              onClick={() => onRequestChanges(review.id)}
              disabled={isLoading}
              className="bg-[var(--status-warning)] text-[var(--bg-base)] hover:opacity-90"
            >
              {isLoading ? <Loader2 className="w-4 h-4 mr-1.5 animate-spin" /> : null}
              Request Changes
            </Button>
          )}
          {onApprove && (
            <Button
              data-testid="review-detail-approve"
              size="sm"
              onClick={() => onApprove(review.id)}
              disabled={isLoading}
              className="bg-[var(--status-success)] text-white hover:opacity-90"
            >
              {isLoading ? <Loader2 className="w-4 h-4 mr-1.5 animate-spin" /> : null}
              Approve
            </Button>
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
  isLoading?: boolean;
}

function ReviewDetailView({
  review,
  taskTitle,
  onBack,
  onApprove,
  onRequestChanges,
  onOpenInIDE,
  isLoading = false,
}: ReviewDetailViewProps) {
  const { changes, commits, isLoadingChanges, isLoadingHistory, fetchDiff } =
    useGitDiff({
      taskId: review.task_id,
      enabled: true,
    });

  const handleCommitSelect = useCallback((_commit: Commit) => {
    // In a real implementation, this would fetch files changed in the commit
  }, []);

  return (
    <div data-testid="review-detail-view" className="flex flex-col h-full">
      <ReviewDetailHeader
        review={review}
        taskTitle={taskTitle}
        onBack={onBack}
        {...(onApprove ? { onApprove } : {})}
        {...(onRequestChanges ? { onRequestChanges } : {})}
        isLoading={isLoading}
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
  isClosing = false,
  isApproving = false,
  isRequestingChanges = false,
}: ReviewsPanelProps) {
  const [activeTab, setActiveTab] = useState<FilterTab>("all");
  const [viewMode, setViewMode] = useState<ViewMode>("list");
  const [selectedReview, setSelectedReview] = useState<ReviewResponse | null>(
    null
  );
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
                  onViewDiff={handleViewDiff}
                  isLoading={isApproving || isRequestingChanges}
                />
              ))}
            </div>
          )}
        </ScrollArea>
      </div>
    </>
  );
}
