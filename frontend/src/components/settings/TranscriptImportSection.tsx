import { AlertTriangle, CheckCircle2, Clock3, DatabaseBackup } from "lucide-react";
import { useChatAttributionBackfillSummary } from "@/hooks/useChatAttributionBackfillSummary";
import { SectionCard } from "./SettingsView.shared";

function getStatusLine(summary: {
  eligibleConversationCount: number;
  runningCount: number;
  remainingCount: number;
  attentionCount: number;
}): string {
  if (summary.eligibleConversationCount === 0) {
    return "No legacy Claude transcript sessions detected on this system.";
  }

  if (summary.runningCount > 0) {
    return "Historical Claude transcript import is running in the background.";
  }

  if (summary.remainingCount > 0) {
    return "Historical Claude transcript import still has pending work.";
  }

  if (summary.attentionCount > 0) {
    return "Historical Claude transcript import finished with sessions that need attention.";
  }

  return "Historical Claude transcript import is complete.";
}

function SummaryChip({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div
      className="rounded-md px-3 py-2 flex flex-col gap-1"
      style={{ backgroundColor: "rgba(255,255,255,0.03)" }}
    >
      <div className="flex items-center gap-2 text-[11px]" style={{ color: "rgba(255,255,255,0.42)" }}>
        {icon}
        <span className="uppercase tracking-[0.08em]">{label}</span>
      </div>
      <div className="text-sm font-medium text-[var(--text-primary)]">{value}</div>
    </div>
  );
}

export function TranscriptImportSection() {
  const { data, isLoading, error } = useChatAttributionBackfillSummary();

  return (
    <SectionCard
      icon={<DatabaseBackup className="w-4 h-4 text-[var(--accent-primary)]" />}
      title="Transcript Import"
      description="Best-effort historical Claude transcript attribution and usage backfill."
    >
      {isLoading && (
        <p className="text-sm text-[var(--text-muted)]">
          Loading transcript import status...
        </p>
      )}

      {!isLoading && error && (
        <p className="text-sm text-[var(--text-danger)]">
          Failed to load transcript import status.
        </p>
      )}

      {!isLoading && !error && data && (
        <div className="space-y-3" data-testid="transcript-import-section">
          <p className="text-sm text-[var(--text-primary)]">{getStatusLine(data)}</p>

          <div className="grid grid-cols-2 gap-3">
            <SummaryChip
              icon={<DatabaseBackup className="w-3.5 h-3.5" />}
              label="Eligible"
              value={data.eligibleConversationCount.toLocaleString("en-US")}
            />
            <SummaryChip
              icon={<Clock3 className="w-3.5 h-3.5" />}
              label="Remaining"
              value={data.remainingCount.toLocaleString("en-US")}
            />
            <SummaryChip
              icon={<CheckCircle2 className="w-3.5 h-3.5" />}
              label="Imported"
              value={data.completedCount.toLocaleString("en-US")}
            />
            <SummaryChip
              icon={<AlertTriangle className="w-3.5 h-3.5" />}
              label="Attention"
              value={data.attentionCount.toLocaleString("en-US")}
            />
          </div>

          <div className="space-y-1 text-xs text-[var(--text-muted)]">
            <p>
              Pending: {data.pendingCount} · Running: {data.runningCount} · Partial: {data.partialCount}
            </p>
            <p>
              Not found: {data.sessionNotFoundCount} · Parse failed: {data.parseFailedCount}
            </p>
            <p>
              Source: <code>~/.claude/projects/**</code> · Idle: {data.isIdle ? "yes" : "no"}
            </p>
          </div>
        </div>
      )}
    </SectionCard>
  );
}
