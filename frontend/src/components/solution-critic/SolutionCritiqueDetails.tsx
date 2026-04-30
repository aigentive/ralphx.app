import { AlertTriangle, CheckCircle2, ShieldAlert } from "lucide-react";
import type { ReactNode } from "react";
import type {
  CompiledContextReadResponse,
  SolutionCritiqueReadResponse,
} from "@/api/solution-critic";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  critiqueGapOriginLabel,
  formatCritiqueEnum,
  type CritiqueDigest,
} from "./critiqueDigest";

type SolutionCritique = SolutionCritiqueReadResponse["solutionCritique"];
type EvidenceRef = SolutionCritique["claims"][number]["evidence"][number];

interface SolutionCritiqueDetailsProps {
  targetLabel: string;
  context: CompiledContextReadResponse | null;
  result: SolutionCritiqueReadResponse | null;
  digest: CritiqueDigest;
  isLoading: boolean;
  error: string | null;
  onRefresh: () => void;
}

const toneClasses: Record<CritiqueDigest["tone"], string> = {
  neutral: "border-[var(--overlay-weak)] bg-[var(--overlay-faint)] text-text-primary/65",
  success: "border-[var(--status-success-border)] bg-[var(--status-success-muted)] text-[var(--status-success)]",
  warning: "border-[var(--status-warning-border)] bg-[var(--status-warning-muted)] text-[var(--status-warning)]",
  danger: "border-[var(--status-error-border)] bg-[var(--status-error-muted)] text-[var(--status-error)]",
  accent: "border-[var(--accent-primary)] bg-[var(--accent-muted)] text-[var(--accent-primary)]",
};

export function SolutionCritiqueDetails({
  targetLabel,
  context,
  result,
  digest,
  isLoading,
  error,
  onRefresh,
}: SolutionCritiqueDetailsProps) {
  const critique = result?.solutionCritique;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-[var(--overlay-weak)] px-4 py-3">
        <div className="text-[11px] font-semibold uppercase text-text-primary/40">
          Solution Critique
        </div>
        <div className="mt-0.5 truncate text-[14px] font-semibold text-text-primary/90">
          {targetLabel}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <div className="space-y-4">
          <DecisionStrip digest={digest} />

          {isLoading && (
            <div className="rounded-md border border-[var(--overlay-weak)] bg-[var(--overlay-faint)] p-3 text-[12px] text-text-primary/55">
              Critique is running. The details will fill in when the model response is persisted.
            </div>
          )}

          {error && (
            <div className="flex items-start gap-2 rounded-md border border-[var(--status-error-border)] bg-[var(--status-error-muted)] p-3 text-[12px] text-[var(--status-error)]">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {critique ? (
            <>
              {critique.safeNextAction && (
                <SignalBlock title="Safe Next Action">
                  <p className="text-[12px] leading-relaxed text-text-primary/75">
                    {critique.safeNextAction}
                  </p>
                </SignalBlock>
              )}

              <SignalBlock title={`Claims (${critique.claims.length})`}>
                <div className="space-y-2">
                  {orderedClaims(critique).map((claim) => (
                    <SignalCard
                      key={claim.id}
                      label={`${formatCritiqueEnum(claim.status)} - ${formatCritiqueEnum(claim.confidence)}`}
                      tone={claim.status === "supported" ? "success" : "warning"}
                      text={claim.claim}
                      note={claim.notes}
                      evidence={claim.evidence}
                    />
                  ))}
                </div>
              </SignalBlock>

              <SignalBlock title={`Risks (${critique.risks.length})`}>
                {critique.risks.length > 0 ? (
                  <div className="space-y-2">
                    {critique.risks.map((risk) => (
                      <SignalCard
                        key={risk.id}
                        label={`${formatCritiqueEnum(risk.severity)} risk`}
                        tone={risk.severity === "critical" || risk.severity === "high" ? "danger" : "warning"}
                        text={risk.risk}
                        note={risk.mitigation}
                        evidence={risk.evidence}
                      />
                    ))}
                  </div>
                ) : (
                  <EmptyLine>No risks captured.</EmptyLine>
                )}
              </SignalBlock>

              <SignalBlock title={`Recommendations (${critique.recommendations.length})`}>
                {critique.recommendations.length > 0 ? (
                  <div className="space-y-2">
                    {critique.recommendations.map((recommendation) => (
                      <SignalCard
                        key={recommendation.id}
                        label={formatCritiqueEnum(recommendation.status)}
                        tone={recommendation.status === "accept" ? "success" : "warning"}
                        text={recommendation.recommendation}
                        note={recommendation.rationale}
                        evidence={recommendation.evidence}
                      />
                    ))}
                  </div>
                ) : (
                  <EmptyLine>No recommendations captured.</EmptyLine>
                )}
              </SignalBlock>

              <SignalBlock title={`Verification Plan (${critique.verificationPlan.length})`}>
                {critique.verificationPlan.length > 0 ? (
                  <div className="space-y-2">
                    {critique.verificationPlan.map((requirement) => (
                      <SignalCard
                        key={requirement.id}
                        label={`${formatCritiqueEnum(requirement.priority)} priority`}
                        tone={requirement.priority === "critical" || requirement.priority === "high" ? "danger" : "warning"}
                        text={requirement.requirement}
                        note={requirement.suggestedTest}
                        evidence={requirement.evidence}
                      />
                    ))}
                  </div>
                ) : (
                  <EmptyLine>No verification requirements captured.</EmptyLine>
                )}
              </SignalBlock>

              <SignalBlock title={`Projected Gaps (${result.projectedGaps.length})`}>
                {result.projectedGaps.length > 0 ? (
                  <div className="space-y-2">
                    {result.projectedGaps.map((gap, index) => (
                      <div
                        key={`${gap.category}-${gap.severity}-${index}`}
                        className="rounded-md border border-[var(--overlay-weak)] bg-[var(--overlay-faint)] p-2.5"
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="text-[10px] font-semibold uppercase text-[var(--status-warning)]">
                            {gap.severity}
                          </span>
                          <span className="text-[10px] text-text-primary/45">
                            {formatCritiqueEnum(gap.category)}
                          </span>
                          {critiqueGapOriginLabel(gap.category) && (
                            <span className="rounded-md border border-[var(--overlay-weak)] px-1.5 py-0.5 text-[10px] text-text-primary/45">
                              {critiqueGapOriginLabel(gap.category)}
                            </span>
                          )}
                        </div>
                        <div className="mt-1 text-[12px] leading-relaxed text-text-primary/75">
                          {gap.description}
                        </div>
                        {gap.whyItMatters && (
                          <div className="mt-1 text-[11px] leading-relaxed text-text-primary/45">
                            {gap.whyItMatters}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <EmptyLine>No projected gaps.</EmptyLine>
                )}
              </SignalBlock>

              {context && (
                <div className="text-[11px] text-text-primary/40">
                  Context: {context.compiledContext.sources.length} source
                  {context.compiledContext.sources.length === 1 ? "" : "s"}
                </div>
              )}
            </>
          ) : (
            !isLoading && !error && (
              <div className="rounded-md border border-[var(--overlay-weak)] bg-[var(--overlay-faint)] p-3 text-[12px] text-text-primary/55">
                No critique has been generated for this target yet.
              </div>
            )
          )}
        </div>
      </div>

      <div className="flex shrink-0 items-center justify-end gap-2 border-t border-[var(--overlay-weak)] px-4 py-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onRefresh}
          disabled={isLoading}
          className="h-8 rounded-md text-[12px]"
        >
          {critique ? "Refresh critique" : "Run critique"}
        </Button>
      </div>
    </div>
  );
}

function DecisionStrip({ digest }: { digest: CritiqueDigest }) {
  return (
    <div className={cn("rounded-md border p-3", toneClasses[digest.tone])}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[13px] font-semibold">{digest.verdictLabel}</span>
        {digest.confidenceLabel && (
          <span className="text-[11px] opacity-80">{digest.confidenceLabel} confidence</span>
        )}
        {digest.isStale && (
          <span className="rounded-md bg-[var(--overlay-weak)] px-1.5 py-0.5 text-[10px] uppercase text-text-primary/55">
            stale
          </span>
        )}
      </div>
      <div className="mt-2 flex flex-wrap gap-2 text-[11px] opacity-85">
        <span>{digest.claimCount} claims</span>
        <span>{digest.flaggedClaimCount} flagged</span>
        <span>{digest.riskCount} risks</span>
        <span>{digest.projectedGapCount} gaps</span>
      </div>
    </div>
  );
}

function SignalBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-2">
      <div className="text-[11px] font-semibold uppercase text-text-primary/40">
        {title}
      </div>
      {children}
    </section>
  );
}

function SignalCard({
  label,
  tone,
  text,
  note,
  evidence,
}: {
  label: string;
  tone: "success" | "warning" | "danger";
  text: string;
  note?: string | null | undefined;
  evidence: EvidenceRef[];
}) {
  const Icon = tone === "success" ? CheckCircle2 : tone === "danger" ? ShieldAlert : AlertTriangle;
  return (
    <div className="rounded-md border border-[var(--overlay-weak)] bg-[var(--overlay-faint)] p-2.5">
      <div className="flex items-center gap-1.5 text-[10px] font-semibold uppercase text-text-primary/45">
        <Icon className="h-3 w-3" />
        <span>{label}</span>
      </div>
      <div className="mt-1 text-[12px] leading-relaxed text-text-primary/75">
        {text}
      </div>
      {note && (
        <div className="mt-1 text-[11px] leading-relaxed text-text-primary/45">
          {note}
        </div>
      )}
      {evidence.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {evidence.map((source) => (
            <span
              key={source.id}
              className="max-w-full truncate rounded-md border border-[var(--overlay-weak)] bg-[var(--overlay-weak)] px-1.5 py-0.5 text-[10px] text-text-primary/50"
            >
              {source.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function EmptyLine({ children }: { children: ReactNode }) {
  return <div className="text-[12px] text-text-primary/45">{children}</div>;
}

function orderedClaims(critique: SolutionCritique) {
  const order: Record<string, number> = {
    contradicted: 0,
    unsupported: 1,
    unclear: 2,
    supported: 3,
  };
  return [...critique.claims].sort(
    (left, right) =>
      (order[left.status] ?? 9) - (order[right.status] ?? 9) ||
      left.claim.localeCompare(right.claim),
  );
}
