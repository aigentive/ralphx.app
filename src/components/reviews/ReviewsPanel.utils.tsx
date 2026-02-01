/**
 * ReviewsPanel utility components
 *
 * Extracted from ReviewsPanel.tsx to reduce component size.
 * Contains presentational sub-components used by ReviewsPanel.
 */

import { useCallback } from "react";
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
import { cn } from "@/lib/utils";
import { useGitDiff } from "@/hooks/useGitDiff";
import { DiffViewer } from "@/components/diff";
import { useTaskStore } from "@/stores/taskStore";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import type { ReviewResponse } from "@/lib/tauri";
import type { Commit } from "@/components/diff";

export type FilterTab = "all" | "ai" | "human";

// Panel slide animation styles
export const PANEL_STYLES = `
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
export function LoadingSpinner() {
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
export function EmptyState() {
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
export function CountBadge({ count }: { count: number }) {
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

export function PanelHeader({ totalCount, onClose }: PanelHeaderProps) {
  return (
    <div
      className="flex items-center justify-between h-11 px-3 border-b shrink-0"
      style={{
        borderColor: "hsla(220 10% 100% / 0.04)",
        background: "hsla(220 10% 100% / 0.02)",
      }}
    >
      <div className="flex items-center gap-2">
        <CheckCircle2 className="w-4 h-4" style={{ color: "hsl(220 10% 50%)" }} />
        <h2
          data-testid="reviews-panel-title"
          className="text-[13px] font-semibold"
          style={{ color: "hsl(220 10% 90%)", letterSpacing: "-0.01em" }}
        >
          Reviews
        </h2>
      </div>
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

export function FilterTabs({
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
        borderColor: "hsla(220 10% 100% / 0.04)",
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

export function ReviewDetailHeader({
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

export function ReviewDetailView({
  review,
  taskTitle,
  onBack,
  onApprove,
  onRequestChanges,
  onOpenInIDE,
  isLoading = false,
}: ReviewDetailViewProps) {
  // Get project path from task's project
  const tasks = useTaskStore((s) => s.tasks);
  const task = tasks[review.task_id];
  const projects = useProjectStore((s) => s.projects);
  const activeProject = useProjectStore(selectActiveProject);

  // Try to get project from task, fall back to active project
  const project = task?.projectId ? projects[task.projectId] : activeProject;
  const projectPath = project?.workingDirectory;

  const { changes, commits, isLoadingChanges, isLoadingHistory, fetchDiff } =
    useGitDiff({
      taskId: review.task_id,
      projectPath,
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
