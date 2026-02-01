/**
 * RevisionTaskDetail - Task detail view for revision_needed state
 *
 * Shows revision-specific information including the review feedback that
 * triggered the revision, attempt count, and highlighted steps that need work.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Badge } from "@/components/ui/badge";
import { StepList } from "../StepList";
import { SectionTitle } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";
import { Loader2, AlertTriangle, Bot, User, RotateCcw, FileCode2 } from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface RevisionTaskDetailProps {
  task: Task;
}

/**
 * AttemptBadge - Shows the current revision attempt number
 */
function AttemptBadge({ attemptNumber }: { attemptNumber: number }) {
  return (
    <Badge
      data-testid="revision-attempt-badge"
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0 gap-1"
      style={{
        backgroundColor: "rgba(245, 158, 11, 0.15)",
        color: "var(--status-warning)",
      }}
    >
      <RotateCcw className="w-3 h-3" />
      Attempt #{attemptNumber}
    </Badge>
  );
}

/**
 * ReviewerIcon - Shows AI or Human icon based on reviewer type
 */
function ReviewerIcon({ reviewer }: { reviewer: string }) {
  const isAi = reviewer === "ai";
  return (
    <div
      className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
      style={{
        backgroundColor: isAi ? "rgba(59, 130, 246, 0.15)" : "rgba(16, 185, 129, 0.15)",
      }}
    >
      {isAi ? (
        <Bot
          className="w-3.5 h-3.5"
          style={{ color: "var(--status-info)" }}
        />
      ) : (
        <User
          className="w-3.5 h-3.5"
          style={{ color: "var(--status-success)" }}
        />
      )}
    </div>
  );
}

/**
 * IssueItem - Renders a single issue with optional file:line reference
 */
function IssueItem({ issue }: { issue: string }) {
  // Check if issue contains a file:line reference pattern (e.g., "src/auth.ts:42")
  const fileLineMatch = issue.match(/([a-zA-Z0-9_/./-]+\.[a-zA-Z]+):(\d+)/);

  if (fileLineMatch) {
    const [fullMatch, file, line] = fileLineMatch;
    const beforeMatch = issue.substring(0, issue.indexOf(fullMatch));
    const afterMatch = issue.substring(issue.indexOf(fullMatch) + fullMatch.length);

    return (
      <li className="flex items-start gap-2 text-[12px] text-white/60">
        <FileCode2 className="w-3.5 h-3.5 mt-0.5 shrink-0" style={{ color: "var(--status-warning)" }} />
        <span>
          {beforeMatch}
          <code
            className="px-1 py-0.5 rounded text-[11px] font-mono"
            style={{
              backgroundColor: "rgba(255, 107, 53, 0.1)",
              color: "var(--accent-primary)",
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
    <li className="flex items-start gap-2 text-[12px] text-white/60">
      <span className="text-white/40 mt-0.5">•</span>
      <span>{issue}</span>
    </li>
  );
}

/**
 * ReviewFeedbackCard - Displays review feedback from AI or Human reviewer
 */
function ReviewFeedbackCard({ review }: { review: ReviewNoteResponse }) {
  const isAi = review.reviewer === "ai";
  const timeAgo = formatTimeAgo(review.created_at);

  // Parse issues from notes if they're formatted as a list
  const issues = parseIssuesFromNotes(review.notes);

  return (
    <div
      data-testid="review-feedback-card"
      className="rounded-lg p-3 space-y-2"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: "1px solid rgba(245, 158, 11, 0.2)",
      }}
    >
      {/* Header: Reviewer icon + label + time */}
      <div className="flex items-center gap-2">
        <ReviewerIcon reviewer={review.reviewer} />
        <span className="text-[12px] font-medium text-white/70">
          {isAi ? "AI Review" : "Human Review"}
        </span>
        <span className="text-[11px] text-white/40 ml-auto">{timeAgo}</span>
      </div>

      {/* Feedback text */}
      {review.notes && (
        <p
          data-testid="review-feedback-notes"
          className="text-[12px] text-white/60"
          style={{ lineHeight: "1.5" }}
        >
          {issues.mainText || review.notes}
        </p>
      )}

      {/* Issues list if parsed */}
      {issues.items.length > 0 && (
        <div className="pt-1">
          <p className="text-[11px] font-medium text-white/50 mb-1.5">Issues:</p>
          <ul className="space-y-1.5">
            {issues.items.map((issue, index) => (
              <IssueItem key={index} issue={issue} />
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

/**
 * Parse issues from notes text
 * Looks for bullet points or numbered lists
 */
function parseIssuesFromNotes(notes: string | null | undefined): {
  mainText: string;
  items: string[];
} {
  if (!notes) return { mainText: "", items: [] };

  // Split on newlines and look for list items
  const lines = notes.split("\n");
  const items: string[] = [];
  const mainTextLines: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    // Match bullet points (-, *, •) or numbered items (1., 2., etc.)
    if (/^[-*•]/.test(trimmed) || /^\d+\./.test(trimmed)) {
      // Extract the item text after the bullet/number
      const itemText = trimmed.replace(/^[-*•]\s*/, "").replace(/^\d+\.\s*/, "");
      if (itemText) items.push(itemText);
    } else if (trimmed) {
      mainTextLines.push(trimmed);
    }
  }

  return {
    mainText: mainTextLines.join(" "),
    items,
  };
}

/**
 * Format time ago string from ISO date
 */
function formatTimeAgo(isoDate: string): string {
  const date = new Date(isoDate);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;

  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;

  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

/**
 * Calculate the revision attempt number from state history
 * Counts the number of 'changes_requested' outcomes
 */
function calculateAttemptNumber(history: ReviewNoteResponse[]): number {
  const changesRequestedCount = history.filter(
    (entry) => entry.outcome === "changes_requested"
  ).length;
  // Attempt number is 1-indexed, so first revision is attempt 2
  return changesRequestedCount + 1;
}

/**
 * Get the latest review feedback that triggered revision
 */
function getLatestRevisionFeedback(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  // Find the most recent changes_requested entry
  const revisionEntries = history.filter(
    (entry) => entry.outcome === "changes_requested"
  );
  if (revisionEntries.length === 0) return null;
  // History is already sorted newest first by useTaskStateHistory
  // Use nullish coalescing to satisfy TypeScript (array access returns T | undefined)
  return revisionEntries[0] ?? null;
}

/**
 * RevisionTaskDetail Component
 *
 * Renders task information for revision_needed state.
 * Shows: revision banner, attempt badge, review feedback, description, and steps.
 */
export function RevisionTaskDetail({ task }: RevisionTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;

  const attemptNumber = calculateAttemptNumber(history);
  const latestFeedback = getLatestRevisionFeedback(history);

  return (
    <div
      data-testid="revision-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* Revision Needed Banner */}
      <div
        data-testid="revision-banner"
        className="flex items-center gap-2 px-3 py-2 rounded-lg"
        style={{
          backgroundColor: "rgba(245, 158, 11, 0.1)",
          border: "1px solid rgba(245, 158, 11, 0.25)",
        }}
      >
        <AlertTriangle
          className="w-4 h-4 shrink-0"
          style={{ color: "var(--status-warning)" }}
        />
        <span
          className="text-[13px] font-medium"
          style={{ color: "var(--status-warning)" }}
        >
          REVISION NEEDED
        </span>
        <div className="ml-auto">
          <AttemptBadge attemptNumber={attemptNumber} />
        </div>
      </div>

      {/* Review Feedback Section */}
      {historyLoading ? (
        <div
          data-testid="revision-feedback-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      ) : latestFeedback ? (
        <div data-testid="revision-feedback-section">
          <SectionTitle>Review Feedback to Address</SectionTitle>
          <ReviewFeedbackCard review={latestFeedback} />
        </div>
      ) : null}

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="revision-task-description"
            className="text-[13px] text-white/60"
            style={{
              lineHeight: "1.6",
              wordBreak: "break-word",
            }}
          >
            {task.description}
          </p>
        ) : (
          <p className="text-[13px] italic text-white/35">
            No description provided
          </p>
        )}
      </div>

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="revision-steps-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-6 h-6 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}
      {!stepsLoading && hasSteps && (
        <div data-testid="revision-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </div>
      )}
    </div>
  );
}
