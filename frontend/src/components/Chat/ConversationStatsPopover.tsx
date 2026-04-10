import { BarChart2 } from "lucide-react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { useConversationStats } from "@/hooks/useConversationStats";

interface ConversationStatsPopoverProps {
  conversationId: string | null;
}

function formatInteger(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatUsd(value: number | null): string {
  if (value == null) {
    return "Unavailable";
  }

  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  }).format(value);
}

export function ConversationStatsPopover({
  conversationId,
}: ConversationStatsPopoverProps) {
  const statsQuery = useConversationStats(conversationId);
  const stats = statsQuery.data;

  if (!conversationId) {
    return null;
  }

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className="flex items-center justify-center w-6 h-6 rounded text-white/38 hover:text-white/72 hover:bg-white/5 transition-colors"
          aria-label="Conversation stats"
          data-testid="chat-session-stats-button"
        >
          <BarChart2 className="w-3.5 h-3.5" />
        </button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-80 p-0 border-white/10 bg-[hsl(220_15%_8%_/_0.96)] shadow-xl"
      >
        <div className="p-3 border-b border-white/6">
          <div className="text-sm font-medium text-white/90">Conversation stats</div>
          <div className="text-[11px] text-white/45 mt-1">
            Aggregated from {stats?.usageCoverage.effectiveTotalsSource ?? "available data"}.
          </div>
        </div>

        {statsQuery.isLoading ? (
          <div className="p-3 text-sm text-white/55">Loading conversation stats...</div>
        ) : !stats ? (
          <div className="p-3 text-sm text-white/55">Stats are not available for this conversation.</div>
        ) : (
          <div className="p-3 space-y-3">
            <div className="grid grid-cols-2 gap-2">
              <div className="rounded-md border border-white/6 bg-white/[0.03] p-2">
                <div className="text-[10px] uppercase tracking-[0.08em] text-white/38">Input</div>
                <div className="text-sm text-white/88 mt-1">
                  {formatInteger(stats.effectiveUsageTotals.inputTokens)}
                </div>
              </div>
              <div className="rounded-md border border-white/6 bg-white/[0.03] p-2">
                <div className="text-[10px] uppercase tracking-[0.08em] text-white/38">Output</div>
                <div className="text-sm text-white/88 mt-1">
                  {formatInteger(stats.effectiveUsageTotals.outputTokens)}
                </div>
              </div>
              <div className="rounded-md border border-white/6 bg-white/[0.03] p-2">
                <div className="text-[10px] uppercase tracking-[0.08em] text-white/38">Cache</div>
                <div className="text-sm text-white/88 mt-1">
                  {formatInteger(
                    stats.effectiveUsageTotals.cacheCreationTokens +
                      stats.effectiveUsageTotals.cacheReadTokens,
                  )}
                </div>
              </div>
              <div className="rounded-md border border-white/6 bg-white/[0.03] p-2">
                <div className="text-[10px] uppercase tracking-[0.08em] text-white/38">Est. cost</div>
                <div className="text-sm text-white/88 mt-1">
                  {formatUsd(stats.effectiveUsageTotals.estimatedUsd)}
                </div>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3 text-[11px]">
              <div>
                <div className="uppercase tracking-[0.08em] text-white/38">Coverage</div>
                <div className="mt-1 text-white/70">
                  Messages: {stats.usageCoverage.providerMessagesWithUsage}/
                  {stats.usageCoverage.providerMessageCount}
                </div>
                <div className="text-white/70">
                  Runs: {stats.usageCoverage.runsWithUsage}/{stats.usageCoverage.runCount}
                </div>
              </div>
              <div>
                <div className="uppercase tracking-[0.08em] text-white/38">Attribution</div>
                <div className="mt-1 text-white/70">
                  Messages: {stats.attributionCoverage.providerMessagesWithAttribution}/
                  {stats.attributionCoverage.providerMessageCount}
                </div>
                <div className="text-white/70">
                  Runs: {stats.attributionCoverage.runsWithAttribution}/
                  {stats.attributionCoverage.runCount}
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <div className="uppercase tracking-[0.08em] text-[10px] text-white/38">Top breakdowns</div>
              {stats.byModel[0] && (
                <div className="flex items-center justify-between text-[11px] text-white/72">
                  <span>Model</span>
                  <span className="truncate max-w-[12rem] text-right">{stats.byModel[0].key}</span>
                </div>
              )}
              {stats.byEffort[0] && (
                <div className="flex items-center justify-between text-[11px] text-white/72">
                  <span>Effort</span>
                  <span>{stats.byEffort[0].key}</span>
                </div>
              )}
              {stats.byUpstreamProvider[0] && (
                <div className="flex items-center justify-between text-[11px] text-white/72">
                  <span>Upstream</span>
                  <span>{stats.byUpstreamProvider[0].key}</span>
                </div>
              )}
              {stats.attributionBackfillStatus && (
                <div className="flex items-center justify-between text-[11px] text-white/72">
                  <span>Backfill</span>
                  <span>{stats.attributionBackfillStatus}</span>
                </div>
              )}
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
