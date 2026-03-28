/**
 * ReviewTimeline - macOS Tahoe-inspired review history timeline
 *
 * Shows a vertical timeline of review events with collapsible markdown content.
 */

import { useState, Fragment } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { CheckCircle2, RotateCcw, Bot, User, Settings, ExternalLink } from "lucide-react";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import type { ReviewNoteResponse } from "@/lib/tauri";
import type { StateTransition } from "@/api/tasks";
import { navigateToIdeationSession } from "@/lib/navigation";

// ============================================================================
// Staleness Detection
// ============================================================================

/**
 * Statuses that indicate the task progressed past a "changes_requested" review.
 * A review is stale if any transition to these statuses happened after it.
 */
const STALE_TRIGGER_STATUSES = new Set<string>([
  "re_executing",
  "approved",
  "pending_merge",
  "merging",
  "merged",
]);

/**
 * Milestone statuses shown in the resolution trail (in display order).
 */
const TRAIL_MILESTONES: { status: string; label: string }[] = [
  { status: "re_executing", label: "Re-execution" },
  { status: "approved", label: "Approved" },
  { status: "merged", label: "Merged" },
];

/**
 * Returns true if the given "changes_requested" review has been superseded
 * by a subsequent state transition (e.g. re_executing, approved, merged).
 */
function isReviewStale(
  review: ReviewNoteResponse,
  stateTransitions: StateTransition[]
): boolean {
  if (review.outcome !== "changes_requested") return false;
  const reviewDate = new Date(review.created_at);
  return stateTransitions.some(
    (t) =>
      STALE_TRIGGER_STATUSES.has(t.toStatus) &&
      new Date(t.timestamp) > reviewDate
  );
}

/**
 * Returns the ordered milestone transitions that occurred after the given review.
 * Used to render the resolution trail below a stale review.
 */
function getResolutionTrail(
  review: ReviewNoteResponse,
  stateTransitions: StateTransition[]
): { label: string; timestamp: string }[] {
  const reviewDate = new Date(review.created_at);
  const result: { label: string; timestamp: string }[] = [];

  for (const { status, label } of TRAIL_MILESTONES) {
    // Find the first transition to this milestone status after the review
    const transition = stateTransitions
      .filter(
        (t) => t.toStatus === status && new Date(t.timestamp) > reviewDate
      )
      .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())[0];

    if (transition) {
      result.push({ label, timestamp: transition.timestamp });
    }
  }

  return result;
}

// ============================================================================
// Helpers
// ============================================================================

function formatRelativeTime(date: Date | string | undefined): string {
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

// ============================================================================
// TimelineItem
// ============================================================================

interface TimelineItemProps {
  entry: ReviewNoteResponse;
  isLast: boolean;
  attemptNumber?: number;
  isStale?: boolean;
  resolutionTrail?: { label: string; timestamp: string }[];
}

function TimelineItem({
  entry,
  isLast,
  attemptNumber,
  isStale = false,
  resolutionTrail = [],
}: TimelineItemProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const isApproved = entry.outcome === "approved" || entry.outcome === "approved_no_changes";
  const isNoChanges = entry.outcome === "approved_no_changes";
  const isChangesRequested = entry.outcome === "changes_requested";
  const isHuman = entry.reviewer === "human";
  const isSystem = entry.reviewer === "system";

  // Use summary if available, otherwise fall back to notes
  const hasSummary = !!entry.summary;
  const hasNotes = !!entry.notes;
  const hasContent = hasSummary || hasNotes;

  // Expandable if there are full notes different from summary
  const isExpandable = hasSummary && hasNotes;

  const handleContentClick = () => {
    if (isExpandable && !isExpanded) {
      setIsExpanded(true);
    }
  };

  const getConfig = () => {
    if (isApproved) {
      return {
        Icon: CheckCircle2,
        color: "#34c759",
        bgColor: "rgba(52, 199, 89, 0.15)",
        lineColor: "rgba(52, 199, 89, 0.3)",
      };
    }
    if (isChangesRequested) {
      return {
        Icon: RotateCcw,
        color: "#ff9f0a",
        bgColor: "rgba(255, 159, 10, 0.15)",
        lineColor: "rgba(255, 159, 10, 0.3)",
      };
    }
    return {
      Icon: CheckCircle2,
      color: "#8e8e93",
      bgColor: "rgba(142, 142, 147, 0.15)",
      lineColor: "rgba(142, 142, 147, 0.2)",
    };
  };

  const config = getConfig();
  const ReviewerIcon = isHuman ? User : isSystem ? Settings : Bot;
  const reviewerLabel = isHuman ? "Human" : isSystem ? "System" : "AI";

  const getLabel = () => {
    if (attemptNumber !== undefined && isChangesRequested) {
      return `Attempt #${attemptNumber}: Changes requested`;
    }
    if (isNoChanges) {
      return `${reviewerLabel} approved (no changes)`;
    }
    if (isApproved) {
      return `${reviewerLabel} approved`;
    }
    if (isChangesRequested) {
      return `${reviewerLabel} requested changes`;
    }
    return `${reviewerLabel} reviewed`;
  };

  return (
    <div className="flex gap-3">
      {/* Timeline connector — dimmed for stale reviews */}
      <div
        className="flex flex-col items-center"
        style={isStale ? { opacity: 0.6 } : undefined}
      >
        {/* Icon circle */}
        <div
          className="flex items-center justify-center w-7 h-7 rounded-xl shrink-0"
          style={{ backgroundColor: config.bgColor }}
        >
          <config.Icon className="w-4 h-4" style={{ color: config.color }} />
        </div>
        {/* Vertical line */}
        {(!isLast || (isStale && resolutionTrail.length > 0)) && (
          <div
            className="w-0.5 flex-1 min-h-[20px] mt-1"
            style={{ backgroundColor: config.lineColor }}
          />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 pb-4">
        {/* Main content — dimmed for stale reviews */}
        <div style={isStale ? { opacity: 0.6 } : undefined}>
          <div className="flex items-center gap-2">
            <ReviewerIcon
              className="w-3.5 h-3.5"
              style={{ color: isHuman ? "#34c759" : isSystem ? "#ff9f0a" : "#0a84ff" }}
            />
            <span className="text-[12px] font-semibold text-white/75">
              {getLabel()}
            </span>
            {isStale && (
              <span
                className="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded"
                style={{
                  backgroundColor: "rgba(52, 199, 89, 0.12)",
                  color: "hsl(145 60% 45%)",
                }}
              >
                Superseded
              </span>
            )}
            <span className="text-[11px] text-white/40 ml-auto">
              {formatRelativeTime(entry.created_at)}
            </span>
          </div>
          {hasContent && (
            <div className="mt-1.5 pl-5">
              {/* Summary (always shown) or notes if no summary */}
              <div
                onClick={handleContentClick}
                className={isExpandable && !isExpanded ? "cursor-pointer" : ""}
              >
                <div className="text-[12px] text-white/50 leading-relaxed">
                  {hasSummary ? (
                    <div className="prose prose-sm prose-invert max-w-none">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={markdownComponents}
                      >
                        {entry.summary ?? ""}
                      </ReactMarkdown>
                    </div>
                  ) : (
                    // No summary - show notes as markdown
                    <div className="prose prose-sm prose-invert max-w-none">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={markdownComponents}
                      >
                        {entry.notes ?? ""}
                      </ReactMarkdown>
                    </div>
                  )}
                </div>

                {/* Show more link when expandable and collapsed */}
                {isExpandable && !isExpanded && (
                  <div
                    className="mt-1 text-[11px] font-medium"
                    style={{ color: "hsl(217 90% 60%)" }}
                  >
                    Show details
                  </div>
                )}
              </div>

              {/* Expanded notes */}
              {isExpandable && isExpanded && (
                <div className="mt-3">
                  <div className="text-[12px] text-white/50 leading-relaxed prose prose-sm prose-invert max-w-none">
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      components={markdownComponents}
                    >
                      {entry.notes ?? ""}
                    </ReactMarkdown>
                  </div>
                  <button
                    onClick={() => setIsExpanded(false)}
                    className="mt-2 text-[11px] font-medium transition-colors hover:opacity-80"
                    style={{ color: "hsl(217 90% 60%)" }}
                  >
                    Show less
                  </button>
                </div>
              )}

              {entry.followup_session_id && (
                <div className="mt-3 flex items-center justify-between gap-2 rounded-lg px-2.5 py-2"
                  style={{
                    backgroundColor: "rgba(255, 107, 53, 0.08)",
                    border: "1px solid rgba(255, 107, 53, 0.14)",
                  }}
                >
                  <div className="min-w-0">
                    <div className="text-[11px] font-medium text-white/65">
                      Follow-up ideation session
                    </div>
                    <div className="mt-0.5 text-[11px] text-white/45 break-all">
                      {entry.followup_session_id}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => navigateToIdeationSession(entry.followup_session_id!)}
                    className="shrink-0 inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-opacity hover:opacity-80"
                    style={{
                      color: "hsl(14 100% 68%)",
                      backgroundColor: "rgba(255, 107, 53, 0.12)",
                    }}
                  >
                    <ExternalLink className="w-3 h-3" />
                    Open
                  </button>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Resolution trail — rendered at full opacity below the stale review */}
        {isStale && resolutionTrail.length > 0 && (
          <div className="pl-5 mt-1.5 flex items-center gap-1 flex-wrap">
            {resolutionTrail.map((item, i) => (
              <Fragment key={`${item.label}-${i}`}>
                <span className="text-[11px]" style={{ color: "hsl(220 10% 45%)" }}>
                  {item.label}{" "}
                  <span style={{ color: "hsl(220 10% 35%)" }}>
                    ({formatRelativeTime(item.timestamp)})
                  </span>
                </span>
                {i < resolutionTrail.length - 1 && (
                  <span
                    className="text-[11px]"
                    style={{ color: "hsl(220 10% 30%)" }}
                  >
                    →
                  </span>
                )}
              </Fragment>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// ReviewTimeline
// ============================================================================

export interface ReviewTimelineProps {
  history: ReviewNoteResponse[];
  filter?: (entry: ReviewNoteResponse) => boolean;
  emptyMessage?: string;
  showAttemptNumbers?: boolean;
  /** State transitions used to detect stale "changes_requested" reviews */
  stateTransitions?: StateTransition[];
}

export function ReviewTimeline({
  history,
  filter,
  emptyMessage = "No review history available",
  showAttemptNumbers = false,
  stateTransitions = [],
}: ReviewTimelineProps) {
  const displayedHistory = filter ? history.filter(filter) : history;

  if (displayedHistory.length === 0) {
    return (
      <p className="text-[12px] text-white/35 italic py-2">
        {emptyMessage}
      </p>
    );
  }

  return (
    <div data-testid="review-history-timeline">
      {displayedHistory.map((entry, index) => {
        const stale =
          stateTransitions.length > 0 && isReviewStale(entry, stateTransitions);
        const trail = stale
          ? getResolutionTrail(entry, stateTransitions)
          : [];

        return (
          <TimelineItem
            key={entry.id}
            entry={entry}
            isLast={index === displayedHistory.length - 1}
            isStale={stale}
            resolutionTrail={trail}
            {...(showAttemptNumbers && {
              // Array is newest-first, so invert: oldest = #1, newest = #N
              attemptNumber: displayedHistory.length - index,
            })}
          />
        );
      })}
    </div>
  );
}
