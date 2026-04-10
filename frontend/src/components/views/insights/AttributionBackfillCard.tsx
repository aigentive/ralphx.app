import type { ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Clock3, DatabaseBackup } from "lucide-react";
import type { AttributionBackfillSummary } from "@/api/metrics";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";

interface AttributionBackfillCardProps {
  summary: AttributionBackfillSummary;
}

function MiniStat({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div
      className="rounded-lg p-3 flex flex-col gap-1"
      style={{ backgroundColor: "rgba(255,255,255,0.03)" }}
    >
      <div className="flex items-center gap-2 text-[11px]" style={{ color: "rgba(255,255,255,0.42)" }}>
        {icon}
        <span className="uppercase tracking-[0.08em]">{label}</span>
      </div>
      <div className="text-[15px] font-medium" style={{ color: "rgba(255,255,255,0.88)" }}>
        {value}
      </div>
    </div>
  );
}

function getStatusLine(summary: AttributionBackfillSummary): string {
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

export function AttributionBackfillCard({ summary }: AttributionBackfillCardProps) {
  return (
    <DetailCard>
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2">
          <DatabaseBackup className="w-4 h-4" style={{ color: "#ff6b35" }} />
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium" style={{ color: "rgba(255,255,255,0.88)" }}>
              Historical Transcript Import
            </span>
            <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
              {getStatusLine(summary)}
            </span>
          </div>
        </div>

        <div className="grid grid-cols-2 min-[800px]:grid-cols-4 gap-3">
          <MiniStat
            icon={<DatabaseBackup className="w-3.5 h-3.5" />}
            label="Eligible"
            value={summary.eligibleConversationCount.toLocaleString("en-US")}
          />
          <MiniStat
            icon={<Clock3 className="w-3.5 h-3.5" />}
            label="Remaining"
            value={summary.remainingCount.toLocaleString("en-US")}
          />
          <MiniStat
            icon={<CheckCircle2 className="w-3.5 h-3.5" />}
            label="Imported"
            value={summary.completedCount.toLocaleString("en-US")}
          />
          <MiniStat
            icon={<AlertTriangle className="w-3.5 h-3.5" />}
            label="Attention"
            value={summary.attentionCount.toLocaleString("en-US")}
          />
        </div>

        <div className="grid grid-cols-1 min-[800px]:grid-cols-2 gap-3 text-[12px]">
          <div className="flex flex-col gap-1.5">
            <div style={{ color: "rgba(255,255,255,0.42)" }} className="uppercase tracking-[0.08em] text-[10px]">
              State breakdown
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Pending: {summary.pendingCount} · Running: {summary.runningCount}
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Completed: {summary.completedCount} · Partial: {summary.partialCount}
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Not found: {summary.sessionNotFoundCount} · Parse failed: {summary.parseFailedCount}
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <div style={{ color: "rgba(255,255,255,0.42)" }} className="uppercase tracking-[0.08em] text-[10px]">
              Source
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Startup backfill scans surviving Claude transcripts under
              {" "}
              <code className="text-[11px]">~/.claude/projects/**</code>.
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Idle: {summary.isIdle ? "yes" : "no"} · Terminal states: {summary.terminalCount}
            </div>
          </div>
        </div>
      </div>
    </DetailCard>
  );
}
