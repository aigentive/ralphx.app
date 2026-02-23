/**
 * AuditTrailDialog - Full chronological audit trail for a task
 * Shows state transitions, activity events, reviews, and merge pipeline steps
 * Enterprise-ready: exact timestamps, source badges, expandable content
 */

import { useState, useCallback } from "react";
import { ScrollText, Loader2, ChevronDown, ChevronRight } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useAuditTrail, type AuditEntry } from "@/hooks/useAuditTrail";

// ============================================================================
// Props
// ============================================================================

interface AuditTrailDialogProps {
  taskId: string;
  isOpen: boolean;
  onClose: () => void;
}

// ============================================================================
// Constants
// ============================================================================

const SOURCE_STYLES = {
  review: {
    label: "Review",
    bg: "rgba(34, 197, 94, 0.12)",
    border: "rgba(34, 197, 94, 0.25)",
    color: "rgb(74, 222, 128)",
  },
  activity: {
    label: "Activity",
    bg: "rgba(255, 107, 53, 0.12)",
    border: "rgba(255, 107, 53, 0.25)",
    color: "var(--accent-primary)",
  },
} as const;

const TYPE_STYLES: Record<string, { color: string }> = {
  Approved: { color: "var(--status-success)" },
  "Changes Requested": { color: "var(--status-warning)" },
  Rejected: { color: "var(--status-error)" },
  error: { color: "var(--status-error)" },
  thinking: { color: "var(--text-muted)" },
  tool_call: { color: "var(--accent-primary)" },
  tool_result: { color: "var(--accent-secondary, var(--text-secondary))" },
  text: { color: "var(--text-secondary)" },
};

const CONTENT_TRUNCATE_LENGTH = 200;

// ============================================================================
// Helpers
// ============================================================================

function formatTimestamp(dateString: string): string {
  try {
    const date = new Date(dateString);
    return date.toLocaleString(undefined, {
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

// ============================================================================
// Sub-components
// ============================================================================

function SourceBadge({ source }: { source: AuditEntry["source"] }) {
  const style = SOURCE_STYLES[source];
  return (
    <span
      className="px-1.5 py-0.5 rounded text-[10px] font-medium"
      style={{
        backgroundColor: style.bg,
        border: `1px solid ${style.border}`,
        color: style.color,
      }}
    >
      {style.label}
    </span>
  );
}

function TypeLabel({ type }: { type: string }) {
  const style = TYPE_STYLES[type];
  return (
    <span
      className="text-[11px] font-medium"
      style={{ color: style?.color ?? "var(--text-secondary)" }}
    >
      {type}
    </span>
  );
}

function EntryContent({ text }: { text: string }) {
  const [expanded, setExpanded] = useState(false);
  const needsTruncation = text.length > CONTENT_TRUNCATE_LENGTH;
  const displayText = !expanded && needsTruncation
    ? text.slice(0, CONTENT_TRUNCATE_LENGTH) + "..."
    : text;

  const toggle = useCallback(() => setExpanded((prev) => !prev), []);

  return (
    <div className="mt-1">
      <p
        className="text-[12px] whitespace-pre-wrap break-words"
        style={{ color: "var(--text-secondary)", lineHeight: "1.5" }}
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

function AuditEntryRow({ entry, isLast }: { entry: AuditEntry; isLast: boolean }) {
  return (
    <div className="relative pl-5" style={{ paddingBottom: isLast ? 0 : "12px" }}>
      {/* Vertical connector line */}
      {!isLast && (
        <div
          className="absolute w-0.5"
          style={{
            left: "5px",
            top: "14px",
            bottom: 0,
            backgroundColor: "var(--border-subtle)",
          }}
        />
      )}

      {/* Timeline dot */}
      <div
        className="absolute rounded-full"
        style={{
          left: 0,
          top: "6px",
          width: "12px",
          height: "12px",
          backgroundColor: entry.source === "review"
            ? SOURCE_STYLES.review.color
            : "var(--border-subtle)",
          border: "2px solid var(--bg-elevated)",
        }}
      />

      {/* Timestamp */}
      <div
        className="text-[10px] font-mono tabular-nums"
        style={{ color: "var(--text-muted)" }}
      >
        {formatTimestamp(entry.timestamp)}
      </div>

      {/* Badges row */}
      <div className="flex items-center gap-1.5 mt-1">
        <SourceBadge source={entry.source} />
        <TypeLabel type={entry.type} />
        <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>
          by {entry.actor}
        </span>
      </div>

      {/* Status snapshot */}
      {entry.status && (
        <div className="mt-1">
          <span
            className="px-1.5 py-0.5 rounded text-[10px] font-mono"
            style={{
              backgroundColor: "rgba(255,255,255,0.04)",
              border: "1px solid rgba(255,255,255,0.06)",
              color: "var(--text-muted)",
            }}
          >
            {entry.status}
          </span>
        </div>
      )}

      {/* Description/Content */}
      {entry.description && <EntryContent text={entry.description} />}

      {/* Metadata */}
      {entry.metadata && (
        <p className="text-[10px] mt-0.5 italic" style={{ color: "var(--text-muted)" }}>
          {entry.metadata}
        </p>
      )}
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function AuditTrailDialog({ taskId, isOpen, onClose }: AuditTrailDialogProps) {
  const { entries, isLoading, isEmpty } = useAuditTrail(taskId, { enabled: isOpen });

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-2xl" data-testid="audit-trail-dialog">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <ScrollText className="w-5 h-5 text-[var(--accent-primary)]" />
            <DialogTitle>Audit Trail</DialogTitle>
          </div>
        </DialogHeader>

        <div
          className="px-6 py-4"
          style={{ backgroundColor: "var(--bg-elevated)" }}
        >
          {isLoading && (
            <div className="flex justify-center py-8" data-testid="audit-trail-loading">
              <Loader2
                className="w-6 h-6 animate-spin"
                style={{ color: "var(--text-muted)" }}
              />
            </div>
          )}

          {!isLoading && isEmpty && (
            <div
              className="flex flex-col items-center justify-center py-8 text-center"
              data-testid="audit-trail-empty"
            >
              <ScrollText
                className="w-8 h-8 mb-2"
                style={{ color: "var(--text-muted)", opacity: 0.5 }}
              />
              <p className="text-sm" style={{ color: "var(--text-muted)" }}>
                No audit events recorded yet
              </p>
              <p className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
                State transitions and activity events will appear here
              </p>
            </div>
          )}

          {!isLoading && !isEmpty && (
            <ScrollArea className="max-h-[60vh]">
              <div
                className="p-4 rounded-lg"
                style={{ backgroundColor: "var(--bg-surface)" }}
                data-testid="audit-trail-timeline"
              >
                <div className="relative">
                  {entries.map((entry, index) => (
                    <AuditEntryRow
                      key={entry.id}
                      entry={entry}
                      isLast={index === entries.length - 1}
                    />
                  ))}
                </div>
              </div>
            </ScrollArea>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
