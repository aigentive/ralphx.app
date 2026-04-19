/**
 * VerificationHistory — round-by-round gap evolution display.
 *
 * Shows gap score trend across verification rounds and lists
 * the current/final gaps with severity breakdown.
 */

import { TrendingDown, TrendingUp, Minus, CheckCircle2, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import { withAlpha } from "@/lib/theme-colors";
import type {
  RoundSummary,
  VerificationGap,
  VerificationRoundDetail,
  VerificationStatus,
} from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

export interface VerificationHistoryProps {
  /** Round summaries for the score trend timeline */
  rounds: RoundSummary[];
  /** Full round detail snapshots when the backend provides them */
  roundDetails?: VerificationRoundDetail[];
  /** Final/current gaps for the last round */
  currentGaps?: VerificationGap[];
  /** Gap score for the current/final round */
  gapScore?: number;
  /** Terminal status when verification is complete */
  status?: VerificationStatus;
  /** Convergence reason code from backend */
  convergenceReason?: string;
}

// ============================================================================
// Helpers
// ============================================================================

const CONVERGENCE_LABELS: Record<string, string> = {
  zero_blocking: "No blocking gaps remain",
  jaccard_converged: "Gap list stabilized across rounds",
  max_rounds: "Maximum verification rounds reached",
  critic_parse_failure: "Critic output could not be parsed",
  user_skipped: "Manually skipped by user",
  user_reverted: "Plan reverted to original version",
};

const SEVERITY_CONFIG = {
  critical: { color: "var(--status-error)", bg: "var(--status-error-muted)", label: "Critical" },
  high: { color: "var(--accent-primary)", bg: withAlpha("var(--accent-primary)", 10), label: "High" },
  medium: { color: "var(--status-warning)", bg: "var(--status-warning-muted)", label: "Medium" },
  low: { color: "var(--text-secondary)", bg: "var(--overlay-weak)", label: "Low" },
} as const;

const SEVERITY_ORDER = ["critical", "high", "medium", "low"] as const;

function groupGapsBySeverity(gaps: VerificationGap[]): Record<string, VerificationGap[]> {
  const grouped: Record<string, VerificationGap[]> = {};
  for (const gap of gaps) {
    if (!grouped[gap.severity]) grouped[gap.severity] = [];
    grouped[gap.severity]!.push(gap);
  }
  return grouped;
}

function gapKey(gap: VerificationGap): string {
  return `${gap.severity}::${gap.category}::${gap.description}`;
}

// ============================================================================
// Sub-components
// ============================================================================

function RoundTimeline({ rounds }: { rounds: RoundSummary[] }) {
  if (rounds.length === 0) return null;

  const maxScore = Math.max(...rounds.map((r) => r.gapScore), 1);

  return (
    <div className="mb-5">
      <div
        className="text-[11px] font-semibold uppercase tracking-wider mb-3"
        style={{ color: "var(--text-muted)" }}
      >
        Gap Score by Round
      </div>
      <div className="flex items-end gap-2">
        {rounds.map((round, idx) => {
          const prevScore = idx > 0 ? rounds[idx - 1]!.gapScore : round.gapScore;
          const delta = round.gapScore - prevScore;
          const barHeight = Math.max(4, Math.round((round.gapScore / maxScore) * 48));
          const isLast = idx === rounds.length - 1;

          // Select semantic token per state. Light variants for normal, heavy for "isLast".
          let barToken = "var(--accent-primary)";
          if (round.gapScore === 0) barToken = "var(--status-success)";
          else if (delta < 0) barToken = "var(--status-success)";
          else if (delta > 0) barToken = "var(--status-error)";

          const barBg = withAlpha(barToken, isLast ? 80 : 50);
          const barBorder = isLast ? `1px solid ${withAlpha(barToken, 90)}` : "1px solid transparent";

          return (
            <div key={round.round} className="flex flex-col items-center gap-1.5 flex-1 min-w-0">
              {/* Trend arrow */}
              <div className="h-4 flex items-center">
                {idx === 0 || delta === 0 ? (
                  <Minus className="w-3 h-3" style={{ color: "var(--text-muted)" }} />
                ) : delta < 0 ? (
                  <TrendingDown className="w-3 h-3" style={{ color: "var(--status-success)" }} />
                ) : (
                  <TrendingUp className="w-3 h-3" style={{ color: "var(--status-error)" }} />
                )}
              </div>
              {/* Bar */}
              <div className="w-full flex flex-col items-center gap-1">
                <span
                  className="text-[10px] font-medium tabular-nums"
                  style={{ color: isLast ? "var(--text-secondary)" : "var(--text-muted)" }}
                >
                  {round.gapScore}
                </span>
                <div
                  className="w-full rounded-sm transition-all duration-300"
                  style={{
                    height: `${barHeight}px`,
                    background: barBg,
                    border: barBorder,
                  }}
                />
              </div>
              {/* Round label */}
              <span
                className="text-[10px]"
                style={{ color: isLast ? "var(--text-secondary)" : "var(--text-muted)" }}
              >
                R{round.round}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function GapBreakdown({ gaps }: { gaps: VerificationGap[] }) {
  const grouped = groupGapsBySeverity(gaps);

  return (
    <div>
      <div
        className="text-[11px] font-semibold uppercase tracking-wider mb-3"
        style={{ color: "var(--text-muted)" }}
      >
        Final Gaps ({gaps.length})
      </div>
      <div className="space-y-3">
        {SEVERITY_ORDER.filter((sev) => grouped[sev]?.length).map((severity) => {
          const severityGaps = grouped[severity]!;
          const config = SEVERITY_CONFIG[severity];
          return (
            <div key={severity}>
              <div className="flex items-center gap-2 mb-1.5">
                <span
                  className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                  style={{ background: config.color }}
                />
                <span
                  className="text-[11px] font-medium"
                  style={{ color: config.color }}
                >
                  {config.label} ({severityGaps.length})
                </span>
              </div>
              <div className="space-y-1.5 pl-3.5">
                {severityGaps.map((gap, idx) => (
                  <div
                    key={idx}
                    className="rounded-md px-2.5 py-2"
                    style={{ background: config.bg }}
                  >
                    <div className="text-[12px]" style={{ color: "var(--text-primary)" }}>
                      {gap.description}
                    </div>
                    {gap.whyItMatters && (
                      <div
                        className="text-[11px] mt-0.5"
                        style={{ color: "var(--text-secondary)" }}
                      >
                        {gap.whyItMatters}
                      </div>
                    )}
                    <div
                      className="text-[10px] mt-1 font-medium uppercase tracking-wide"
                      style={{ color: "var(--text-muted)" }}
                    >
                      {gap.category}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function RoundLineage({ roundDetails }: { roundDetails: VerificationRoundDetail[] }) {
  return (
    <div className="space-y-4">
      <div
        className="text-[11px] font-semibold uppercase tracking-wider"
        style={{ color: "var(--text-muted)" }}
      >
        Round Lineage
      </div>
      {roundDetails.map((round, index) => {
        const previous = index > 0 ? roundDetails[index - 1] : undefined;
        const currentKeys = new Set(round.gaps.map(gapKey));
        const resolved = previous
          ? previous.gaps.filter((gap) => !currentKeys.has(gapKey(gap)))
          : [];

        return (
          <div
            key={round.round}
            className="rounded-lg px-3 py-3 space-y-2"
            style={{
              background: "var(--overlay-faint)",
              border: "1px solid var(--overlay-faint)",
            }}
          >
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-[12px] font-medium" style={{ color: "var(--text-primary)" }}>
                Round {round.round}
              </span>
              <span className="text-[11px]" style={{ color: "var(--text-secondary)" }}>
                Score {round.gapScore}
              </span>
              <span className="text-[11px]" style={{ color: "var(--text-secondary)" }}>
                {round.gapCount} gap{round.gapCount === 1 ? "" : "s"}
              </span>
            </div>

            <div>
              <div className="text-[11px] font-medium mb-1" style={{ color: "var(--text-secondary)" }}>
                Remaining after round {round.round}
              </div>
              {round.gaps.length > 0 ? (
                <div className="space-y-1.5">
                  {round.gaps.map((gap, gapIndex) => (
                    <div
                      key={`${round.round}-${gapIndex}-${gapKey(gap)}`}
                      className="rounded-md px-2.5 py-2"
                      style={{ background: "var(--overlay-faint)" }}
                    >
                      <div className="text-[12px]" style={{ color: "var(--text-primary)" }}>
                        {gap.description}
                      </div>
                      <div className="text-[10px] mt-1 uppercase tracking-wide" style={{ color: "var(--text-muted)" }}>
                        {gap.severity} · {gap.category}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
                  No gaps remained after this round.
                </div>
              )}
            </div>

            {resolved.length > 0 && (
              <div>
                <div className="text-[11px] font-medium mb-1" style={{ color: "var(--status-success)" }}>
                  Addressed Since Round {previous?.round}
                </div>
                <div className="space-y-1.5">
                  {resolved.map((gap, gapIndex) => (
                    <div
                      key={`${round.round}-resolved-${gapIndex}-${gapKey(gap)}`}
                      className="rounded-md px-2.5 py-2"
                      style={{ background: "var(--status-success-muted)" }}
                    >
                      <div className="text-[12px]" style={{ color: "var(--text-primary)" }}>
                        {gap.description}
                      </div>
                      <div className="text-[10px] mt-1 uppercase tracking-wide" style={{ color: "var(--text-muted)" }}>
                        {gap.severity} · {gap.category}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ============================================================================
// Main component
// ============================================================================

export function VerificationHistory({
  rounds,
  roundDetails,
  currentGaps,
  gapScore,
  status,
  convergenceReason,
}: VerificationHistoryProps) {
  const isVerified = status === "verified";
  const hasGaps = currentGaps !== undefined && currentGaps.length > 0;
  const hasRoundDetails = roundDetails !== undefined && roundDetails.length > 0;
  const convergenceLabel = convergenceReason
    ? (CONVERGENCE_LABELS[convergenceReason] ?? convergenceReason)
    : undefined;

  return (
    <div className="py-2">
      {/* Status summary */}
      {status && status !== "reviewing" && (
        <div
          className={cn(
            "flex items-center gap-2.5 rounded-lg px-3 py-2.5 mb-4",
          )}
          style={{
            background: isVerified
              ? "var(--status-success-muted)"
              : "var(--status-error-muted)",
            border: isVerified
              ? "1px solid var(--status-success-border)"
              : "1px solid var(--status-error-border)",
          }}
        >
          {isVerified ? (
            <CheckCircle2 className="w-4 h-4 flex-shrink-0" style={{ color: "var(--status-success)" }} />
          ) : (
            <AlertTriangle className="w-4 h-4 flex-shrink-0" style={{ color: "var(--status-error)" }} />
          )}
          <div>
            <div
              className="text-[12px] font-medium"
              style={{ color: isVerified ? "var(--status-success)" : "var(--status-error)" }}
            >
              {isVerified ? "Plan verified" : "Gaps require attention"}
            </div>
            {convergenceLabel && (
              <div className="text-[11px] mt-0.5" style={{ color: "var(--text-secondary)" }}>
                {convergenceLabel}
              </div>
            )}
            {gapScore !== undefined && gapScore > 0 && (
              <div className="text-[11px] mt-0.5" style={{ color: "var(--text-secondary)" }}>
                Gap score: {gapScore}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Round timeline */}
      {rounds.length > 0 && <RoundTimeline rounds={rounds} />}

      {/* No rounds yet */}
      {rounds.length === 0 && !hasGaps && (
        <p
          className="text-[12px] py-4 text-center"
          style={{ color: "var(--text-muted)" }}
        >
          No verification rounds recorded.
        </p>
      )}

      {hasRoundDetails && <RoundLineage roundDetails={roundDetails!} />}

      {/* Final gaps */}
      {hasGaps && !hasRoundDetails && <GapBreakdown gaps={currentGaps!} />}
    </div>
  );
}
