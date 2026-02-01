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
  const shouldAutoCollapse = defaultCollapsed ?? proposalCount >= AUTO_COLLAPSE_THRESHOLD;
  const [internalIsOpen, setInternalIsOpen] = useState(!shouldAutoCollapse);

  const isOpen = isExpanded !== undefined ? isExpanded : internalIsOpen;
  const setIsOpen = onExpandedChange ?? setInternalIsOpen;

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
              "flex items-center gap-3 w-full text-left py-2.5 px-3",
              "rounded-lg transition-all duration-200",
              "focus:outline-none focus-visible:ring-1 focus-visible:ring-[#ff6b35]/50"
            )}
            style={{
              background: isOpen ? "rgba(255,255,255,0.02)" : "transparent",
            }}
            onMouseEnter={(e) => {
              if (!isOpen) {
                e.currentTarget.style.background = "rgba(255,255,255,0.02)";
              }
            }}
            onMouseLeave={(e) => {
              if (!isOpen) {
                e.currentTarget.style.background = "transparent";
              }
            }}
          >
            {/* Accent bar */}
            <div
              className="w-[3px] h-5 rounded-full transition-all duration-200"
              style={{
                background: isOpen
                  ? "linear-gradient(180deg, #ff6b35 0%, #ff8050 100%)"
                  : "rgba(255,255,255,0.15)",
                boxShadow: isOpen ? "0 0 8px rgba(255,107,53,0.3)" : "none",
              }}
            />

            {/* Tier info */}
            <div className="flex items-center gap-2 flex-1">
              <span
                className="text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: isOpen ? "#ff6b35" : "var(--text-muted)" }}
              >
                Tier {tier}
              </span>
              <span
                className="text-[11px]"
                style={{ color: "var(--text-muted)", opacity: 0.5 }}
              >
                ·
              </span>
              <span
                className="text-[12px] font-medium"
                style={{ color: "var(--text-secondary)" }}
              >
                {displayLabel}
              </span>
            </div>

            {/* Proposal count */}
            <span
              className="text-[11px] font-medium px-2 py-0.5 rounded-md"
              style={{
                background: "rgba(255,255,255,0.04)",
                border: "1px solid rgba(255,255,255,0.06)",
                color: "var(--text-muted)",
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
              style={{ color: "var(--text-muted)" }}
            />
          </button>
        </CollapsibleTrigger>

        {/* Content */}
        <CollapsibleContent>
          <div
            className="pl-5 pr-1 pb-3 pt-2 ml-[14px]"
            style={{
              borderLeft: "2px solid rgba(255,107,53,0.15)",
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
