import { useMemo, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileSearch, ListChecks, SearchCheck, ShieldAlert } from "lucide-react";
import {
  solutionCriticApi,
  type CompiledContextReadResponse,
  type SolutionCritiqueReadResponse,
  type SolutionCritiqueTargetInput,
} from "@/api/solution-critic";
import { Button } from "@/components/ui/button";
import {
  buildCritiqueDigest,
  critiqueGapOriginLabel,
  formatCritiqueEnum,
} from "@/components/solution-critic/critiqueDigest";
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
  rollup: (sessionId: string) =>
    ["solutionCritic", sessionId, "rollup"] as const,
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
      return "Critique pending";
  }
}

function targetScopeLabel(targetType: string | undefined): string {
  switch (targetType) {
    case "plan_artifact":
      return "Plan";
    case "chat_message":
      return "Message";
    case "task_execution":
      return "Task Execution";
    case "review_report":
      return "Review";
    case "agent_run":
      return "Agent Run";
    case "task":
      return "Task";
    case "artifact":
      return "Artifact";
    default:
      return "Target";
  }
}

function sameTarget(
  left: { targetType: string; id: string } | undefined,
  right: { targetType: string; id: string } | undefined,
): boolean {
  return Boolean(left && right && left.targetType === right.targetType && left.id === right.id);
}

function shortTargetId(id: string | undefined): string | null {
  if (!id) return null;
  const lastPart = id.split(":").filter(Boolean).pop() ?? id;
  return lastPart.length > 12 ? lastPart.slice(-12) : lastPart;
}

function targetScopeDescription(
  target: { targetType: string; id: string; label?: string } | undefined,
): string {
  const shortId = shortTargetId(target?.id);
  let label: string;
  switch (target?.targetType) {
    case "plan_artifact":
      label = "Plan artifact";
      break;
    case "chat_message":
      label = "Assistant response";
      break;
    case "task_execution":
      label = "Task execution";
      break;
    case "review_report":
      label = "Review report";
      break;
    case "agent_run":
      label = "Agent run";
      break;
    case "task":
      label = "Task";
      break;
    case "artifact":
      label = target.label ?? "Artifact";
      break;
    default:
      label = target?.label ?? "Target";
      break;
  }
  return shortId ? `${label} · ${shortId}` : label;
}

function contextTargetToInput(target: CompiledContext["target"]): SolutionCritiqueTargetInput {
  return {
    targetType: target.targetType as SolutionCritiqueTargetInput["targetType"],
    id: target.id,
    ...(target.label ? { label: target.label } : {}),
  };
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
      label: formatCritiqueEnum(claim.classification),
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
        label: formatCritiqueEnum(claim.status),
        text: claim.claim,
      })),
    ...critique.recommendations
      .filter((recommendation) => recommendation.status !== "accept")
      .map((recommendation) => ({
        id: `recommendation-${recommendation.id}`,
        label: formatCritiqueEnum(recommendation.status),
        text: recommendation.recommendation,
      })),
    ...critique.risks
      .filter((risk) => risk.severity !== "low")
      .map((risk) => ({
        id: `risk-${risk.id}`,
        label: `${formatCritiqueEnum(risk.severity)} Risk`,
        text: risk.risk,
      })),
    ...critique.verificationPlan
      .filter((requirement) => requirement.priority !== "low")
      .map((requirement) => ({
        id: `verification-${requirement.id}`,
        label: `${formatCritiqueEnum(requirement.priority)} Verification`,
        text: requirement.requirement,
      })),
  ].slice(0, 5);
}

export function SolutionCritiqueSummary({
  sessionId,
  enabled,
}: SolutionCritiqueSummaryProps) {
  const queryClient = useQueryClient();
  const { data: contextData } = useQuery({
    queryKey: solutionCriticKeys.latestCompiledContext(sessionId),
    queryFn: () => solutionCriticApi.getLatestCompiledContext(sessionId),
    enabled,
    staleTime: 30_000,
    retry: false,
  });
  const hasCompiledContext = Boolean(contextData?.compiledContext);
  const { data: critiqueData } = useQuery({
    queryKey: solutionCriticKeys.latestSolutionCritique(sessionId),
    queryFn: () => solutionCriticApi.getLatestSolutionCritique(sessionId),
    enabled,
    staleTime: 30_000,
    refetchInterval: hasCompiledContext
      ? (query) => (query.state.data ? false : 1_000)
      : false,
    retry: false,
  });
  const { data: rollupData } = useQuery({
    queryKey: solutionCriticKeys.rollup(sessionId),
    queryFn: () => solutionCriticApi.getSolutionCritiqueRollup(sessionId),
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
  const hasCritique = Boolean(critique);
  const targetScope = targetScopeLabel(context?.target.targetType);
  const targetDescription = targetScopeDescription(context?.target);
  const latestOtherTarget = useMemo(() => {
    if (!context) return null;
    return [...(rollupData?.targets ?? [])]
      .filter((item) => !sameTarget(item.target, context.target))
      .sort((left, right) => Date.parse(right.generatedAt) - Date.parse(left.generatedAt))[0] ?? null;
  }, [context, rollupData?.targets]);
  const showPlanCritiqueMissing =
    !hasCritique &&
    context?.target.targetType === "plan_artifact" &&
    latestOtherTarget != null;
  const digest = buildCritiqueDigest({
    context: contextData ?? null,
    result: critiqueData ?? null,
    isLoading: false,
    error: null,
  });
  const planCritiqueMutation = useMutation({
    mutationFn: async () => {
      if (!context) throw new Error("No compiled plan context is available.");
      const target = contextTargetToInput(context.target);
      const compiledContext = await solutionCriticApi.compileTargetContext(sessionId, target);
      const solutionCritique = await solutionCriticApi.critiqueTarget(
        sessionId,
        target,
        compiledContext.artifactId,
      );
      return { compiledContext, solutionCritique };
    },
    onSuccess: (response) => {
      queryClient.setQueryData(
        solutionCriticKeys.latestCompiledContext(sessionId),
        response.compiledContext,
      );
      queryClient.setQueryData(
        solutionCriticKeys.latestSolutionCritique(sessionId),
        response.solutionCritique,
      );
      void queryClient.invalidateQueries({ queryKey: solutionCriticKeys.rollup(sessionId) });
    },
  });

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
            {hasCritique ? `${targetScope} Solution Critique` : `${targetScope} Critique Pending`}
          </div>
          <div
            className="mt-1 text-[13px] font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
          >
          {hasCritique ? verdictLabel(critique?.verdict) : `Compiled ${targetScope.toLowerCase()} context ready`}
          </div>
          <div className="mt-1 text-[11px]" style={{ color: "var(--text-muted)" }}>
            Target: {targetDescription}
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

      {hasCritique && (
        <div
          className="rounded-md px-2.5 py-2"
          style={{
            background: "var(--overlay-weak)",
            border: "1px solid var(--overlay-faint)",
          }}
        >
          <div className="flex flex-wrap items-center gap-2 text-[11px]">
            <span className="font-semibold" style={{ color: "var(--text-primary)" }}>
              {digest.verdictLabel}
            </span>
            {digest.confidenceLabel && (
              <span style={{ color: "var(--text-secondary)" }}>
                {digest.confidenceLabel} confidence
              </span>
            )}
            {digest.isStale && (
              <span
                className="rounded-md px-1.5 py-0.5 text-[10px] font-semibold uppercase"
                style={{
                  background: "var(--status-warning-muted)",
                  color: "var(--status-warning)",
                  border: "1px solid var(--status-warning-border)",
                }}
              >
                stale
              </span>
            )}
          </div>
          <div className="mt-1 flex flex-wrap gap-2 text-[10px]" style={{ color: "var(--text-muted)" }}>
            <span>{formatCount(digest.flaggedClaimCount, "flagged claim")}</span>
            <span>{formatCount(digest.riskCount, "risk")}</span>
            <span>{formatCount(digest.projectedGapCount, "gap")}</span>
          </div>
        </div>
      )}

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
      {!critique && context && (
        <p className="text-[11px] leading-relaxed" style={{ color: "var(--text-secondary)" }}>
          Model critique has not been persisted for this context yet.
        </p>
      )}

      {showPlanCritiqueMissing && latestOtherTarget && (
        <div
          data-testid="plan-critique-missing-row"
          className="flex flex-col gap-2 rounded-md px-2.5 py-2 sm:flex-row sm:items-center sm:justify-between"
          style={{
            background: "var(--overlay-weak)",
            border: "1px solid var(--overlay-faint)",
          }}
        >
          <div className="min-w-0">
            <div className="text-[11px] font-semibold" style={{ color: "var(--text-primary)" }}>
              Plan critique not yet run
            </div>
            <div className="mt-0.5 text-[11px]" style={{ color: "var(--text-muted)" }}>
              Latest saved critique is for {targetScopeDescription(latestOtherTarget.target)}.
            </div>
            {planCritiqueMutation.error instanceof Error && (
              <div className="mt-1 text-[11px]" style={{ color: "var(--status-danger)" }}>
                {planCritiqueMutation.error.message}
              </div>
            )}
          </div>
          <Button
            type="button"
            size="sm"
            onClick={() => planCritiqueMutation.mutate()}
            disabled={planCritiqueMutation.isPending}
            className="h-7 px-2.5 text-[11px] font-semibold gap-1.5 rounded-lg shrink-0"
            style={{
              color: "var(--accent-primary)",
              background: "var(--overlay-faint)",
              border: "1px solid var(--accent-border)",
            }}
          >
            <SearchCheck className="w-3 h-3" />
            {planCritiqueMutation.isPending ? "Critiquing plan" : "Critique plan"}
          </Button>
        </div>
      )}

      <div className="grid gap-2 lg:grid-cols-2">
        <SummarySignalList
          title="Compiled Context"
          items={contextSignals}
          emptyText="No context signals captured yet."
        />
        <SummarySignalList
          title={hasCritique ? "Critique Signals" : "Model Critique"}
          items={critiqueSignals}
          emptyText={
            hasCritique
              ? "No critique signals captured yet."
              : "No LLM critique persisted yet."
          }
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
                  {formatCritiqueEnum(gap.category)}
                </span>
                {critiqueGapOriginLabel(gap.category) && (
                  <span
                    className="rounded-md px-1.5 py-0.5 text-[10px]"
                    style={{
                      color: "var(--text-muted)",
                      background: "var(--overlay-faint)",
                      border: "1px solid var(--overlay-faint)",
                    }}
                  >
                    {critiqueGapOriginLabel(gap.category)}
                  </span>
                )}
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
