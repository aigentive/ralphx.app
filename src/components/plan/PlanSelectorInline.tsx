/**
 * PlanSelectorInline Component
 *
 * Shared inline plan selector using Radix UI Popover.
 * Shows current plan title + task count badge, opens searchable list of accepted plans,
 * supports keyboard navigation and single-select semantics.
 * Used in both Graph and Kanban toolbars.
 */

import * as React from "react";
import { FileText, AlertCircle, ChevronDown, Loader2, RefreshCw } from "lucide-react";
import { usePlanStore, type PlanCandidate } from "@/stores/planStore";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import type { SelectionSource } from "@/api/plan";

// ============================================================================
// Props Interface
// ============================================================================

interface PlanSelectorInlineProps {
  projectId: string;
  /** Icon-only mode for tight layouts */
  compact?: boolean;
  /** Selection source for analytics tracking */
  source: SelectionSource;
}

// ============================================================================
// Component
// ============================================================================

export function PlanSelectorInline({
  projectId,
  compact = false,
  source,
}: PlanSelectorInlineProps) {
  const [open, setOpen] = React.useState(false);
  const [searchQuery, setSearchQuery] = React.useState("");
  const [highlightedIndex, setHighlightedIndex] = React.useState(0);

  // Store state
  const activePlanId = usePlanStore(
    (state) => state.activePlanByProject[projectId] ?? null
  );
  const planCandidates = usePlanStore((state) => state.planCandidates);
  const isLoading = usePlanStore((state) => state.isLoading);
  const error = usePlanStore((state) => state.error);

  // Store actions
  const loadCandidates = usePlanStore((state) => state.loadCandidates);
  const setActivePlan = usePlanStore((state) => state.setActivePlan);
  const clearActivePlan = usePlanStore((state) => state.clearActivePlan);

  // Find active plan details
  const activePlan = React.useMemo(
    () => planCandidates.find((p) => p.sessionId === activePlanId),
    [planCandidates, activePlanId]
  );

  // Filter candidates by search query (client-side)
  const filteredCandidates = React.useMemo(() => {
    if (!searchQuery.trim()) return planCandidates;
    const query = searchQuery.toLowerCase();
    return planCandidates.filter(
      (p) => p.title?.toLowerCase().includes(query) ?? false
    );
  }, [planCandidates, searchQuery]);

  // Load candidates when popover opens (initial load)
  React.useEffect(() => {
    if (open) {
      loadCandidates(projectId);
      setHighlightedIndex(0);
    }
  }, [open, projectId, loadCandidates]);

  // Debounced search (300ms)
  React.useEffect(() => {
    if (!open) return;

    const timer = setTimeout(() => {
      if (searchQuery) {
        loadCandidates(projectId, searchQuery);
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [searchQuery, open, projectId, loadCandidates]);

  // Reset search query when popover closes
  React.useEffect(() => {
    if (!open) {
      setSearchQuery("");
    }
  }, [open]);

  // Handle plan selection
  const handleSelect = React.useCallback(
    async (sessionId: string) => {
      await setActivePlan(projectId, sessionId, source);
      setOpen(false);
    },
    [projectId, source, setActivePlan]
  );

  // Handle clear selection
  const handleClear = React.useCallback(async () => {
    await clearActivePlan(projectId);
    setOpen(false);
  }, [projectId, clearActivePlan]);

  // Handle retry
  const handleRetry = React.useCallback(() => {
    loadCandidates(projectId, searchQuery);
  }, [projectId, searchQuery, loadCandidates]);

  // Keyboard navigation
  const handleKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setHighlightedIndex((i) =>
            i < filteredCandidates.length - 1 ? i + 1 : 0
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setHighlightedIndex((i) =>
            i > 0 ? i - 1 : filteredCandidates.length - 1
          );
          break;
        case "Enter":
          e.preventDefault();
          if (highlightedIndex >= 0 && highlightedIndex < filteredCandidates.length) {
            const selectedCandidate = filteredCandidates[highlightedIndex];
            if (selectedCandidate) {
              handleSelect(selectedCandidate.sessionId);
            }
          }
          break;
        case "Escape":
          e.preventDefault();
          setOpen(false);
          break;
      }
    },
    [filteredCandidates, highlightedIndex, handleSelect]
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className={cn(
            "gap-1.5",
            !activePlanId && "text-muted-foreground"
          )}
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
      </PopoverTrigger>

      <PopoverContent align="start" className="w-80 p-0">
        <div className="flex flex-col">
          {/* Search Input */}
          <div className="p-3 border-b">
            <Input
              placeholder="Search plans..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              className="h-8"
              autoFocus
            />
          </div>

          {/* Candidate List */}
          {error ? (
            <div className="p-8 flex flex-col items-center justify-center gap-3 text-sm">
              <AlertCircle className="h-5 w-5 text-destructive" />
              <p className="text-muted-foreground text-center">{error}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={handleRetry}
                className="gap-2"
              >
                <RefreshCw className="h-4 w-4" />
                Retry
              </Button>
            </div>
          ) : isLoading ? (
            <div className="p-8 flex flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin" />
              <span>Loading plans...</span>
            </div>
          ) : filteredCandidates.length === 0 ? (
            <div className="p-8 flex flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
              <FileText className="h-5 w-5 opacity-50" />
              <span>{searchQuery ? "No accepted plans found" : "No accepted plans yet"}</span>
            </div>
          ) : (
            <ScrollArea className="max-h-64">
              <div className="p-2">
                {filteredCandidates.map((plan, index) => (
                  <PlanCandidateItem
                    key={plan.sessionId}
                    plan={plan}
                    isActive={plan.sessionId === activePlanId}
                    isHighlighted={index === highlightedIndex}
                    onSelect={() => handleSelect(plan.sessionId)}
                    onMouseEnter={() => setHighlightedIndex(index)}
                  />
                ))}
              </div>
            </ScrollArea>
          )}

          {/* Clear Selection Button */}
          {activePlanId && (
            <div className="p-2 border-t">
              <Button
                variant="ghost"
                onClick={handleClear}
                className="w-full justify-start text-sm"
                size="sm"
              >
                Clear selection
              </Button>
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// ============================================================================
// Plan Candidate Item Component
// ============================================================================

interface PlanCandidateItemProps {
  plan: PlanCandidate;
  isActive: boolean;
  isHighlighted: boolean;
  onSelect: () => void;
  onMouseEnter: () => void;
}

function PlanCandidateItem({
  plan,
  isActive,
  isHighlighted,
  onSelect,
  onMouseEnter,
}: PlanCandidateItemProps) {
  return (
    <button
      onClick={onSelect}
      onMouseEnter={onMouseEnter}
      className={cn(
        "w-full text-left px-3 py-2 rounded-md transition-colors",
        "hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        isHighlighted && "bg-accent",
        isActive && "bg-accent/50"
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="font-medium truncate">
            {plan.title || "Untitled Plan"}
          </div>
          <div className="text-sm text-muted-foreground flex items-center gap-2 mt-0.5">
            <span>
              {plan.taskStats.incomplete}/{plan.taskStats.total} incomplete
            </span>
            {plan.taskStats.activeNow > 0 && (
              <Badge variant="secondary" className="text-xs">
                Active
              </Badge>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}
