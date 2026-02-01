/**
 * RevisionTaskDetail - macOS Tahoe-inspired revision needed view
 *
 * Shows the revision feedback and attempt count with native styling.
 */

import { StepList } from "../StepList";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";
import {
  Loader2,
  AlertTriangle,
  Bot,
  User,
  RotateCcw,
  FileCode2,
  MessageCircleWarning,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface RevisionTaskDetailProps {
  task: Task;
}

function formatTimeAgo(isoDate: string): string {
  const diff = Date.now() - new Date(isoDate).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "Just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function calculateAttemptNumber(history: ReviewNoteResponse[]): number {
  const changesRequestedCount = history.filter(
    (entry) => entry.outcome === "changes_requested"
  ).length;
  return changesRequestedCount + 1;
}

function getLatestRevisionFeedback(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  const revisionEntries = history.filter(
    (entry) => entry.outcome === "changes_requested"
  );
  if (revisionEntries.length === 0) return null;
  return revisionEntries[0] ?? null;
}

/**
 * Parse issues from notes text
 */
function parseIssuesFromNotes(notes: string | null | undefined): {
  mainText: string;
  items: string[];
} {
  if (!notes) return { mainText: "", items: [] };

  const lines = notes.split("\n");
  const items: string[] = [];
  const mainTextLines: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (/^[-*•]/.test(trimmed) || /^\d+\./.test(trimmed)) {
      const itemText = trimmed.replace(/^[-*•]\s*/, "").replace(/^\d+\.\s*/, "");
      if (itemText) items.push(itemText);
    } else if (trimmed) {
      mainTextLines.push(trimmed);
    }
  }

  return { mainText: mainTextLines.join(" "), items };
}

/**
 * IssueItem - Renders a single issue with optional file:line reference
 */
function IssueItem({ issue }: { issue: string }) {
  const fileLineMatch = issue.match(/([a-zA-Z0-9_/./-]+\.[a-zA-Z]+):(\d+)/);

  if (fileLineMatch) {
    const [fullMatch, file, line] = fileLineMatch;
    const beforeMatch = issue.substring(0, issue.indexOf(fullMatch));
    const afterMatch = issue.substring(issue.indexOf(fullMatch) + fullMatch.length);

    return (
      <li className="flex items-start gap-2.5 py-1.5">
        <FileCode2
          className="w-4 h-4 mt-0.5 shrink-0"
          style={{ color: "#ff9f0a" }}
        />
        <span className="text-[13px] text-white/60 leading-relaxed">
          {beforeMatch}
          <code
            className="px-1.5 py-0.5 rounded-md text-[11px] font-mono mx-1"
            style={{
              backgroundColor: "rgba(255, 107, 53, 0.15)",
              color: "#ff8050",
            }}
          >
            {file}:{line}
          </code>
          {afterMatch}
        </span>
      </li>
    );
  }

  return (
    <li className="flex items-start gap-2.5 py-1.5">
      <div
        className="w-1.5 h-1.5 rounded-full mt-2 shrink-0"
        style={{ backgroundColor: "rgba(255,255,255,0.3)" }}
      />
      <span className="text-[13px] text-white/60 leading-relaxed">{issue}</span>
    </li>
  );
}

/**
 * FeedbackCard - Shows the review feedback that triggered revision
 */
function FeedbackCard({ review }: { review: ReviewNoteResponse }) {
  const isAiReviewer = review.reviewer === "ai";
  const timeAgo = formatTimeAgo(review.created_at);
  const issues = parseIssuesFromNotes(review.notes);

  return (
    <DetailCard variant="warning">
      {/* Header */}
      <div className="flex items-center gap-3 mb-4">
        <div
          className="flex items-center justify-center w-9 h-9 rounded-xl shrink-0"
          style={{
            backgroundColor: isAiReviewer
              ? "rgba(10, 132, 255, 0.15)"
              : "rgba(52, 199, 89, 0.15)",
          }}
        >
          {isAiReviewer ? (
            <Bot className="w-5 h-5" style={{ color: "#0a84ff" }} />
          ) : (
            <User className="w-5 h-5" style={{ color: "#34c759" }} />
          )}
        </div>
        <div className="flex-1">
          <span className="text-[13px] font-semibold text-white/80 block">
            {isAiReviewer ? "AI Review Feedback" : "Human Review Feedback"}
          </span>
          <span className="text-[11px] text-white/40">{timeAgo}</span>
        </div>
      </div>

      {/* Main feedback text */}
      {(issues.mainText || (!issues.items.length && review.notes)) && (
        <p className="text-[13px] text-white/55 leading-relaxed mb-4 pl-12">
          {issues.mainText || review.notes}
        </p>
      )}

      {/* Issues list */}
      {issues.items.length > 0 && (
        <div className="pl-12">
          <div className="flex items-center gap-2 mb-2">
            <AlertTriangle className="w-3.5 h-3.5" style={{ color: "#ff9f0a" }} />
            <span className="text-[11px] font-semibold uppercase tracking-wider text-white/50">
              Issues to Address
            </span>
          </div>
          <ul className="space-y-0.5">
            {issues.items.map((issue, index) => (
              <IssueItem key={index} issue={issue} />
            ))}
          </ul>
        </div>
      )}
    </DetailCard>
  );
}

export function RevisionTaskDetail({ task }: RevisionTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;

  const attemptNumber = calculateAttemptNumber(history);
  const latestFeedback = getLatestRevisionFeedback(history);

  return (
    <TwoColumnLayout
      description={task.description}
      testId="revision-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={MessageCircleWarning}
        title="Revision Needed"
        subtitle="Changes were requested by the reviewer"
        variant="warning"
        badge={
          <StatusPill
            icon={RotateCcw}
            label={`Attempt #${attemptNumber}`}
            variant="warning"
            size="md"
          />
        }
      />

      {/* Review Feedback */}
      {historyLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      ) : latestFeedback ? (
        <section data-testid="revision-feedback-section">
          <SectionTitle>Feedback to Address</SectionTitle>
          <FeedbackCard review={latestFeedback} />
        </section>
      ) : null}

      {/* Steps */}
      {stepsLoading && (
        <div
          data-testid="revision-steps-loading"
          className="flex items-center justify-center py-8"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      )}

      {!stepsLoading && hasSteps && (
        <section data-testid="revision-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
