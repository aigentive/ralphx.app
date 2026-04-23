import { useState } from "react";
import { Code, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ReviewDetailModal } from "@/components/reviews/ReviewDetailModal";
import { useGitDiff } from "@/hooks/useGitDiff";
import type { StateTransition } from "@/api/tasks";
import type { ReviewNoteResponse } from "@/lib/tauri";
import { DetailCard } from "./DetailCard";
import { ReviewTimeline } from "./ReviewTimeline";
import { SectionTitle } from "./SectionTitle";

interface CommitSummaryCardProps {
  taskId: string;
}

interface ChangeReviewSectionProps {
  taskId: string;
  history: ReviewNoteResponse[] | undefined;
  stateTransitions: StateTransition[];
  context?: "task" | "plan_merge";
}

export function CommitSummaryCard({ taskId }: CommitSummaryCardProps) {
  const { commits, isLoadingHistory } = useGitDiff({ taskId });

  if (isLoadingHistory) {
    return (
      <div className="flex items-center justify-center py-4">
        <Loader2 className="w-5 h-5 animate-spin text-text-primary/30" />
      </div>
    );
  }

  if (commits.length === 0) {
    return (
      <p className="text-[13px] text-text-primary/50 italic">
        No commit history available
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {commits.map((commit) => (
        <div
          key={commit.sha}
          className="flex items-start gap-3 py-2"
        >
          <span className="text-[11px] font-mono text-text-primary/50 shrink-0 pt-0.5">
            {commit.shortSha}
          </span>
          <span className="text-[13px] text-text-primary/70 line-clamp-2">
            {commit.message}
          </span>
        </div>
      ))}
    </div>
  );
}

export function ChangeReviewSection({
  taskId,
  history,
  stateTransitions,
  context = "task",
}: ChangeReviewSectionProps) {
  const [showReviewModal, setShowReviewModal] = useState(false);
  const isPlanMerge = context === "plan_merge";
  const hasLocalReviewHistory = (history?.length ?? 0) > 0 || stateTransitions.length > 0;

  return (
    <>
      <section data-testid="commits-section">
        <SectionTitle>Commits</SectionTitle>
        <DetailCard>
          <CommitSummaryCard taskId={taskId} />
        </DetailCard>
      </section>

      <section data-testid="review-history-section">
        <div className="flex items-center justify-between mb-3">
          <SectionTitle>{isPlanMerge ? "Code Review" : "Review History"}</SectionTitle>
          <Button
            data-testid="review-code-button"
            onClick={() => setShowReviewModal(true)}
            variant="ghost"
            className="h-8 px-3 gap-2 rounded-lg font-medium text-[12px]"
            style={{ color: "var(--status-info)" }}
          >
            <Code className="w-4 h-4" />
            {isPlanMerge ? "Review Diff" : "Review Code"}
          </Button>
        </div>
        <DetailCard>
          {isPlanMerge && !hasLocalReviewHistory ? (
            <p className="text-[13px] text-text-primary/50 italic">
              Feature branch changes are available in the merged diff
            </p>
          ) : (
            <ReviewTimeline history={history ?? []} stateTransitions={stateTransitions} />
          )}
        </DetailCard>
      </section>

      {showReviewModal && (
        <ReviewDetailModal
          taskId={taskId}
          reviewMode={context}
          showActions={false}
          onClose={() => setShowReviewModal(false)}
          {...(history !== undefined && { history })}
        />
      )}
    </>
  );
}
