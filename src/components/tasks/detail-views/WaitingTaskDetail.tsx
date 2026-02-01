/**
 * WaitingTaskDetail - macOS Tahoe-inspired pending review view
 *
 * Shows task waiting for AI reviewer with completion summary
 * and clean step list presentation.
 */

import { Clock, CheckCircle2, Loader2, Hourglass } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { StepList } from "../StepList";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import { reviewIssuesApi } from "@/api/review-issues";
import { IssueProgressBar } from "@/components/reviews/IssueList";
import type { Task } from "@/types/task";

interface WaitingTaskDetailProps {
  task: Task;
}

function formatRelativeTime(date: Date | string | undefined): string {
  if (!date) return "Just now";

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
 * WorkSummaryCard - Shows completion statistics
 */
function WorkSummaryCard({
  submittedAt,
  stepsCompleted,
  totalSteps,
  isLoading,
}: {
  submittedAt: Date | string | undefined;
  stepsCompleted: number;
  totalSteps: number;
  isLoading: boolean;
}) {
  const allComplete = stepsCompleted === totalSteps && totalSteps > 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-6">
        <Loader2
          className="w-5 h-5 animate-spin"
          style={{ color: "rgba(255,255,255,0.3)" }}
        />
      </div>
    );
  }

  return (
    <DetailCard>
      <div className="space-y-3">
        {/* Submitted time */}
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
            style={{ backgroundColor: "rgba(142, 142, 147, 0.15)" }}
          >
            <Clock className="w-4 h-4" style={{ color: "#8e8e93" }} />
          </div>
          <div>
            <span className="text-[11px] uppercase tracking-wider text-white/40 block">
              Submitted
            </span>
            <span className="text-[13px] text-white/70 font-medium">
              {formatRelativeTime(submittedAt)}
            </span>
          </div>
        </div>

        {/* Divider */}
        <div
          className="h-px"
          style={{ backgroundColor: "rgba(255,255,255,0.06)" }}
        />

        {/* Steps status */}
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
            style={{
              backgroundColor: allComplete
                ? "rgba(52, 199, 89, 0.15)"
                : "rgba(142, 142, 147, 0.15)",
            }}
          >
            <CheckCircle2
              className="w-4 h-4"
              style={{ color: allComplete ? "#34c759" : "#8e8e93" }}
            />
          </div>
          <div>
            <span className="text-[11px] uppercase tracking-wider text-white/40 block">
              Steps
            </span>
            <span
              className="text-[13px] font-medium"
              style={{ color: allComplete ? "#30d158" : "rgba(255,255,255,0.7)" }}
            >
              {totalSteps > 0
                ? allComplete
                  ? "All completed"
                  : `${stepsCompleted} of ${totalSteps} completed`
                : "No steps defined"}
            </span>
          </div>
        </div>
      </div>
    </DetailCard>
  );
}

export function WaitingTaskDetail({ task }: WaitingTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: progress } = useStepProgress(task.id);
  const { data: issueProgress } = useQuery({
    queryKey: ["issue-progress", task.id],
    queryFn: () => reviewIssuesApi.getProgress(task.id),
  });

  const hasSteps = (steps?.length ?? 0) > 0;
  const stepsCompleted = progress?.completed ?? 0;
  const totalSteps = progress?.total ?? 0;
  const hasIssueProgress = issueProgress && issueProgress.total > 0;

  return (
    <TwoColumnLayout
      description={task.description}
      testId="waiting-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={Hourglass}
        title="Awaiting AI Review"
        subtitle="Work complete — waiting for automated review"
        variant="neutral"
        badge={
          <StatusPill
            icon={Clock}
            label="Pending"
            variant="neutral"
            size="md"
          />
        }
      />

      {/* Work Summary */}
      <section data-testid="work-completed-wrapper">
        <SectionTitle>Work Summary</SectionTitle>
        <WorkSummaryCard
          submittedAt={task.updatedAt}
          stepsCompleted={stepsCompleted}
          totalSteps={totalSteps}
          isLoading={stepsLoading}
        />
      </section>

      {/* Issue Resolution Progress */}
      {hasIssueProgress && (
        <section data-testid="issue-progress-section">
          <SectionTitle>Issue Resolution</SectionTitle>
          <DetailCard>
            <IssueProgressBar progress={issueProgress} showSeverityBreakdown />
          </DetailCard>
        </section>
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="waiting-steps-loading"
          className="flex items-center justify-center py-8"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      )}

      {!stepsLoading && hasSteps && (
        <section data-testid="waiting-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
