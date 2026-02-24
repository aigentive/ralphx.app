/* eslint-disable react-refresh/only-export-components -- pure constants/helpers file, no HMR components */
/**
 * EventCardHelpers - Shared constants, helpers, and sub-components for EventCards.
 */

import { useState, useCallback } from "react";
import {
  MessageSquare,
  Terminal,
  Brain,
  AlertCircle,
  ChevronDown,
  ChevronRight,
  Settings,
} from "lucide-react";

import type { AuditEntry } from "./EventCards";

// ============================================================================
// Style constants
// ============================================================================

export const STATUS_COLORS: Record<
  string,
  { label: string; color: string; bgColor: string }
> = {
  backlog: { label: "Backlog", color: "#8e8e93", bgColor: "rgba(142,142,147,0.15)" },
  ready: { label: "Ready", color: "#0a84ff", bgColor: "rgba(10,132,255,0.15)" },
  blocked: { label: "Blocked", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  executing: { label: "Executing", color: "#ff6b35", bgColor: "rgba(255,107,53,0.15)" },
  qa_refining: { label: "QA Refining", color: "#ff6b35", bgColor: "rgba(255,107,53,0.15)" },
  qa_testing: { label: "QA Testing", color: "#ff6b35", bgColor: "rgba(255,107,53,0.15)" },
  qa_passed: { label: "QA Passed", color: "#34c759", bgColor: "rgba(52,199,89,0.15)" },
  qa_failed: { label: "QA Failed", color: "#ff453a", bgColor: "rgba(255,69,58,0.15)" },
  pending_review: { label: "Pending Review", color: "#8e8e93", bgColor: "rgba(142,142,147,0.15)" },
  revision_needed: { label: "Revision Needed", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  approved: { label: "Approved", color: "#34c759", bgColor: "rgba(52,199,89,0.15)" },
  failed: { label: "Failed", color: "#ff453a", bgColor: "rgba(255,69,58,0.15)" },
  cancelled: { label: "Cancelled", color: "#8e8e93", bgColor: "rgba(142,142,147,0.15)" },
  reviewing: { label: "Reviewing", color: "#0a84ff", bgColor: "rgba(10,132,255,0.15)" },
  review_passed: { label: "Review Passed", color: "#34c759", bgColor: "rgba(52,199,89,0.15)" },
  escalated: { label: "Escalated", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  re_executing: { label: "Re-executing", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  pending_merge: { label: "Pending Merge", color: "#ff6b35", bgColor: "rgba(255,107,53,0.15)" },
  merging: { label: "Merging", color: "#ff6b35", bgColor: "rgba(255,107,53,0.15)" },
  merge_incomplete: { label: "Merge Incomplete", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  merge_conflict: { label: "Merge Conflict", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  merged: { label: "Merged", color: "#34c759", bgColor: "rgba(52,199,89,0.15)" },
  paused: { label: "Paused", color: "#ff9f0a", bgColor: "rgba(255,159,10,0.15)" },
  stopped: { label: "Stopped", color: "#ff453a", bgColor: "rgba(255,69,58,0.15)" },
};

export const SOURCE_STYLES: Record<
  AuditEntry["source"],
  { label: string; bg: string; border: string; color: string }
> = {
  transition: {
    label: "Transition",
    bg: "rgba(10,132,255,0.12)",
    border: "rgba(10,132,255,0.25)",
    color: "#0a84ff",
  },
  review: {
    label: "Review",
    bg: "rgba(34,197,94,0.12)",
    border: "rgba(34,197,94,0.25)",
    color: "rgb(74,222,128)",
  },
  activity: {
    label: "Activity",
    bg: "rgba(255,107,53,0.12)",
    border: "rgba(255,107,53,0.25)",
    color: "var(--accent-primary)",
  },
};

export const REVIEW_OUTCOME_CONFIG: Record<string, { color: string; bgColor: string }> = {
  Approved: { color: "var(--status-success)", bgColor: "rgba(52,199,89,0.12)" },
  "Changes Requested": { color: "var(--status-warning)", bgColor: "rgba(255,159,10,0.12)" },
  Rejected: { color: "var(--status-error)", bgColor: "rgba(255,69,58,0.12)" },
};

export const ACTIVITY_TYPE_CONFIG: Record<
  string,
  { icon: typeof MessageSquare; color: string }
> = {
  text: { icon: MessageSquare, color: "var(--text-muted)" },
  tool_call: { icon: Terminal, color: "var(--accent-primary)" },
  tool_result: { icon: Terminal, color: "var(--text-secondary)" },
  thinking: { icon: Brain, color: "var(--text-muted)" },
  error: { icon: AlertCircle, color: "var(--status-error)" },
  system: { icon: Settings, color: "var(--text-muted)" },
};

export const DEFAULT_ACTIVITY_CONFIG = { icon: MessageSquare, color: "var(--text-muted)" };

export const CARD_STYLE = {
  backgroundColor: "rgba(0,0,0,0.15)",
  border: "1px solid rgba(255,255,255,0.05)",
};

export const ERROR_CARD_STYLE = {
  backgroundColor: "rgba(255,69,58,0.06)",
  border: "1px solid rgba(255,69,58,0.15)",
  borderLeft: "3px solid rgba(255,69,58,0.5)",
};

export const CONTENT_TRUNCATE_LENGTH = 200;
export const THINKING_TRUNCATE_LENGTH = 100;

// ============================================================================
// Helpers
// ============================================================================

export function formatTimestamp(dateString: string): string {
  try {
    return new Date(dateString).toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    });
  } catch {
    return dateString;
  }
}

export function extractToolName(description: string): string | null {
  try {
    const parsed = JSON.parse(description) as Record<string, unknown>;
    if (typeof parsed.tool === "string") return parsed.tool;
    if (typeof parsed.name === "string") return parsed.name;
  } catch {
    // not JSON — fall through
  }
  return null;
}

// ============================================================================
// Shared sub-components
// ============================================================================

export function ExpandableContent({
  text,
  maxLength = CONTENT_TRUNCATE_LENGTH,
  italic = false,
}: {
  text: string;
  maxLength?: number;
  italic?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const needsTruncation = text.length > maxLength;
  const displayText =
    !expanded && needsTruncation ? text.slice(0, maxLength) + "..." : text;
  const toggle = useCallback(() => setExpanded((p) => !p), []);

  return (
    <div className="mt-1">
      <p
        className={`text-[12px] whitespace-pre-wrap break-words${italic ? " italic" : ""}`}
        style={{ color: "rgba(255,255,255,0.6)", lineHeight: "1.5" }}
      >
        {displayText}
      </p>
      {needsTruncation && (
        <button
          onClick={toggle}
          className="flex items-center gap-0.5 mt-0.5 text-[11px]"
          style={{ color: "var(--accent-primary)" }}
        >
          {expanded ? (
            <>
              <ChevronDown className="w-3 h-3" /> Show less
            </>
          ) : (
            <>
              <ChevronRight className="w-3 h-3" /> Show more
            </>
          )}
        </button>
      )}
    </div>
  );
}

export function StatusBadge({ status }: { status: string }) {
  const config = STATUS_COLORS[status] ?? {
    label: status,
    color: "#8e8e93",
    bgColor: "rgba(142,142,147,0.15)",
  };
  return (
    <span
      className="rounded-full px-2 py-0.5 text-[10px] font-semibold"
      style={{
        backgroundColor: config.bgColor,
        color: config.color,
        border: `1px solid ${config.color}40`,
      }}
    >
      {config.label}
    </span>
  );
}
