/**
 * MergedTaskDetail - View for successfully merged tasks
 *
 * Shows completion info, merge commit SHA, and read-only historical chat.
 */

import { useState } from "react";
import {
  CheckCircle2,
  GitMerge,
  GitCommit,
  Loader2,
  Code,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { ReviewTimeline } from "./shared/ReviewTimeline";
import { ValidationProgress } from "./MergingTaskDetail";
import { useTaskStateHistory } from "@/hooks/useReviews";
import { useGitDiff } from "@/hooks/useGitDiff";
import type { Task } from "@/types/task";
import { ReviewDetailModal } from "@/components/reviews/ReviewDetailModal";

interface MergedTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

function formatRelativeTime(date: Date | string | null | undefined): string {
  if (!date) return "Unknown";

  const now = new Date();
  const then = typeof date === "string" ? new Date(date) : date;
  const diffMs = now.getTime() - then.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

/**
 * MergeInfoCard - Shows merge commit and branch details
 */
function MergeInfoCard({
  mergeCommitSha,
  branchName,
  mergedAt,
}: {
  mergeCommitSha: string | null | undefined;
  branchName: string | null | undefined;
  mergedAt: Date | string | null | undefined;
}) {
  const shortSha = mergeCommitSha?.substring(0, 7) ?? "unknown";

  return (
    <DetailCard variant="success">
      <div className="space-y-4">
        {/* Merge commit */}
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
            style={{ backgroundColor: "rgba(52, 199, 89, 0.15)" }}
          >
            <GitCommit className="w-4 h-4" style={{ color: "#34c759" }} />
          </div>
          <div className="flex-1 min-w-0">
            <span className="text-[11px] uppercase tracking-wider text-white/40 block">
              Merge Commit
            </span>
            <span className="text-[13px] text-white/70 font-mono">
              {shortSha}
            </span>
          </div>
          <span className="text-[12px] text-white/40">
            {formatRelativeTime(mergedAt)}
          </span>
        </div>

        {/* Branch info */}
        {branchName && (
          <>
            <div
              className="h-px"
              style={{ backgroundColor: "rgba(255,255,255,0.06)" }}
            />
            <div className="flex items-center gap-3">
              <div
                className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
                style={{ backgroundColor: "rgba(52, 199, 89, 0.15)" }}
              >
                <GitMerge className="w-4 h-4" style={{ color: "#34c759" }} />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-[11px] uppercase tracking-wider text-white/40 block">
                  Branch
                </span>
                <span className="text-[13px] text-white/60 font-mono truncate block">
                  {branchName}
                </span>
              </div>
              <span className="text-[10px] px-2 py-0.5 rounded bg-white/5 text-white/40">
                Deleted
              </span>
            </div>
          </>
        )}
      </div>
    </DetailCard>
  );
}

/**
 * CommitSummaryCard - Shows commits that were merged
 */
function CommitSummaryCard({ taskId }: { taskId: string }) {
  const { commits, isLoadingHistory } = useGitDiff({ taskId });

  if (isLoadingHistory) {
    return (
      <div className="flex items-center justify-center py-4">
        <Loader2
          className="w-5 h-5 animate-spin"
          style={{ color: "rgba(255,255,255,0.3)" }}
        />
      </div>
    );
  }

  if (commits.length === 0) {
    return (
      <p className="text-[13px] text-white/50 italic">
        No commit history available
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {commits.slice(0, 5).map((commit) => (
        <div
          key={commit.sha}
          className="flex items-start gap-3 py-2"
        >
          <span className="text-[11px] font-mono text-white/50 shrink-0 pt-0.5">
            {commit.shortSha}
          </span>
          <span className="text-[13px] text-white/70 line-clamp-2">
            {commit.message}
          </span>
        </div>
      ))}
      {commits.length > 5 && (
        <p className="text-[12px] text-white/40 italic">
          +{commits.length - 5} more commits
        </p>
      )}
    </div>
  );
}

export function MergedTaskDetail({ task, isHistorical: _isHistorical = false }: MergedTaskDetailProps) {
  const { data: history, isLoading } = useTaskStateHistory(task.id);
  const [showReviewModal, setShowReviewModal] = useState(false);

  // Use completedAt as mergedAt (merge happens after approval which sets completedAt)
  const mergedAt = task.completedAt ?? task.updatedAt;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2
          className="w-6 h-6 animate-spin"
          style={{ color: "rgba(255,255,255,0.3)" }}
        />
      </div>
    );
  }

  return (
    <>
      <TwoColumnLayout
        description={task.description}
        testId="merged-task-detail"
      >
      {/* Status Banner */}
      <StatusBanner
        icon={CheckCircle2}
        title="Task Merged"
        subtitle="Changes have been merged into the base branch"
        variant="success"
        badge={
          <StatusPill
            icon={CheckCircle2}
            label="Merged"
            variant="success"
            size="md"
          />
        }
      />

      {/* Merge Info */}
      <section data-testid="merge-info-section">
        <SectionTitle>Merge Details</SectionTitle>
        <MergeInfoCard
          mergeCommitSha={task.mergeCommitSha}
          branchName={task.taskBranch}
          mergedAt={mergedAt}
        />
      </section>

      {/* Commits Summary */}
      <section data-testid="commits-section">
        <SectionTitle>Commits</SectionTitle>
        <DetailCard>
          <CommitSummaryCard taskId={task.id} />
        </DetailCard>
      </section>

      {/* Merge Validation History */}
      <ValidationProgress
        taskId={task.id}
        metadata={task.metadata}
      />

      {/* Review History */}
      <section data-testid="review-history-section">
        <div className="flex items-center justify-between mb-3">
          <SectionTitle>Review History</SectionTitle>
          <Button
            data-testid="review-code-button"
            onClick={() => setShowReviewModal(true)}
            variant="ghost"
            className="h-8 px-3 gap-2 rounded-lg font-medium text-[12px]"
            style={{ color: "hsl(217 90% 60%)" }}
          >
            <Code className="w-4 h-4" />
            Review Code
          </Button>
        </div>
        <DetailCard>
          <ReviewTimeline history={history ?? []} />
        </DetailCard>
      </section>
      </TwoColumnLayout>

      {showReviewModal && (
        <ReviewDetailModal
          taskId={task.id}
          history={history}
          showActions={false}
          onClose={() => setShowReviewModal(false)}
        />
      )}
    </>
  );
}
