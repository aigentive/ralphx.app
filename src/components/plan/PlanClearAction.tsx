/**
 * PlanClearAction - Reusable "Clear active plan" button for quick switcher
 *
 * Features:
 * - Glass morphism styling with accent colors
 * - Hover/highlight states
 * - Auto-scaling on hover
 */

import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

interface PlanClearActionProps {
  isHighlighted: boolean;
  onMouseEnter: () => void;
  onClick: () => void;
  highlightedRef?: React.RefObject<HTMLButtonElement> | undefined;
}

// ============================================================================
// Component
// ============================================================================

export function PlanClearAction({
  isHighlighted,
  onMouseEnter,
  onClick,
  highlightedRef,
}: PlanClearActionProps) {
  return (
    <button
      ref={highlightedRef}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      className={cn(
        "w-full text-left px-3 py-2 rounded-lg flex items-center justify-between",
        "transition-all duration-150 origin-center",
        "hover:scale-[1.01]",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        isHighlighted && "bg-accent"
      )}
      style={{
        background:
          isHighlighted
            ? "hsla(14 100% 60% / 0.16)"
            : "transparent",
        border:
          isHighlighted
            ? "1px solid hsla(14 100% 60% / 0.35)"
            : "1px solid transparent",
      }}
      data-testid="plan-quick-switcher-clear"
    >
      <div className="flex-1 min-w-0">
        <div
          className="text-[13px] font-medium leading-tight"
          style={{ color: isHighlighted ? "hsl(14 100% 66%)" : "hsl(220 10% 90%)" }}
        >
          Clear active plan
        </div>
        <div className="text-xs leading-tight mt-0.5" style={{ color: "hsl(220 10% 62%)" }}>
          Return to no active plan state
        </div>
      </div>
    </button>
  );
}
