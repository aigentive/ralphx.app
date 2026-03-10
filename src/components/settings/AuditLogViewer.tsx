/**
 * AuditLogViewer - Per-key request history table.
 *
 * Shows tool_name, timestamp, success/fail status, and latency_ms
 * for each audit log entry from /api/auth/keys/:id/audit.
 */

import { useApiKeyAuditLog } from "@/hooks/useApiKeys";
import { CheckCircle2, XCircle, Clock, AlertCircle } from "lucide-react";

// ============================================================================
// Props
// ============================================================================

export interface AuditLogViewerProps {
  keyId: string;
}

// ============================================================================
// Helpers
// ============================================================================

function formatTimestamp(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatLatency(ms: number | null): string {
  if (ms === null) return "—";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

// ============================================================================
// Component
// ============================================================================

export function AuditLogViewer({ keyId }: AuditLogViewerProps) {
  const { data: entries = [], isLoading, error } = useApiKeyAuditLog(keyId);

  if (isLoading) {
    return (
      <div className="py-3 flex items-center justify-center gap-2">
        <div className="w-3.5 h-3.5 border-2 border-[var(--accent-primary)] border-t-transparent rounded-full animate-spin" />
        <span className="text-xs text-[var(--text-muted)]">Loading audit log…</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-red-400 py-2">
        <AlertCircle className="w-3.5 h-3.5 shrink-0" />
        {error.message}
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <p className="text-xs text-[var(--text-muted)] italic py-2">
        No requests logged yet
      </p>
    );
  }

  return (
    <div
      className="overflow-x-auto max-h-48 overflow-y-auto"
      data-testid="audit-log-viewer"
    >
      <table className="w-full text-xs">
        <thead>
          <tr className="text-[var(--text-muted)] border-b border-[var(--border-subtle)]">
            <th className="text-left py-1.5 pr-3 font-medium">Tool</th>
            <th className="text-left py-1.5 pr-3 font-medium">When</th>
            <th className="text-left py-1.5 pr-3 font-medium">Status</th>
            <th className="text-right py-1.5 font-medium">Latency</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => {
            const success = entry.success;
            return (
              <tr
                key={entry.id}
                data-testid={`audit-entry-${entry.id}`}
                className="border-b border-[var(--border-subtle)] last:border-0 hover:bg-[var(--bg-surface-hover)] transition-colors"
              >
                {/* Tool name */}
                <td className="py-1.5 pr-3 font-mono text-[var(--text-primary)] truncate max-w-[160px]">
                  {entry.tool_name}
                </td>

                {/* Timestamp */}
                <td className="py-1.5 pr-3 text-[var(--text-muted)] whitespace-nowrap">
                  {formatTimestamp(entry.created_at)}
                </td>

                {/* Success / fail */}
                <td className="py-1.5 pr-3">
                  <span
                    className={[
                      "flex items-center gap-1",
                      success ? "text-green-400" : "text-red-400",
                    ].join(" ")}
                  >
                    {success ? (
                      <>
                        <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
                        OK
                      </>
                    ) : (
                      <>
                        <XCircle className="w-3.5 h-3.5 shrink-0" />
                        Failed
                      </>
                    )}
                  </span>
                </td>

                {/* Latency */}
                <td className="py-1.5 text-right text-[var(--text-muted)]">
                  <span className="flex items-center justify-end gap-0.5">
                    <Clock className="w-3 h-3 shrink-0" />
                    {formatLatency(entry.latency_ms)}
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
