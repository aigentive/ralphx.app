import type { ReactNode } from "react";
import { BarChart2, Boxes, Coins, Cpu, DatabaseZap } from "lucide-react";
import type { ScopeUsageStats } from "@/api/metrics";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";

interface UsageInsightsCardProps {
  stats: ScopeUsageStats;
}

function formatInt(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatUsd(value: number | null): string {
  if (value == null) {
    return "—";
  }

  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  }).format(value);
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

export function UsageInsightsCard({ stats }: UsageInsightsCardProps) {
  const topHarness = stats.byHarness[0]?.key ?? "—";
  const topModel = stats.byModel[0]?.key ?? "—";
  const topProvider = stats.byUpstreamProvider[0]?.key ?? "—";

  return (
    <DetailCard>
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2">
          <BarChart2 className="w-4 h-4" style={{ color: "var(--accent-primary)" }} />
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium" style={{ color: "rgba(255,255,255,0.88)" }}>
              AI Usage
            </span>
            <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
              Aggregated from {stats.usageCoverage.effectiveTotalsSource}
            </span>
          </div>
        </div>

        <div className="grid grid-cols-2 min-[800px]:grid-cols-4 gap-3">
          <MiniStat
            icon={<Cpu className="w-3.5 h-3.5" />}
            label="Input"
            value={formatInt(stats.effectiveUsageTotals.inputTokens)}
          />
          <MiniStat
            icon={<BarChart2 className="w-3.5 h-3.5" />}
            label="Output"
            value={formatInt(stats.effectiveUsageTotals.outputTokens)}
          />
          <MiniStat
            icon={<DatabaseZap className="w-3.5 h-3.5" />}
            label="Cache"
            value={formatInt(
              stats.effectiveUsageTotals.cacheCreationTokens +
                stats.effectiveUsageTotals.cacheReadTokens,
            )}
          />
          <MiniStat
            icon={<Coins className="w-3.5 h-3.5" />}
            label="Est. cost"
            value={formatUsd(stats.effectiveUsageTotals.estimatedUsd)}
          />
        </div>

        <div className="grid grid-cols-1 min-[800px]:grid-cols-2 gap-3 text-[12px]">
          <div className="flex flex-col gap-1.5">
            <div style={{ color: "rgba(255,255,255,0.42)" }} className="uppercase tracking-[0.08em] text-[10px]">
              Coverage
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Messages with usage: {stats.usageCoverage.providerMessagesWithUsage}/
              {stats.usageCoverage.providerMessageCount}
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Runs with usage: {stats.usageCoverage.runsWithUsage}/{stats.usageCoverage.runCount}
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>
              Conversations: {stats.conversationCount}
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <div style={{ color: "rgba(255,255,255,0.42)" }} className="uppercase tracking-[0.08em] text-[10px]">
              Dominant breakdowns
            </div>
            <div className="flex items-center gap-2" style={{ color: "rgba(255,255,255,0.72)" }}>
              <Boxes className="w-3.5 h-3.5 shrink-0" />
              <span>Harness: {topHarness}</span>
            </div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>Provider: {topProvider}</div>
            <div style={{ color: "rgba(255,255,255,0.72)" }}>Model: {topModel}</div>
          </div>
        </div>
      </div>
    </DetailCard>
  );
}
