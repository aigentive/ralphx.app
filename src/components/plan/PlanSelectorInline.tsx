/**
 * PlanSelectorInline Component
 *
 * Shared inline plan selector trigger used in Graph and Kanban toolbars.
 * Shows current plan title + task count badge and opens the global palette.
 */

import * as React from "react";
import { FileText, AlertCircle, ChevronDown } from "lucide-react";
import { usePlanStore } from "@/stores/planStore";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { SelectionSource } from "@/api/plan";

// ============================================================================
// Props Interface
// ============================================================================

interface PlanSelectorInlineProps {
  projectId: string;
  /** Icon-only mode for tight layouts */
  compact?: boolean;
  /** Selection source attributed when this launcher opens the palette */
  source: SelectionSource;
  /** Opens the global plan palette */
  onOpenPalette: (source: SelectionSource) => void;
}

// ============================================================================
// Component
// ============================================================================

export function PlanSelectorInline({
  projectId,
  compact = false,
  source,
  onOpenPalette,
}: PlanSelectorInlineProps) {
  // Store state
  const activePlanId = usePlanStore(
    (state) => state.activePlanByProject[projectId] ?? null
  );
  const planCandidates = usePlanStore((state) => state.planCandidates);

  // Find active plan details
  const activePlan = React.useMemo(
    () => planCandidates.find((p) => p.sessionId === activePlanId),
    [planCandidates, activePlanId]
  );

  const handleOpenPalette = React.useCallback(() => {
    onOpenPalette(source);
  }, [onOpenPalette, source]);

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={handleOpenPalette}
      className={cn(
        "gap-1.5",
        !activePlanId && "text-muted-foreground"
      )}
      data-testid="plan-selector-inline-trigger"
    >
      {activePlan ? (
        <>
          <FileText className="h-4 w-4" />
          {!compact && (
            <>
              <span className="truncate max-w-[200px]">
                {activePlan.title || "Untitled Plan"}
              </span>
              <Badge variant="secondary" className="ml-1">
                {activePlan.taskStats.incomplete}/{activePlan.taskStats.total}
              </Badge>
            </>
          )}
          {compact && (
            <ChevronDown className="h-4 w-4 opacity-50" />
          )}
        </>
      ) : (
        <>
          <AlertCircle className="h-4 w-4" />
          {!compact && <span>Select plan</span>}
          <ChevronDown className="h-4 w-4 opacity-50" />
        </>
      )}
    </Button>
  );
}
