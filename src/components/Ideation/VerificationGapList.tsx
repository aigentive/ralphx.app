/**
 * VerificationGapList — grouped gap display with severity counts and round trend.
 *
 * Design: macOS Tahoe style, warm orange accent (#ff6b35), SF Pro, no purple/blue.
 */

import { TrendingDown, TrendingUp, Minus } from "lucide-react";
import type { VerificationGap, VerificationRound } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

export interface VerificationGapListProps {
  /** Gaps from the latest/current round */
  gaps: VerificationGap[];
  /** All rounds — used for gap score trend visualization */
  rounds?: VerificationRound[];
  /** Gap score for the latest round (critical*10 + high*3 + medium*1) */
  gapScore?: number;
}

// ============================================================================
// Config
// ============================================================================

const SEVERITY_CONFIG = {
  critical: {
    label: "Critical",
    color: "hsl(0 70% 65%)",
    bg: "hsla(0 70% 50% / 0.08)",
    border: "hsla(0 70% 50% / 0.2)",
    dotColor: "hsl(0 70% 55%)",
    order: 0,
  },
  high: {
    label: "High",
    color: "hsl(14 100% 65%)",
    bg: "hsla(14 100% 60% / 0.08)",
    border: "hsla(14 100% 60% / 0.2)",
    dotColor: "hsl(14 100% 60%)",
    order: 1,
  },
  medium: {
    label: "Medium",
    color: "hsl(45 93% 60%)",
    bg: "hsla(45 93% 50% / 0.08)",
    border: "hsla(45 93% 50% / 0.2)",
    dotColor: "hsl(45 93% 55%)",
    order: 2,
  },
  low: {
    label: "Low",
    color: "hsl(220 10% 60%)",
    bg: "hsla(220 10% 100% / 0.04)",
    border: "hsla(220 10% 100% / 0.1)",
    dotColor: "hsl(220 10% 55%)",
    order: 3,
  },
} as const;

type Severity = keyof typeof SEVERITY_CONFIG;

// ============================================================================
// Helpers
// ============================================================================

function groupBySeverity(gaps: VerificationGap[]) {
  const groups: Partial<Record<Severity, VerificationGap[]>> = {};
  for (const gap of gaps) {
    const sev = gap.severity as Severity;
    if (!groups[sev]) groups[sev] = [];
    groups[sev]!.push(gap);
  }
  // Sort by severity order
  return (Object.keys(groups) as Severity[]).sort(
    (a, b) => SEVERITY_CONFIG[a].order - SEVERITY_CONFIG[b].order
  ).map((sev) => ({ severity: sev, gaps: groups[sev]! }));
}

// ============================================================================
// Sub-components
// ============================================================================

function GapScoreTrend({ rounds }: { rounds: VerificationRound[] }) {
  if (rounds.length < 2) return null;

  const last = rounds[rounds.length - 1]!;
  const prev = rounds[rounds.length - 2]!;
  const delta = last.gapScore - prev.gapScore;
  const improving = delta < 0;
  const stable = delta === 0;

  return (
    <div className="flex items-center gap-3 mb-3">
      <div
        className="flex items-center gap-1.5 text-[11px]"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        <span>Gap score:</span>
        <span
          className="font-semibold"
          style={{ color: "hsl(220 10% 85%)" }}
        >
          {last.gapScore}
        </span>
      </div>

      <div
        className="flex items-center gap-1 text-[11px]"
        style={{
          color: stable
            ? "hsl(220 10% 55%)"
            : improving
              ? "hsl(145 70% 50%)"
              : "hsl(0 70% 65%)",
        }}
      >
        {stable ? (
          <Minus className="w-3 h-3" />
        ) : improving ? (
          <TrendingDown className="w-3 h-3" />
        ) : (
          <TrendingUp className="w-3 h-3" />
        )}
        <span>
          {stable
            ? "No change"
            : improving
              ? `−${Math.abs(delta)} from round ${prev.round}`
              : `+${delta} from round ${prev.round}`}
        </span>
      </div>

      {/* Mini round history */}
      <div className="flex items-center gap-1 ml-auto">
        {rounds.slice(-5).map((r) => {
          const isLast = r.round === last.round;
          return (
            <div
              key={r.round}
              className="w-1.5 h-4 rounded-sm flex-shrink-0"
              title={`Round ${r.round}: score ${r.gapScore}`}
              style={{
                background: isLast
                  ? "hsl(14 100% 60%)"
                  : "hsla(220 10% 100% / 0.1)",
                opacity: isLast ? 1 : 0.6,
              }}
            />
          );
        })}
      </div>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function VerificationGapList({
  gaps,
  rounds,
  gapScore,
}: VerificationGapListProps) {
  if (gaps.length === 0) {
    return (
      <div
        className="flex items-center justify-center py-4 text-[12px]"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        No gaps found
      </div>
    );
  }

  const grouped = groupBySeverity(gaps);

  return (
    <div className="space-y-3">
      {/* Score and trend row */}
      {rounds && rounds.length > 0 ? (
        <GapScoreTrend rounds={rounds} />
      ) : gapScore !== undefined ? (
        <div
          className="flex items-center gap-1.5 text-[11px] mb-3"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          <span>Gap score:</span>
          <span className="font-semibold" style={{ color: "hsl(220 10% 85%)" }}>
            {gapScore}
          </span>
        </div>
      ) : null}

      {/* Severity summary row */}
      <div className="flex items-center gap-2 flex-wrap">
        {grouped.map(({ severity, gaps: sGaps }) => {
          const cfg = SEVERITY_CONFIG[severity];
          return (
            <span
              key={severity}
              className="inline-flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-md"
              style={{
                background: cfg.bg,
                border: `1px solid ${cfg.border}`,
                color: cfg.color,
              }}
            >
              <span
                className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                style={{ background: cfg.dotColor }}
              />
              {sGaps.length} {cfg.label}
            </span>
          );
        })}
      </div>

      {/* Gap items grouped by severity */}
      <div className="space-y-2">
        {grouped.map(({ severity, gaps: sGaps }) => {
          const cfg = SEVERITY_CONFIG[severity];
          return (
            <div key={severity}>
              <div
                className="text-[10px] font-semibold uppercase tracking-wider mb-1.5"
                style={{ color: cfg.color, opacity: 0.7 }}
              >
                {cfg.label}
              </div>
              <div className="space-y-1.5">
                {sGaps.map((gap, idx) => (
                  <div
                    key={idx}
                    className="rounded-md px-2.5 py-2"
                    style={{
                      background: cfg.bg,
                      border: `1px solid ${cfg.border}`,
                    }}
                  >
                    <div
                      className="text-[12px] leading-snug"
                      style={{ color: "hsl(220 10% 85%)" }}
                    >
                      {gap.description}
                    </div>
                    {gap.whyItMatters && (
                      <div
                        className="text-[11px] mt-1 leading-snug"
                        style={{ color: "hsl(220 10% 55%)" }}
                      >
                        {gap.whyItMatters}
                      </div>
                    )}
                    <div
                      className="text-[10px] mt-1 opacity-60"
                      style={{ color: cfg.color }}
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
