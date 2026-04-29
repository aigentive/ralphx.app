import { useMemo, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { FileSearch, ListChecks, ShieldAlert } from "lucide-react";
import { solutionCriticApi } from "@/api/solution-critic";
import type { VerificationGap } from "@/types/ideation";

interface SolutionCritiqueSummaryProps {
  sessionId: string;
  enabled: boolean;
}

const severityOrder: Record<VerificationGap["severity"], number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

const EMPTY_PROJECTED_GAPS: VerificationGap[] = [];

const solutionCriticKeys = {
  latestCompiledContext: (sessionId: string) =>
    ["solutionCritic", sessionId, "latestCompiledContext"] as const,
  latestSolutionCritique: (sessionId: string) =>
    ["solutionCritic", sessionId, "latestSolutionCritique"] as const,
};

function formatCount(count: number, singular: string): string {
  return `${count} ${count === 1 ? singular : `${singular}s`}`;
}

function verdictLabel(verdict: string | undefined): string {
  switch (verdict) {
    case "accept":
      return "Accepted";
    case "revise":
      return "Revise";
    case "investigate":
      return "Investigate";
    case "reject":
      return "Reject";
    default:
      return "No critique";
  }
}

function topProjectedGaps(gaps: VerificationGap[]): VerificationGap[] {
  return [...gaps]
    .sort((left, right) => {
      return (
        severityOrder[left.severity] - severityOrder[right.severity] ||
        left.category.localeCompare(right.category) ||
        left.description.localeCompare(right.description)
      );
    })
    .slice(0, 3);
}

export function SolutionCritiqueSummary({
  sessionId,
  enabled,
}: SolutionCritiqueSummaryProps) {
  const { data: contextData } = useQuery({
    queryKey: solutionCriticKeys.latestCompiledContext(sessionId),
    queryFn: () => solutionCriticApi.getLatestCompiledContext(sessionId),
    enabled,
    staleTime: 30_000,
    retry: false,
  });
  const { data: critiqueData } = useQuery({
    queryKey: solutionCriticKeys.latestSolutionCritique(sessionId),
    queryFn: () => solutionCriticApi.getLatestSolutionCritique(sessionId),
    enabled,
    staleTime: 30_000,
    retry: false,
  });

  const context = contextData?.compiledContext;
  const critique = critiqueData?.solutionCritique;
  const projectedGaps = critiqueData?.projectedGaps ?? EMPTY_PROJECTED_GAPS;
  const visibleGaps = useMemo(() => topProjectedGaps(projectedGaps), [projectedGaps]);

  if (!context && !critique) return null;

  return (
    <section
      data-testid="solution-critique-summary"
      className="rounded-lg p-3 space-y-3"
      style={{
        background: "var(--overlay-faint)",
        border: "1px solid var(--overlay-faint)",
      }}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[11px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
            Solution Critique
          </div>
          <div
            className="mt-1 text-[13px] font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {verdictLabel(critique?.verdict)}
          </div>
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          {critique?.confidence && (
            <span
              className="rounded-md px-2 py-1 text-[10px] font-semibold uppercase"
              style={{
                color: "var(--text-secondary)",
                background: "var(--overlay-weak)",
                border: "1px solid var(--overlay-faint)",
              }}
            >
              {critique.confidence}
            </span>
          )}
          {projectedGaps.length > 0 && (
            <span
              className="rounded-md px-2 py-1 text-[10px] font-semibold uppercase"
              style={{
                color: "var(--status-warning)",
                background: "var(--status-warning-muted)",
                border: "1px solid var(--status-warning-border)",
              }}
            >
              {formatCount(projectedGaps.length, "gap")}
            </span>
          )}
        </div>
      </div>

      <div className="grid gap-2 sm:grid-cols-3">
        <SummaryMetric
          icon={<FileSearch className="w-3.5 h-3.5" />}
          label="Sources"
          value={formatCount(context?.sources.length ?? 0, "source")}
        />
        <SummaryMetric
          icon={<ListChecks className="w-3.5 h-3.5" />}
          label="Claims"
          value={formatCount(context?.claims.length ?? 0, "claim")}
        />
        <SummaryMetric
          icon={<ShieldAlert className="w-3.5 h-3.5" />}
          label="Projected"
          value={formatCount(projectedGaps.length, "gap")}
        />
      </div>

      {critique?.safeNextAction && (
        <p className="text-[11px] leading-relaxed" style={{ color: "var(--text-secondary)" }}>
          {critique.safeNextAction}
        </p>
      )}

      {visibleGaps.length > 0 && (
        <div className="space-y-1.5">
          {visibleGaps.map((gap, index) => (
            <div
              key={`${gap.severity}-${gap.category}-${index}`}
              className="rounded-md px-2.5 py-2"
              style={{
                background: "var(--overlay-weak)",
                border: "1px solid var(--overlay-faint)",
              }}
            >
              <div className="flex items-center gap-2">
                <span
                  className="text-[10px] font-semibold uppercase"
                  style={{ color: "var(--status-warning)" }}
                >
                  {gap.severity}
                </span>
                <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>
                  {gap.category.replace(/_/g, " ")}
                </span>
              </div>
              <div className="mt-1 text-[11px] leading-relaxed" style={{ color: "var(--text-primary)" }}>
                {gap.description}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function SummaryMetric({
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
      className="min-w-0 rounded-md px-2.5 py-2"
      style={{
        background: "var(--overlay-weak)",
        border: "1px solid var(--overlay-faint)",
      }}
    >
      <div className="flex items-center gap-1.5 text-[10px]" style={{ color: "var(--text-muted)" }}>
        {icon}
        <span>{label}</span>
      </div>
      <div className="mt-1 text-[12px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
        {value}
      </div>
    </div>
  );
}
