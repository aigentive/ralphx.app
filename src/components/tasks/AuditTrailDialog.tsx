/**
 * AuditTrailDialog - Full chronological audit trail for a task
 * Near full-screen modal matching ReviewDetailModal's premium glass style.
 * Shows state transitions, activity events, reviews, and merge pipeline steps.
 */

import { useState, useCallback, useMemo } from "react";
import {
  ScrollText,
  X,
  Loader2,
  CheckCircle2,
  RotateCcw,
  AlertCircle,
  MessageSquare,
  Terminal,
  Brain,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
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

const TYPE_ICONS: Record<string, { icon: typeof CheckCircle2; color: string }> = {
  Approved: { icon: CheckCircle2, color: "var(--status-success)" },
  "Changes Requested": { icon: RotateCcw, color: "var(--status-warning)" },
  Rejected: { icon: X, color: "var(--status-error)" },
  text: { icon: MessageSquare, color: "var(--text-muted)" },
  tool_call: { icon: Terminal, color: "var(--accent-primary)" },
  tool_result: { icon: Terminal, color: "var(--text-secondary)" },
  thinking: { icon: Brain, color: "var(--text-muted)" },
  error: { icon: AlertCircle, color: "var(--status-error)" },
};

const DEFAULT_ICON = { icon: MessageSquare, color: "var(--text-muted)" };

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

function formatDateRange(entries: AuditEntry[]): string {
  if (entries.length === 0) return "";
  const first = new Date(entries[0].timestamp);
  const last = new Date(entries[entries.length - 1].timestamp);
  const fmt = (d: Date) =>
    d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  if (first.toDateString() === last.toDateString()) return fmt(first);
  return `${fmt(first)} \u2014 ${fmt(last)}`;
}

// ============================================================================
// Sub-components
// ============================================================================

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h4
      className="text-[11px] font-semibold uppercase tracking-wider mb-2"
      style={{ color: "var(--text-muted)" }}
    >
      {children}
    </h4>
  );
}

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

function EntryIcon({ type }: { type: string }) {
  const config = TYPE_ICONS[type] ?? DEFAULT_ICON;
  const Icon = config.icon;
  return (
    <div
      className="flex items-center justify-center w-7 h-7 rounded-full shrink-0 mt-0.5"
      style={{ backgroundColor: "rgba(255,255,255,0.04)" }}
    >
      <Icon className="w-3.5 h-3.5" style={{ color: config.color }} />
    </div>
  );
}

function EntryContent({ text }: { text: string }) {
  const [expanded, setExpanded] = useState(false);
  const needsTruncation = text.length > CONTENT_TRUNCATE_LENGTH;
  const displayText =
    !expanded && needsTruncation
      ? text.slice(0, CONTENT_TRUNCATE_LENGTH) + "..."
      : text;

  const toggle = useCallback(() => setExpanded((prev) => !prev), []);

  return (
    <div className="mt-1">
      <p
        className="text-[12px] whitespace-pre-wrap break-words"
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

function SummarySection({ entries }: { entries: AuditEntry[] }) {
  const dateRange = useMemo(() => formatDateRange(entries), [entries]);
  const reviewCount = entries.filter((e) => e.source === "review").length;
  const activityCount = entries.filter((e) => e.source === "activity").length;

  return (
    <div>
      <SectionTitle>Summary</SectionTitle>
      <div
        className="flex items-center gap-4 py-2.5 px-3 rounded"
        style={{
          backgroundColor: "rgba(0,0,0,0.15)",
          border: "1px solid rgba(255,255,255,0.05)",
        }}
      >
        <div className="text-[12px]">
          <span className="text-white/90 font-medium">{entries.length}</span>
          <span className="text-white/50 ml-1">events</span>
        </div>
        {reviewCount > 0 && (
          <div className="text-[12px]">
            <span className="text-white/90 font-medium">{reviewCount}</span>
            <span className="text-white/50 ml-1">reviews</span>
          </div>
        )}
        {activityCount > 0 && (
          <div className="text-[12px]">
            <span className="text-white/90 font-medium">{activityCount}</span>
            <span className="text-white/50 ml-1">activity</span>
          </div>
        )}
        {dateRange && (
          <div className="text-[11px] text-white/40 ml-auto">{dateRange}</div>
        )}
      </div>
    </div>
  );
}

function AuditEntryCard({ entry }: { entry: AuditEntry }) {
  return (
    <div
      className="flex items-start gap-2 py-2 px-3 rounded"
      style={{
        backgroundColor: "rgba(0,0,0,0.15)",
        border: "1px solid rgba(255,255,255,0.05)",
      }}
    >
      <EntryIcon type={entry.type} />

      <div className="flex-1 min-w-0">
        {/* Top row: type + source badge + timestamp */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span
            className="text-[11px] font-medium"
            style={{ color: (TYPE_ICONS[entry.type] ?? DEFAULT_ICON).color }}
          >
            {entry.type}
          </span>
          <SourceBadge source={entry.source} />
          <span className="text-[11px] text-white/40 ml-auto shrink-0">
            {formatTimestamp(entry.timestamp)}
          </span>
        </div>

        {/* Actor */}
        <div className="text-[11px] text-white/50 mt-0.5">
          by {entry.actor}
        </div>

        {/* Description (expandable) */}
        {entry.description && <EntryContent text={entry.description} />}

        {/* Status snapshot badge */}
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

        {/* Metadata */}
        {entry.metadata && (
          <p
            className="text-[10px] mt-0.5 italic"
            style={{ color: "var(--text-muted)" }}
          >
            {entry.metadata}
          </p>
        )}
      </div>
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
      <DialogContent
        data-testid="audit-trail-dialog"
        hideCloseButton
        className="p-0 gap-0 overflow-hidden flex flex-col max-w-[95vw] w-[95vw] h-[95vh]"
        style={{
          backgroundColor: "var(--bg-surface)",
          border: "1px solid rgba(255,255,255,0.08)",
        }}
      >
        {/* Glass Header */}
        <div
          className="flex items-center justify-between px-4 py-3 border-b shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
          }}
        >
          <div className="flex items-center gap-3">
            <ScrollText
              className="w-5 h-5"
              style={{ color: "var(--accent-primary)" }}
            />
            <DialogTitle
              className="text-base font-semibold text-white/90 tracking-normal"
              style={{ letterSpacing: "-0.02em" }}
            >
              Audit Trail
            </DialogTitle>
          </div>
          <Button
            data-testid="dialog-close"
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="w-8 h-8 text-white/50 hover:text-white/80 hover:bg-white/10"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto min-h-0">
          {isLoading && (
            <div
              className="flex justify-center py-16"
              data-testid="audit-trail-loading"
            >
              <Loader2
                className="w-6 h-6 animate-spin"
                style={{ color: "var(--text-muted)" }}
              />
            </div>
          )}

          {!isLoading && isEmpty && (
            <div
              className="flex flex-col items-center justify-center py-16 text-center"
              data-testid="audit-trail-empty"
            >
              <ScrollText
                className="w-8 h-8 mb-2"
                style={{ color: "var(--text-muted)", opacity: 0.5 }}
              />
              <p className="text-sm text-white/50">
                No audit events recorded yet
              </p>
              <p className="text-xs mt-1 text-white/30">
                State transitions and activity events will appear here
              </p>
            </div>
          )}

          {!isLoading && !isEmpty && (
            <div className="p-4 space-y-5">
              <SummarySection entries={entries} />
              <div>
                <SectionTitle>Timeline</SectionTitle>
                <div
                  data-testid="audit-trail-timeline"
                  className="space-y-2"
                >
                  {entries.map((entry) => (
                    <AuditEntryCard key={entry.id} entry={entry} />
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Glass Footer */}
        <div
          className="flex items-center justify-between px-4 py-3 border-t shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
          }}
        >
          <span className="text-[12px] text-white/50">
            {entries.length} {entries.length === 1 ? "event" : "events"}
          </span>
          <Button
            variant="ghost"
            onClick={onClose}
            className="text-[13px] text-white/60 hover:text-white/80 hover:bg-white/10"
          >
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
