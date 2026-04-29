import { useMemo, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { FileSearch, ListChecks, ShieldAlert } from "lucide-react";
import {
  solutionCriticApi,
  type CompiledContextReadResponse,
  type SolutionCritiqueReadResponse,
} from "@/api/solution-critic";
import type { VerificationGap } from "@/types/ideation";

interface SolutionCritiqueSummaryProps {
  sessionId: string;
  enabled: boolean;
}

type CompiledContext = CompiledContextReadResponse["compiledContext"];
type SolutionCritique = SolutionCritiqueReadResponse["solutionCritique"];

interface SummaryItem {
  id: string;
  label: string;
  text: string;
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

function formatEnum(value: string): string {
  return value
    .split("_")
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(" ");
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

function contextItems(context: CompiledContext | undefined): SummaryItem[] {
  if (!context) return [];
  return [
    ...context.claims.map((claim) => ({
      id: `claim-${claim.id}`,
      label: formatEnum(claim.classification),
      text: claim.text,
    })),
    ...context.openQuestions.map((question) => ({
      id: `question-${question.id}`,
      label: "Open Question",
      text: question.question,
    })),
    ...context.staleAssumptions.map((assumption) => ({
      id: `assumption-${assumption.id}`,
      label: "Stale Assumption",
      text: assumption.text,
    })),
  ].slice(0, 5);
}

function critiqueItems(critique: SolutionCritique | undefined): SummaryItem[] {
  if (!critique) return [];
  return [
    ...critique.claims
      .filter((claim) => claim.status !== "supported")
      .map((claim) => ({
        id: `claim-${claim.id}`,
        label: formatEnum(claim.status),
        text: claim.claim,
      })),
    ...critique.recommendations
      .filter((recommendation) => recommendation.status !== "accept")
      .map((recommendation) => ({
        id: `recommendation-${recommendation.id}`,
        label: formatEnum(recommendation.status),
        text: recommendation.recommendation,
      })),
    ...critique.risks
      .filter((risk) => risk.severity !== "low")
      .map((risk) => ({
        id: `risk-${risk.id}`,
        label: `${formatEnum(risk.severity)} Risk`,
        text: risk.risk,
      })),
    ...critique.verificationPlan
      .filter((requirement) => requirement.priority !== "low")
      .map((requirement) => ({
        id: `verification-${requirement.id}`,
        label: `${formatEnum(requirement.priority)} Verification`,
        text: requirement.requirement,
      })),
  ].slice(0, 5);
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
  const contextSignals = useMemo(() => contextItems(context), [context]);
  const critiqueSignals = useMemo(() => critiqueItems(critique), [critique]);
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

      <div className="grid gap-2 lg:grid-cols-2">
        <SummarySignalList
          title="Compiled Context"
          items={contextSignals}
          emptyText="No context signals captured yet."
        />
        <SummarySignalList
          title="Critique Signals"
          items={critiqueSignals}
          emptyText="No critique signals captured yet."
        />
      </div>

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

function SummarySignalList({
  title,
  items,
  emptyText,
}: {
  title: string;
  items: SummaryItem[];
  emptyText: string;
}) {
  return (
    <div
      className="min-w-0 rounded-md px-2.5 py-2"
      style={{
        background: "var(--overlay-weak)",
        border: "1px solid var(--overlay-faint)",
      }}
    >
      <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
        {title}
      </div>
      {items.length > 0 ? (
        <div className="mt-2 space-y-2">
          {items.map((item) => (
            <div key={item.id} className="min-w-0">
              <div className="text-[10px] font-semibold" style={{ color: "var(--status-warning)" }}>
                {item.label}
              </div>
              <div className="mt-0.5 text-[11px] leading-relaxed" style={{ color: "var(--text-primary)" }}>
                {item.text}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="mt-2 text-[11px]" style={{ color: "var(--text-muted)" }}>
          {emptyText}
        </div>
      )}
    </div>
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
