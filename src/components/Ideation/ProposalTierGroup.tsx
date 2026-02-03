/* eslint-disable react-refresh/only-export-components */
/**
 * ProposalTierGroup - macOS Tahoe styled collapsible tier section
 *
 * Design: Clean collapsible with warm orange accent bar,
 * refined typography, and smooth animations.
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
  tier: number;
  label?: string;
  proposalCount: number;
  defaultCollapsed?: boolean;
  isExpanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
  children: React.ReactNode;
  className?: string;
}

// ============================================================================
// Helpers
// ============================================================================

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
  const shouldCollapse = defaultCollapsed ?? false;
  const [internalIsOpen, setInternalIsOpen] = useState(!shouldCollapse);

  const isOpen = isExpanded !== undefined ? isExpanded : internalIsOpen;
  const setIsOpen = onExpandedChange ?? setInternalIsOpen;

  useEffect(() => {
    if (isExpanded === undefined) {
      setInternalIsOpen(!shouldCollapse);
    }
  }, [shouldCollapse, isExpanded]);

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
              "flex items-center gap-3 w-full text-left py-2.5 px-3",
              "rounded-lg transition-all duration-200",
              "focus:outline-none focus-visible:ring-1 focus-visible:ring-[hsl(14_100%_60%)]/50"
            )}
            style={{
              background: "hsla(220 10% 100% / 0.02)",
            }}
          >
            {/* Accent bar - flat, no glow */}
            <div
              className="w-[3px] h-5 rounded-full transition-all duration-200"
              style={{
                background: isOpen
                  ? "hsl(14 100% 60%)"
                  : "hsla(220 10% 100% / 0.15)",
              }}
            />

            {/* Tier info */}
            <div className="flex items-center gap-2 flex-1">
              <span
                className="text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: isOpen ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)" }}
              >
                Tier {tier}
              </span>
              <span
                className="text-[11px]"
                style={{ color: "hsl(220 10% 50%)", opacity: 0.5 }}
              >
                ·
              </span>
              <span
                className="text-[12px] font-medium"
                style={{ color: "hsl(220 10% 70%)" }}
              >
                {displayLabel}
              </span>
            </div>

            {/* Proposal count */}
            <span
              className="text-[11px] font-medium px-2 py-0.5 rounded-md"
              style={{
                background: "hsla(220 10% 100% / 0.04)",
                border: "1px solid hsla(220 10% 100% / 0.06)",
                color: "hsl(220 10% 55%)",
              }}
            >
              {proposalCount}
            </span>

            {/* Chevron */}
            <ChevronDown
              className={cn(
                "w-4 h-4 transition-transform duration-200",
                !isOpen && "-rotate-90"
              )}
              style={{ color: "hsl(220 10% 50%)" }}
            />
          </button>
        </CollapsibleTrigger>

        {/* Content */}
        <CollapsibleContent>
          <div
            className="pl-5 pr-1 pb-3 pt-2 ml-[14px]"
            style={{
              borderLeft: "2px solid hsla(14 100% 60% / 0.15)",
            }}
          >
            {children}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
});

export default ProposalTierGroup;
