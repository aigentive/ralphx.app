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
  const loadCandidates = usePlanStore((state) => state.loadCandidates);

  // Find active plan details
  const activePlan = React.useMemo(
    () => planCandidates.find((p) => p.sessionId === activePlanId),
    [planCandidates, activePlanId]
  );

  // Ensure inline selector can resolve active plan title/stats without requiring
  // the command palette to be opened first.
  React.useEffect(() => {
    if (!projectId) return;
    if (!activePlanId) return;
    if (activePlan) return;
    void loadCandidates(projectId);
  }, [projectId, activePlanId, activePlan, loadCandidates]);

  const handleOpenPalette = React.useCallback(() => {
    onOpenPalette(source);
  }, [onOpenPalette, source]);

  const hasActivePlan = Boolean(activePlanId);

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={handleOpenPalette}
      className={cn(
        "gap-1.5",
        !hasActivePlan && "text-muted-foreground"
      )}
      data-testid="plan-selector-inline-trigger"
    >
      {hasActivePlan ? (
        <>
          <FileText className="h-4 w-4" />
          {!compact && (
            <>
              <span className="truncate max-w-[200px]">
                {activePlan?.title || "Untitled Plan"}
              </span>
              {activePlan?.taskStats && (
                <Badge variant="secondary" className="ml-1">
                  {activePlan.taskStats.incomplete}/{activePlan.taskStats.total}
                </Badge>
              )}
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
