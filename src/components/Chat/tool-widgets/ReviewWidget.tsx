/**
 * ReviewWidget — Specialized renderer for complete_review and get_review_notes tool calls.
 *
 * complete_review: outcome-colored card showing decision, feedback, and issues.
 * get_review_notes: compact list of review notes with timestamps and outcomes.
 */

import React, { useState, useMemo } from "react";
import {
  CheckCircle,
  AlertTriangle,
  AlertCircle,
  ChevronDown,
  ChevronRight,
  FileText,
  Clock,
} from "lucide-react";
import type { ToolCallWidgetProps } from "./shared";

// ============================================================================
// Types
// ============================================================================

interface ReviewIssue {
  severity: string;
  file?: string;
  line?: number;
  description: string;
}

interface CompleteReviewArgs {
  task_id?: string;
  decision?: string;
  feedback?: string;
  summary?: string;
  issues?: ReviewIssue[];
}

interface CompleteReviewResult {
  success?: boolean;
  message?: string;
  new_status?: string;
  fix_task_id?: string;
}

interface ReviewNoteEntry {
  id: string;
  reviewer: string;
  outcome: string;
  summary?: string;
  notes?: string;
  issues?: ReviewIssue[];
  created_at: string;
}

interface ReviewNotesResult {
  task_id?: string;
  revision_count?: number;
  max_revisions?: number;
  reviews?: ReviewNoteEntry[];
}

// ============================================================================
// Helpers
// ============================================================================

function parseArgs<T>(args: unknown): T {
  if (!args || typeof args !== "object") return {} as T;
  return args as T;
}

function parseResult<T>(result: unknown): T | null {
  if (result == null) return null;
  if (typeof result === "string") {
    try {
      return JSON.parse(result) as T;
    } catch {
      return null;
    }
  }
  return result as T;
}

const OUTCOME_STYLES = {
  approved: {
    bg: "hsla(142, 70%, 45%, 0.12)",
    border: "hsla(142, 70%, 45%, 0.25)",
    accent: "hsl(142, 70%, 55%)",
    text: "hsl(142, 70%, 70%)",
    label: "Approved",
    Icon: CheckCircle,
  },
  approved_no_changes: {
    bg: "hsla(142, 70%, 45%, 0.12)",
    border: "hsla(142, 70%, 45%, 0.25)",
    accent: "hsl(142, 70%, 55%)",
    text: "hsl(142, 70%, 70%)",
    label: "No Changes",
    Icon: CheckCircle,
  },
  needs_changes: {
    bg: "hsla(25, 95%, 53%, 0.12)",
    border: "hsla(25, 95%, 53%, 0.25)",
    accent: "hsl(25, 95%, 53%)",
    text: "hsl(25, 90%, 68%)",
    label: "Changes Requested",
    Icon: AlertTriangle,
  },
  changes_requested: {
    bg: "hsla(25, 95%, 53%, 0.12)",
    border: "hsla(25, 95%, 53%, 0.25)",
    accent: "hsl(25, 95%, 53%)",
    text: "hsl(25, 90%, 68%)",
    label: "Changes Requested",
    Icon: AlertTriangle,
  },
  escalate: {
    bg: "hsla(210, 70%, 55%, 0.12)",
    border: "hsla(210, 70%, 55%, 0.25)",
    accent: "hsl(210, 70%, 55%)",
    text: "hsl(210, 70%, 70%)",
    label: "Escalated",
    Icon: AlertCircle,
  },
  rejected: {
    bg: "hsla(210, 70%, 55%, 0.12)",
    border: "hsla(210, 70%, 55%, 0.25)",
    accent: "hsl(210, 70%, 55%)",
    text: "hsl(210, 70%, 70%)",
    label: "Escalated",
    Icon: AlertCircle,
  },
} as const;

type OutcomeKey = keyof typeof OUTCOME_STYLES;

const DEFAULT_STYLE = {
  bg: "hsl(220 10% 14%)",
  border: "hsla(220, 10%, 100%, 0.06)",
  accent: "hsl(220, 10%, 55%)",
  text: "hsl(220, 10%, 70%)",
  label: "Review",
  Icon: FileText,
};

function getOutcomeStyle(outcome: string | undefined) {
  if (!outcome) return DEFAULT_STYLE;
  return OUTCOME_STYLES[outcome as OutcomeKey] ?? DEFAULT_STYLE;
}

function getSeverityColor(severity: string): string {
  switch (severity) {
    case "critical":
      return "hsl(0, 70%, 65%)";
    case "major":
      return "hsl(25, 95%, 60%)";
    case "minor":
      return "hsl(45, 80%, 60%)";
    case "suggestion":
      return "hsl(210, 60%, 65%)";
    default:
      return "hsl(220, 10%, 60%)";
  }
}

function getSeverityBg(severity: string): string {
  switch (severity) {
    case "critical":
      return "hsla(0, 70%, 55%, 0.15)";
    case "major":
      return "hsla(25, 95%, 53%, 0.15)";
    case "minor":
      return "hsla(45, 80%, 50%, 0.15)";
    case "suggestion":
      return "hsla(210, 60%, 55%, 0.15)";
    default:
      return "hsla(220, 10%, 50%, 0.15)";
  }
}

function formatTimestamp(ts: string): string {
  try {
    const date = new Date(ts);
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

// ============================================================================
// CompleteReview Renderer
// ============================================================================

function CompleteReviewCard({ toolCall, className = "", compact = false }: ToolCallWidgetProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const args = useMemo(() => parseArgs<CompleteReviewArgs>(toolCall.arguments), [toolCall.arguments]);
  const result = useMemo(() => parseResult<CompleteReviewResult>(toolCall.result), [toolCall.result]);
  const hasError = Boolean(toolCall.error);

  const style = getOutcomeStyle(args.decision);
  const { Icon } = style;
  const issues = args.issues ?? [];
  const hasBody = Boolean(args.feedback) || issues.length > 0;
  const iconSize = compact ? 12 : 14;

  return (
    <div
      data-testid="review-widget-complete"
      className={`${compact ? "rounded-md" : "rounded-lg"} overflow-hidden ${compact ? "mb-1" : ""} ${className}`}
      style={{
        backgroundColor: hasError ? "hsla(0, 70%, 55%, 0.15)" : style.bg,
        borderLeft: `3px solid ${hasError ? "hsl(0, 70%, 55%)" : style.accent}`,
      }}
    >
      {/* Header */}
      <button
        onClick={() => hasBody && setIsExpanded(!isExpanded)}
        className={`w-full flex items-center gap-2 ${compact ? "px-2 py-1.5" : "px-3 py-2"} text-left ${hasBody ? "hover:opacity-80 cursor-pointer" : "cursor-default"} transition-opacity`}
        aria-expanded={hasBody ? isExpanded : undefined}
      >
        {/* Expand/collapse chevron */}
        {hasBody ? (
          isExpanded ? (
            <ChevronDown size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220, 10%, 45%)" }} />
          ) : (
            <ChevronRight size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220, 10%, 45%)" }} />
          )
        ) : null}

        {/* Outcome icon */}
        <Icon size={iconSize} className="flex-shrink-0" style={{ color: style.accent }} />

        {/* Outcome badge */}
        <span
          className={`${compact ? "text-[9px]" : "text-[10px]"} px-1.5 py-0.5 rounded font-medium flex-shrink-0`}
          style={{ backgroundColor: style.bg, color: style.text, border: `1px solid ${style.border}` }}
        >
          {style.label}
        </span>

        {/* Issue count or summary */}
        <span
          className={`${compact ? "text-[11px]" : "text-xs"} truncate flex-1 min-w-0`}
          style={{ color: hasError ? "hsl(0, 70%, 75%)" : "hsl(220, 10%, 70%)" }}
        >
          {issues.length > 0
            ? `${issues.length} issue${issues.length !== 1 ? "s" : ""} found`
            : args.summary || (result?.new_status ? `→ ${result.new_status.replace(/_/g, " ")}` : "")}
        </span>

        {/* Error indicator */}
        {hasError && (
          <span
            className={`${compact ? "text-[9px]" : "text-[10px]"} font-medium px-1.5 py-0.5 rounded`}
            style={{ backgroundColor: "hsla(0, 70%, 50%, 0.2)", color: "hsl(0, 70%, 70%)" }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded body */}
      {isExpanded && hasBody && (
        <div
          className={`${compact ? "px-2 pb-2" : "px-3 pb-3"} space-y-2 pt-1`}
          style={{ borderTop: "1px solid hsla(220, 10%, 100%, 0.04)" }}
        >
          {/* Feedback text */}
          {args.feedback && (
            <div
              className={`${compact ? "text-[10px]" : "text-[11px]"} px-2 py-1.5 rounded`}
              style={{
                backgroundColor: "hsl(220, 10%, 10%)",
                color: "hsl(220, 10%, 75%)",
                whiteSpace: "pre-wrap",
                lineHeight: 1.5,
              }}
            >
              {args.feedback}
            </div>
          )}

          {/* Issues list */}
          {issues.length > 0 && (
            <div className="space-y-1">
              <div
                className={`${compact ? "text-[9px]" : "text-[10px]"} font-medium uppercase tracking-wide`}
                style={{ color: "hsl(220, 10%, 45%)" }}
              >
                Issues
              </div>
              {issues.map((issue, idx) => (
                <div
                  key={idx}
                  className={`${compact ? "text-[10px]" : "text-[11px]"} px-2 py-1.5 rounded flex items-start gap-2`}
                  style={{ backgroundColor: "hsl(220, 10%, 10%)" }}
                >
                  {/* Severity badge */}
                  <span
                    className="text-[9px] px-1 py-0.5 rounded flex-shrink-0 font-medium mt-0.5"
                    style={{ backgroundColor: getSeverityBg(issue.severity), color: getSeverityColor(issue.severity) }}
                  >
                    {issue.severity}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div style={{ color: "hsl(220, 10%, 80%)" }}>{issue.description}</div>
                    {issue.file && (
                      <div className="mt-0.5 font-mono text-[9px]" style={{ color: "hsl(220, 10%, 50%)" }}>
                        {issue.file}{issue.line != null ? `:${issue.line}` : ""}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// GetReviewNotes Renderer
// ============================================================================

function GetReviewNotesCard({ toolCall, className = "", compact = false }: ToolCallWidgetProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const result = useMemo(() => parseResult<ReviewNotesResult>(toolCall.result), [toolCall.result]);
  const hasError = Boolean(toolCall.error);

  const reviews = result?.reviews ?? [];
  const revisionCount = result?.revision_count ?? 0;
  const maxRevisions = result?.max_revisions;
  const hasBody = reviews.length > 0;
  const iconSize = compact ? 12 : 14;

  // Empty state
  if (!hasError && reviews.length === 0 && toolCall.result != null) {
    return (
      <div
        data-testid="review-widget-notes-empty"
        className={`${compact ? "rounded-md" : "rounded-lg"} overflow-hidden ${compact ? "mb-1" : ""} ${className}`}
        style={{ backgroundColor: "hsl(220, 10%, 14%)" }}
      >
        <div className={`flex items-center gap-2 ${compact ? "px-2 py-1.5" : "px-3 py-2"}`}>
          <FileText size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220, 10%, 40%)" }} />
          <span className={`${compact ? "text-[11px]" : "text-xs"}`} style={{ color: "hsl(220, 10%, 50%)" }}>
            No review notes
          </span>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="review-widget-notes"
      className={`${compact ? "rounded-md" : "rounded-lg"} overflow-hidden ${compact ? "mb-1" : ""} ${className}`}
      style={{
        backgroundColor: hasError ? "hsla(0, 70%, 55%, 0.15)" : "hsl(220, 10%, 14%)",
        border: "none",
      }}
    >
      {/* Header */}
      <button
        onClick={() => hasBody && setIsExpanded(!isExpanded)}
        className={`w-full flex items-center gap-2 ${compact ? "px-2 py-1.5" : "px-3 py-2"} text-left ${hasBody ? "hover:opacity-80 cursor-pointer" : "cursor-default"} transition-opacity`}
        aria-expanded={hasBody ? isExpanded : undefined}
      >
        {hasBody ? (
          isExpanded ? (
            <ChevronDown size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220, 10%, 45%)" }} />
          ) : (
            <ChevronRight size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220, 10%, 45%)" }} />
          )
        ) : null}

        <FileText size={iconSize} className="flex-shrink-0" style={{ color: "hsl(14, 100%, 60%)" }} />

        <span
          className={`${compact ? "text-[11px]" : "text-xs"} flex-1 min-w-0`}
          style={{ color: "hsl(220, 10%, 75%)" }}
        >
          {reviews.length} review note{reviews.length !== 1 ? "s" : ""}
        </span>

        {/* Revision counter badge */}
        {revisionCount > 0 && (
          <span
            className={`${compact ? "text-[9px]" : "text-[10px]"} px-1.5 py-0.5 rounded flex-shrink-0`}
            style={{
              backgroundColor: "hsla(25, 95%, 53%, 0.15)",
              color: "hsl(25, 90%, 65%)",
            }}
          >
            {revisionCount}{maxRevisions != null ? `/${maxRevisions}` : ""} revision{revisionCount !== 1 ? "s" : ""}
          </span>
        )}

        {hasError && (
          <span
            className={`${compact ? "text-[9px]" : "text-[10px]"} font-medium px-1.5 py-0.5 rounded`}
            style={{ backgroundColor: "hsla(0, 70%, 50%, 0.2)", color: "hsl(0, 70%, 70%)" }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded: note list */}
      {isExpanded && hasBody && (
        <div
          className={`${compact ? "px-2 pb-2" : "px-3 pb-3"} space-y-1.5 pt-1`}
          style={{ borderTop: "1px solid hsla(220, 10%, 100%, 0.04)" }}
        >
          {reviews.map((note) => {
            const noteStyle = getOutcomeStyle(note.outcome);
            const NoteIcon = noteStyle.Icon;
            return (
              <div
                key={note.id}
                className={`${compact ? "text-[10px]" : "text-[11px]"} px-2 py-1.5 rounded`}
                style={{ backgroundColor: "hsl(220, 10%, 10%)" }}
              >
                <div className="flex items-center gap-1.5 mb-1">
                  <NoteIcon size={10} style={{ color: noteStyle.accent }} />
                  <span className="font-medium" style={{ color: noteStyle.text }}>
                    {noteStyle.label}
                  </span>
                  <span className="text-[9px]" style={{ color: "hsl(220, 10%, 45%)" }}>
                    {note.reviewer}
                  </span>
                  {note.created_at && (
                    <span className="flex items-center gap-0.5 text-[9px] ml-auto" style={{ color: "hsl(220, 10%, 40%)" }}>
                      <Clock size={8} />
                      {formatTimestamp(note.created_at)}
                    </span>
                  )}
                </div>
                {note.notes && (
                  <div style={{ color: "hsl(220, 10%, 70%)", lineHeight: 1.4, whiteSpace: "pre-wrap" }}>
                    {note.notes.length > 200 ? note.notes.slice(0, 200) + "..." : note.notes}
                  </div>
                )}
                {note.issues && note.issues.length > 0 && (
                  <div className="mt-1 flex items-center gap-1">
                    <span style={{ color: "hsl(220, 10%, 50%)" }}>
                      {note.issues.length} issue{note.issues.length !== 1 ? "s" : ""}
                    </span>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Main Widget (dispatches based on tool name)
// ============================================================================

export const ReviewWidget = React.memo(function ReviewWidget(props: ToolCallWidgetProps) {
  const toolName = props.toolCall.name.toLowerCase();
  if (toolName === "complete_review") {
    return <CompleteReviewCard {...props} />;
  }
  return <GetReviewNotesCard {...props} />;
});

