/**
 * ProposalTierGroup - Collapsible tier section for grouping proposals by dependency level
 *
 * Features:
 * - Collapsible section with expand/collapse toggle
 * - Tier labels: Foundation (0), Core (1), Integration (2+)
 * - Auto-collapse when proposalCount >= 5
 * - Warm accent border-left styling (#ff6b35)
 */

import React, { useState, useEffect } from "react";
import { ChevronDown } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

export interface ProposalTierGroupProps {
  /** Tier level (0, 1, 2, etc.) */
  tier: number;
  /** Optional custom label, otherwise computed from tier */
  label?: string;
  /** Number of proposals in this tier (for auto-collapse logic) */
  proposalCount: number;
  /** Whether to start collapsed - defaults based on proposalCount >= 5 */
  defaultCollapsed?: boolean;
  /** Controlled expanded state - if provided, component is controlled */
  isExpanded?: boolean;
  /** Callback when expanded state changes */
  onExpandedChange?: (expanded: boolean) => void;
  /** Children to render inside the collapsible content */
  children: React.ReactNode;
  /** Additional className for the root element */
  className?: string;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Get tier label based on tier level
 * - Tier 0: Foundation (no dependencies)
 * - Tier 1: Core (depends on foundation)
 * - Tier 2+: Integration (depends on multiple tiers)
 */
export function getTierLabel(tier: number): string {
  switch (tier) {
    case 0:
      return "Foundation";
    case 1:
      return "Core";
    default:
      return "Integration";
  }
}

/**
 * Auto-collapse threshold - tiers with 5+ proposals auto-collapse
 */
const AUTO_COLLAPSE_THRESHOLD = 5;

// ============================================================================
// Component
// ============================================================================

export const ProposalTierGroup = React.memo(function ProposalTierGroup({
  tier,
  label,
  proposalCount,
  defaultCollapsed,
  isExpanded,
  onExpandedChange,
  children,
  className,
}: ProposalTierGroupProps) {
  // Compute whether to auto-collapse based on proposalCount
  const shouldAutoCollapse = defaultCollapsed ?? proposalCount >= AUTO_COLLAPSE_THRESHOLD;

  // Internal state for uncontrolled mode (default: NOT collapsed, unless auto-collapse kicks in)
  const [internalIsOpen, setInternalIsOpen] = useState(!shouldAutoCollapse);

  // Use controlled state if isExpanded prop is provided, otherwise use internal state
  const isOpen = isExpanded !== undefined ? isExpanded : internalIsOpen;
  const setIsOpen = onExpandedChange ?? setInternalIsOpen;

  // Update internal state if defaultCollapsed or proposalCount changes
  useEffect(() => {
    if (isExpanded === undefined) {
      setInternalIsOpen(!shouldAutoCollapse);
    }
  }, [shouldAutoCollapse, isExpanded]);

  const displayLabel = label ?? getTierLabel(tier);

  return (
    <div
      data-testid={`proposal-tier-group-${tier}`}
      className={cn("relative", className)}
    >
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        {/* Tier Header */}
        <CollapsibleTrigger asChild>
          <button
            className={cn(
              "flex items-center gap-2 w-full text-left py-2 px-3",
              "rounded-md transition-all duration-200",
              "hover:bg-white/[0.02]",
              "focus:outline-none focus-visible:ring-1 focus-visible:ring-[#ff6b35]/50"
            )}
          >
            {/* Accent bar */}
            <div
              className={cn(
                "w-0.5 h-4 rounded-full transition-colors",
                isOpen ? "bg-[#ff6b35]" : "bg-white/20"
              )}
            />

            {/* Tier info */}
            <span className="text-xs font-medium text-[var(--text-primary)] uppercase tracking-wider">
              Tier {tier}
            </span>
            <span className="text-xs text-[var(--text-muted)]">·</span>
            <span className="text-xs text-[var(--text-secondary)]">
              {displayLabel}
            </span>

            {/* Proposal count - always show */}
            <span className="text-xs text-[var(--text-muted)] px-1.5 py-0.5 rounded bg-white/[0.04] border border-white/[0.06]">
              {proposalCount} {proposalCount === 1 ? "proposal" : "proposals"}
            </span>

            {/* Chevron indicator */}
            <ChevronDown
              className={cn(
                "w-3.5 h-3.5 ml-auto text-[var(--text-muted)] transition-transform duration-200",
                !isOpen && "-rotate-90"
              )}
            />
          </button>
        </CollapsibleTrigger>

        {/* Content */}
        <CollapsibleContent>
          <div
            className={cn(
              "pl-4 pr-1 pb-2 pt-1",
              // Left border accent line connecting to header
              "border-l-2 border-[#ff6b35]/20 ml-[11px]"
            )}
          >
            {children}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
});

export default ProposalTierGroup;
