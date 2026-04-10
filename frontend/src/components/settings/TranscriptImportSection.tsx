import { AlertTriangle, CheckCircle2, CircleSlash2, Clock3, DatabaseBackup, FileWarning } from "lucide-react";
import { useChatAttributionBackfillSummary } from "@/hooks/useChatAttributionBackfillSummary";
import { SectionCard } from "./SettingsView.shared";

function getStatusLine(summary: {
  eligibleConversationCount: number;
  runningCount: number;
  remainingCount: number;
  partialCount: number;
  sessionNotFoundCount: number;
  parseFailedCount: number;
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

  if (
    summary.partialCount > 0
    || summary.sessionNotFoundCount > 0
    || summary.parseFailedCount > 0
  ) {
    return "Historical Claude transcript import finished with unresolved sessions.";
  }

  return "Historical Claude transcript import is complete.";
}

function SummaryChip({
  icon,
  label,
  value,
  detail,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  detail: string;
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
      <p className="text-[11px] leading-4 text-[var(--text-muted)]">{detail}</p>
    </div>
  );
}

export function TranscriptImportSection() {
  const { data, isLoading, error } = useChatAttributionBackfillSummary();

  return (
    <SectionCard
      icon={<DatabaseBackup className="w-4 h-4 text-[var(--accent-primary)]" />}
      title="Transcript Import"
      description="Best-effort historical Claude transcript attribution and usage backfill. Updates live while this screen is open."
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
          <p className="text-xs text-[var(--text-muted)]">
            RalphX matches stored legacy <code>claude_session_id</code> values against local Claude JSONL
            transcripts under <code>~/.claude/projects/**</code> and imports attribution and usage only when the
            historical mapping is safe enough to write into the database.
          </p>

          <div className="grid grid-cols-3 gap-3">
            <SummaryChip
              icon={<DatabaseBackup className="w-3.5 h-3.5" />}
              label="Eligible"
              value={data.eligibleConversationCount.toLocaleString("en-US")}
              detail="Legacy RalphX conversations with a stored Claude session id that were checked for transcript import."
            />
            <SummaryChip
              icon={<CheckCircle2 className="w-3.5 h-3.5" />}
              label="Imported"
              value={data.completedCount.toLocaleString("en-US")}
              detail="Conversations where historical attribution and usage imported cleanly."
            />
            <SummaryChip
              icon={<Clock3 className="w-3.5 h-3.5" />}
              label="Remaining"
              value={data.remainingCount.toLocaleString("en-US")}
              detail="Conversations still pending or currently being processed in this pass."
            />
            <SummaryChip
              icon={<AlertTriangle className="w-3.5 h-3.5" />}
              label="Partially Imported"
              value={data.partialCount.toLocaleString("en-US")}
              detail="Transcript was found, but historical messages or runs did not map cleanly enough for a full import."
            />
            <SummaryChip
              icon={<CircleSlash2 className="w-3.5 h-3.5" />}
              label="Transcript Not Found"
              value={data.sessionNotFoundCount.toLocaleString("en-US")}
              detail="RalphX has the stored Claude session id, but the matching JSONL transcript file is not on this machine."
            />
            <SummaryChip
              icon={<FileWarning className="w-3.5 h-3.5" />}
              label="Import Failed"
              value={data.parseFailedCount.toLocaleString("en-US")}
              detail="Transcript file existed, but parsing or import failed."
            />
          </div>

          <div className="space-y-1 text-xs text-[var(--text-muted)]">
            <p>
              Pending: {data.pendingCount} · Running: {data.runningCount} · Idle: {data.isIdle ? "yes" : "no"}
            </p>
            <p>
              Source: <code>~/.claude/projects/**</code>
            </p>
          </div>
        </div>
      )}
    </SectionCard>
  );
}
