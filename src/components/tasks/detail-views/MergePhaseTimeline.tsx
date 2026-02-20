/**
 * MergePhaseTimeline - Shows high-level merge progress through phases
 *
 * Renders a linear timeline of merge phases with status indicators:
 * - started → spinner (in progress)
 * - passed → green check (success)
 * - failed → red X (error)
 * - pending → gray circle (not started)
 *
 * Supports dynamic phase lists derived from project analysis.
 * Falls back to a hardcoded default if no dynamic list is provided.
 */

import {
  Loader2,
  CheckCircle2,
  XCircle,
  SkipForward,
} from "lucide-react";
import { SectionTitle, DetailCard } from "./shared";
import type { MergeProgressEvent, MergePhaseInfo } from "@/types/events";

/** Default phase config — used as fallback when no dynamic phase list is received */
const DEFAULT_PHASE_CONFIG: MergePhaseInfo[] = [
  { id: "worktree_setup", label: "Worktree Setup" },
  { id: "programmatic_merge", label: "Merge" },
  { id: "npm_run_typecheck", label: "Type Check" },
  { id: "npm_run_lint", label: "Lint" },
  { id: "cargo_clippy", label: "Clippy" },
  { id: "cargo_test", label: "Test" },
  { id: "finalize", label: "Finalize" },
];

function PhaseIcon({ status }: { status: "started" | "passed" | "failed" | "skipped" | "pending" }) {
  if (status === "started") {
    return (
      <div className="relative">
        <Loader2 className="w-4 h-4 animate-spin" style={{ color: "#0a84ff" }} />
      </div>
    );
  }
  if (status === "passed") {
    return <CheckCircle2 className="w-4 h-4" style={{ color: "#34c759" }} />;
  }
  if (status === "failed") {
    return <XCircle className="w-4 h-4" style={{ color: "#ff453a" }} />;
  }
  if (status === "skipped") {
    return <SkipForward className="w-4 h-4" style={{ color: "rgba(255,255,255,0.3)" }} />;
  }
  return (
    <div
      className="w-4 h-4 rounded-full border-2"
      style={{ borderColor: "rgba(255,255,255,0.15)" }}
    />
  );
}

function phaseTextColor(status: "started" | "passed" | "failed" | "skipped" | "pending"): string {
  switch (status) {
    case "started":
      return "#0a84ff";
    case "passed":
      return "rgba(255, 255, 255, 0.6)";
    case "failed":
      return "#ff453a";
    case "skipped":
      return "rgba(255, 255, 255, 0.3)";
    default:
      return "rgba(255, 255, 255, 0.25)";
  }
}

interface MergePhaseTimelineProps {
  phases: MergeProgressEvent[];
  /** Dynamic phase list from project analysis. Falls back to DEFAULT_PHASE_CONFIG if null. */
  phaseList?: MergePhaseInfo[] | null;
}

export function MergePhaseTimeline({ phases, phaseList }: MergePhaseTimelineProps) {
  if (phases.length === 0) return null;

  const phaseConfig = phaseList ?? DEFAULT_PHASE_CONFIG;

  // Build a lookup of received phase events
  const phaseMap = new Map(phases.map((p) => [p.phase, p]));

  // Determine which phases to display: only phases we've received events for,
  // plus phases from config up to and including the last received one
  const receivedPhases = new Set(phases.map((p) => p.phase));
  let lastReceivedIdx = -1;
  for (let i = phaseConfig.length - 1; i >= 0; i--) {
    const cfg = phaseConfig[i];
    if (cfg && receivedPhases.has(cfg.id)) {
      lastReceivedIdx = i;
      break;
    }
  }

  // Show all phases up to the last received one, plus any beyond if received
  const visiblePhases = phaseConfig.filter((cfg, i) => {
    return i <= lastReceivedIdx || receivedPhases.has(cfg.id);
  });

  return (
    <section data-testid="merge-phase-timeline">
      <SectionTitle>
        Merge Progress
        <span className="ml-2 text-[10px] font-normal text-white/30">(live)</span>
      </SectionTitle>
      <DetailCard>
        <div className="space-y-0.5">
          {visiblePhases.map((config, index) => {
            const event = phaseMap.get(config.id);
            const status = event?.status ?? "pending";

            return (
              <div
                key={config.id}
                className="flex items-center gap-2.5 py-1.5"
                style={{
                  borderTop:
                    index > 0
                      ? "1px solid rgba(255, 255, 255, 0.05)"
                      : "none",
                }}
              >
                <PhaseIcon status={status} />
                <span
                  className="text-[13px] font-medium flex-1"
                  style={{ color: phaseTextColor(status) }}
                >
                  {config.label}
                </span>
                {event?.message && status === "started" && (
                  <span className="text-[11px] text-white/40 truncate max-w-[200px]">
                    {event.message}
                  </span>
                )}
                {status === "failed" && event?.message && (
                  <span className="text-[11px] truncate max-w-[200px]" style={{ color: "#ff6961" }}>
                    {event.message}
                  </span>
                )}
                {status === "skipped" && (
                  <span className="text-[11px] text-white/25 truncate max-w-[200px]">
                    skipped
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </DetailCard>
    </section>
  );
}
