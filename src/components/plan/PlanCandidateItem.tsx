/**
 * PlanCandidateItem - Individual plan row in quick switcher
 *
 * Extracted from PlanQuickSwitcherPalette to reduce component complexity.
 * Renders a single plan candidate with title, task stats, progress bar, and active indicator.
 */

import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import type { PlanCandidate } from "@/stores/planStore";

// ============================================================================
// Types
// ============================================================================

interface PlanCandidateItemProps {
  plan: PlanCandidate;
  isActive: boolean;
  isHighlighted: boolean;
  onMouseEnter: () => void;
  onClick: () => void;
  highlightedRef?: React.RefObject<HTMLButtonElement | null>;
}

// ============================================================================
// Utility Functions
// ============================================================================

function formatIncompleteSummary(incomplete: number, total: number): string {
  if (total <= 0) return "No tasks yet";
  if (incomplete <= 0) {
    return total === 1 ? "1 task complete" : `${total} tasks complete`;
  }
  return `${incomplete} of ${total} incomplete`;
}

function getCompletionPercent(incomplete: number, total: number): number {
  if (total <= 0) return 0;
  const completed = Math.max(0, total - Math.max(0, incomplete));
  return Math.round((completed / total) * 100);
}

// ============================================================================
// Component
// ============================================================================

export function PlanCandidateItem({
  plan,
  isActive,
  isHighlighted,
  onMouseEnter,
  onClick,
  highlightedRef,
}: PlanCandidateItemProps) {
  const completionPercent = getCompletionPercent(
    plan.taskStats.incomplete,
    plan.taskStats.total
  );
  const showProgressBar =
    plan.taskStats.total > 0 && plan.taskStats.incomplete > 0;

  return (
    <button
      ref={isHighlighted ? highlightedRef : null}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      className={cn(
        "w-full text-left px-3 py-2 rounded-lg flex items-center justify-between",
        "transition-all duration-150 origin-center",
        "hover:scale-[1.01]",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        isHighlighted && "bg-accent",
        isActive && "bg-accent/50"
      )}
      style={{
        background: isHighlighted
          ? "hsla(14 100% 60% / 0.16)"
          : isActive
            ? "hsla(14 100% 60% / 0.1)"
            : "transparent",
        border: isHighlighted
          ? "1px solid hsla(14 100% 60% / 0.35)"
          : "1px solid transparent",
      }}
    >
      <div className="flex-1 min-w-0">
        <div
          className="text-[13px] font-medium leading-tight"
          style={{ color: isHighlighted ? "hsl(14 100% 66%)" : "hsl(220 10% 90%)" }}
        >
          {plan.title || "Untitled Plan"}
        </div>
        <div className="text-xs leading-tight mt-0.5" style={{ color: "hsl(220 10% 62%)" }}>
          {formatIncompleteSummary(plan.taskStats.incomplete, plan.taskStats.total)}
          {plan.taskStats.activeNow > 0 && " • Active work"}
        </div>
      </div>

      <div className="flex items-center gap-2 ml-3 shrink-0">
        {showProgressBar && (
          <div className="flex items-center gap-1.5" aria-hidden="true">
            <div
              className="w-14 h-1 rounded-full overflow-hidden"
              style={{ backgroundColor: "hsla(220 10% 100% / 0.1)" }}
            >
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${completionPercent}%`,
                  backgroundColor: "hsla(14 100% 60% / 0.7)",
                }}
              />
            </div>
            <span
              className="text-[10px] tabular-nums"
              style={{ color: "hsl(220 10% 48%)" }}
            >
              {completionPercent}%
            </span>
          </div>
        )}
        {isActive && <Check className="h-4 w-4" style={{ color: "hsl(14 100% 62%)" }} />}
      </div>
    </button>
  );
}
