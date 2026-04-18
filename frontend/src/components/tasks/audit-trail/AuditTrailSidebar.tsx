/**
 * AuditTrailSidebar - Vertical phase timeline navigation for the audit trail dialog.
 * 320px wide, scrollable. Summary stats + clickable phase list with dot+line design.
 */

import type { ReactNode } from "react";
import type { AuditPhase } from "@/hooks/useAuditTrail";

export interface AuditTrailSidebarProps {
  phases: AuditPhase[];
  selectedPhaseId: string | null;
  onPhaseSelect: (phaseId: string | null) => void;
  totalEvents: number;
  dateRange: string;
  isLoading: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const PHASE_TYPE_COLORS: Record<
  AuditPhase["type"],
  { color: string; bgColor: string }
> = {
  execution: { color: "#ff6b35", bgColor: "rgba(255, 107, 53, 0.15)" },
  review: { color: "#0a84ff", bgColor: "rgba(10, 132, 255, 0.15)" },
  merge: { color: "#34c759", bgColor: "rgba(52, 199, 89, 0.15)" },
  idle: { color: "#8e8e93", bgColor: "rgba(142, 142, 147, 0.15)" },
  qa: { color: "#ff9f0a", bgColor: "rgba(255, 159, 10, 0.15)" },
};

const REVIEW_OUTCOME_ICONS: Record<string, string> = {
  approved: "✅",
  changes_requested: "↩️",
  rejected: "❌",
};

// ============================================================================
// Helpers
// ============================================================================

function formatDuration(startMs: number, endMs: number | null): string {
  const end = endMs ?? Date.now();
  const diff = end - startMs;
  if (diff < 60000) return `${Math.round(diff / 1000)}s`;
  if (diff < 3600000) return `${Math.round(diff / 60000)}m`;
  return `${Math.round(diff / 3600000)}h`;
}

// ============================================================================
// Sub-components
// ============================================================================

function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h4
      className="text-[11px] font-semibold uppercase tracking-wider mb-2"
      style={{ color: "var(--text-muted)" }}
    >
      {children}
    </h4>
  );
}

interface PhaseItemProps {
  phase: AuditPhase;
  isSelected: boolean;
  isLast: boolean;
  onToggle: () => void;
}

function PhaseItem({ phase, isSelected, isLast, onToggle }: PhaseItemProps) {
  const config = PHASE_TYPE_COLORS[phase.type];

  return (
    <div
      data-testid={`phase-item-${phase.id}`}
      className="relative pl-6"
      style={{ paddingBottom: isLast ? 0 : "12px" }}
    >
      {/* Vertical connector line */}
      {!isLast && (
        <div
          data-testid={`phase-connector-${phase.id}`}
          className="absolute w-0.5"
          style={{
            left: "5px",
            top: isSelected ? "20px" : "12px",
            bottom: 0,
            backgroundColor: "var(--border-subtle, rgba(255,255,255,0.08))",
          }}
        />
      )}

      {/* Status dot */}
      <div
        className="absolute rounded-full transition-all duration-200"
        style={{
          left: isSelected ? "-2px" : 0,
          top: isSelected ? "2px" : "4px",
          width: isSelected ? "16px" : "12px",
          height: isSelected ? "16px" : "12px",
          backgroundColor: config.color,
          border: isSelected ? "none" : "2px solid var(--bg-elevated, #1a1a1a)",
          boxShadow: isSelected ? `0 0 0 4px ${config.color}33` : undefined,
        }}
      />

      {/* Clickable content area */}
      <button
        data-testid={`phase-button-${phase.id}`}
        type="button"
        onClick={onToggle}
        aria-pressed={isSelected}
        className="w-full text-left rounded px-2 py-1.5 transition-all duration-200"
        style={{
          backgroundColor: isSelected ? config.bgColor : "transparent",
          boxShadow: isSelected
            ? `0 0 0 2px ${config.color}50, 0 2px 8px ${config.color}30`
            : undefined,
        }}
      >
        <div className="flex items-center justify-between gap-2">
          <span
            className="text-[12px] font-medium truncate"
            style={{ color: isSelected ? config.color : "var(--text-primary)" }}
          >
            {phase.label}
          </span>
          <span
            data-testid={`phase-duration-${phase.id}`}
            className="text-[11px] shrink-0 tabular-nums"
            style={{ color: "var(--text-muted)" }}
          >
            {formatDuration(phase.startTime, phase.endTime)}
          </span>
        </div>

        <div className="text-[11px] mt-0.5" style={{ color: "var(--text-secondary)" }}>
          {phase.reviewOutcome && (
            <>
              {REVIEW_OUTCOME_ICONS[phase.reviewOutcome] ?? ""}{" "}
              {phase.reviewOutcome.replace(/_/g, " ")}{" · "}
            </>
          )}
          {phase.entryCount} {phase.entryCount === 1 ? "event" : "events"}
        </div>
      </button>
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div data-testid="sidebar-loading" className="p-4 space-y-4 animate-pulse">
      <div>
        <div
          className="h-3 w-16 rounded mb-2"
          style={{ backgroundColor: "var(--overlay-weak)" }}
        />
        <div
          className="h-10 rounded"
          style={{ backgroundColor: "var(--overlay-faint)" }}
        />
      </div>
      <div>
        <div
          className="h-3 w-16 rounded mb-3"
          style={{ backgroundColor: "var(--overlay-weak)" }}
        />
        {[1, 2, 3].map((i) => (
          <div key={i} className="relative pl-6 pb-3">
            <div
              className="absolute w-0.5"
              style={{
                left: "5px",
                top: "12px",
                bottom: 0,
                backgroundColor: "var(--overlay-weak)",
              }}
            />
            <div
              className="absolute w-3 h-3 rounded-full"
              style={{ left: 0, top: "4px", backgroundColor: "var(--overlay-moderate)" }}
            />
            <div
              className="h-8 rounded"
              style={{ backgroundColor: "var(--overlay-faint)" }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function AuditTrailSidebar({
  phases,
  selectedPhaseId,
  onPhaseSelect,
  totalEvents,
  dateRange,
  isLoading,
}: AuditTrailSidebarProps) {
  if (isLoading) {
    return <LoadingSkeleton />;
  }

  return (
    <div
      data-testid="audit-trail-sidebar"
      className="w-[320px] shrink-0 flex flex-col overflow-y-auto"
      style={{ borderRight: "0.5px solid var(--overlay-weak)" }}
    >
      <div className="p-4 space-y-5">
        {/* Summary */}
        <div>
          <SectionTitle>Summary</SectionTitle>
          <div
            data-testid="sidebar-summary"
            className="flex items-center gap-3 py-2.5 px-3 rounded"
            style={{
              backgroundColor: "rgba(0,0,0,0.15)",
              border: "1px solid rgba(255,255,255,0.05)",
            }}
          >
            <div className="text-[12px]">
              <span
                data-testid="total-events"
                className="font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                {totalEvents}
              </span>
              <span className="ml-1" style={{ color: "var(--text-muted)" }}>
                events
              </span>
            </div>
            {dateRange && (
              <div
                data-testid="date-range"
                className="text-[11px] ml-auto"
                style={{ color: "var(--text-muted)" }}
              >
                {dateRange}
              </div>
            )}
          </div>
        </div>

        {/* Phase timeline */}
        {phases.length > 0 && (
          <div>
            <SectionTitle>Timeline</SectionTitle>
            <div>
              {phases.map((phase, index) => (
                <PhaseItem
                  key={phase.id}
                  phase={phase}
                  isSelected={selectedPhaseId === phase.id}
                  isLast={index === phases.length - 1}
                  onToggle={() =>
                    onPhaseSelect(selectedPhaseId === phase.id ? null : phase.id)
                  }
                />
              ))}
            </div>
          </div>
        )}

        {/* View All */}
        {phases.length > 0 && (
          <button
            data-testid="view-all-button"
            type="button"
            onClick={() => onPhaseSelect(null)}
            className="w-full py-2 rounded text-[12px] font-medium transition-colors duration-200"
            style={{
              backgroundColor:
                selectedPhaseId === null
                  ? "var(--accent-muted)"
                  : "var(--overlay-faint)",
              color:
                selectedPhaseId === null
                  ? "var(--accent-primary)"
                  : "var(--text-secondary)",
              border: "1px solid var(--overlay-weak)",
            }}
          >
            View All Events
          </button>
        )}
      </div>
    </div>
  );
}
