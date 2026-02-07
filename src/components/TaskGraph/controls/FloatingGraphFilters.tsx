/**
 * FloatingGraphFilters - Floating filter panel for Task Graph
 *
 * Provides controls stacked vertically in a glass container:
 * - Status filter (multi-select by status category)
 * - Plan filter (select specific plans)
 * - Layout direction toggle (TB ↔ LR)
 * - Node mode toggle (standard ↔ compact)
 * - Grouping options dropdown
 *
 * Positioned absolute, left side of canvas, vertically centered.
 */

import { memo, useState, useCallback } from "react";
import {
  Filter,
  ChevronDown,
  LayoutGrid,
  ArrowDownFromLine,
  ArrowRightFromLine,
  Layers,
  X,
  Maximize2,
  Minimize2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  STATUS_LEGEND_GROUPS,
  CATEGORY_LABELS,
  getCategoryColor,
  getNodeStyle,
  type StatusCategory,
} from "../nodes/nodeStyles";
import type { PlanGroupInfo } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import type {
  GraphFilters,
  LayoutDirection,
  GroupingState,
  NodeMode,
} from "./GraphControls";

// ============================================================================
// Types
// ============================================================================

export interface FloatingGraphFiltersProps {
  /** Current filter state */
  filters: GraphFilters;
  /** Callback when filters change */
  onFiltersChange: (filters: GraphFilters) => void;
  /** Current layout direction */
  layoutDirection: LayoutDirection;
  /** Callback when layout direction changes */
  onLayoutDirectionChange: (direction: LayoutDirection) => void;
  /** Current node display mode */
  nodeMode: NodeMode;
  /** Callback when node mode changes */
  onNodeModeChange: (mode: NodeMode) => void;
  /** Whether compact mode was auto-activated (50+ tasks) */
  isAutoCompact: boolean;
  /** Current grouping state */
  grouping: GroupingState;
  /** Callback when grouping changes */
  onGroupingChange: (grouping: GroupingState) => void;
  /** Available plan groups for filtering */
  planGroups: PlanGroupInfo[];
  /** Whether toolbar should be icon-only */
  isCompact: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const STATUS_CATEGORIES: StatusCategory[] = [
  "idle",
  "blocked",
  "executing",
  "qa",
  "review",
  "merge",
  "complete",
  "terminal",
];

const GROUPING_OPTIONS = [
  { key: "byPlan", label: "By Plan" },
  { key: "byTier", label: "By Tier" },
  { key: "showUncategorized", label: "Uncategorized" },
] as const;

const getGroupingLabel = (grouping: GroupingState): string => {
  if (!grouping.byPlan && !grouping.byTier) return "None";
  const labels: string[] = [];
  if (grouping.byPlan) labels.push("Plan");
  if (grouping.byTier) labels.push("Tier");
  return labels.join(" + ");
};

// ============================================================================
// Glass Container Style
// ============================================================================

const GLASS_STYLE: React.CSSProperties = {
  borderRadius: "10px",
  background: "hsla(220 10% 10% / 0.92)",
  backdropFilter: "blur(20px) saturate(180%)",
  border: "1px solid hsla(220 20% 100% / 0.08)",
  boxShadow:
    "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
};

// ============================================================================
// Sub-components
// ============================================================================

interface StatusFilterContentProps {
  filters: GraphFilters;
  onFiltersChange: (filters: GraphFilters) => void;
}

const StatusFilterContent = memo(function StatusFilterContent({
  filters,
  onFiltersChange,
}: StatusFilterContentProps) {
  const handleStatusToggle = useCallback(
    (status: InternalStatus) => {
      const newStatuses = filters.statuses.includes(status)
        ? filters.statuses.filter((s) => s !== status)
        : [...filters.statuses, status];
      onFiltersChange({ ...filters, statuses: newStatuses });
    },
    [filters, onFiltersChange]
  );

  const handleCategoryToggle = useCallback(
    (category: StatusCategory) => {
      const categoryStatuses = STATUS_LEGEND_GROUPS[category].map(
        (item) => item.status
      );
      const allSelected = categoryStatuses.every((s) =>
        filters.statuses.includes(s)
      );

      if (allSelected) {
        const newStatuses = filters.statuses.filter(
          (s) => !categoryStatuses.includes(s)
        );
        onFiltersChange({ ...filters, statuses: newStatuses });
      } else {
        const newStatuses = [
          ...new Set([...filters.statuses, ...categoryStatuses]),
        ];
        onFiltersChange({ ...filters, statuses: newStatuses });
      }
    },
    [filters, onFiltersChange]
  );

  const handleClearAll = useCallback(() => {
    onFiltersChange({ ...filters, statuses: [] });
  }, [filters, onFiltersChange]);

  const handleShowArchivedToggle = useCallback(() => {
    onFiltersChange({ ...filters, showArchived: !filters.showArchived });
  }, [filters, onFiltersChange]);

  return (
    <div className="space-y-3">
      {filters.statuses.length > 0 && (
        <div className="flex justify-end">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearAll}
            className="h-6 px-2 text-xs text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_80%)]"
          >
            <X className="w-3 h-3 mr-1" />
            Clear all
          </Button>
        </div>
      )}

      {STATUS_CATEGORIES.map((category) => {
        const items = STATUS_LEGEND_GROUPS[category];
        const categoryColor = getCategoryColor(category);
        const categoryLabel = CATEGORY_LABELS[category];
        const someSelected = items.some((item) =>
          filters.statuses.includes(item.status)
        );

        return (
          <div key={category} className="space-y-1">
            <button
              onClick={() => handleCategoryToggle(category)}
              className="flex items-center gap-2 w-full hover:bg-[hsl(220_10%_15%)] rounded px-1 py-0.5 transition-colors"
            >
              <div
                className="w-2.5 h-2.5 rounded-sm"
                style={{ backgroundColor: categoryColor }}
              />
              <span
                className="text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: categoryColor }}
              >
                {categoryLabel}
              </span>
              {someSelected && (
                <span className="ml-auto text-[10px] text-[hsl(220_10%_50%)]">
                  {
                    items.filter((item) =>
                      filters.statuses.includes(item.status)
                    ).length
                  }
                  /{items.length}
                </span>
              )}
            </button>

            <div className="pl-4 space-y-0.5">
              {items.map((item) => {
                const style = getNodeStyle(item.status);
                const isSelected = filters.statuses.includes(item.status);

                return (
                  <label
                    key={item.status}
                    className="flex items-center gap-2 py-0.5 cursor-pointer hover:bg-[hsl(220_10%_12%)] rounded px-1 transition-colors"
                  >
                    <Checkbox
                      checked={isSelected}
                      onCheckedChange={() => handleStatusToggle(item.status)}
                      className="h-3.5 w-3.5"
                    />
                    <div
                      className="w-2.5 h-2.5 rounded-sm border"
                      style={{
                        borderColor: style.borderColor,
                        backgroundColor: style.backgroundColor,
                      }}
                    />
                    <span className="text-xs text-[hsl(220_10%_70%)]">
                      {item.label}
                    </span>
                  </label>
                );
              })}
            </div>
          </div>
        );
      })}

      <div className="pt-2 border-t border-[hsl(220_10%_20%)]">
        <label className="flex items-center gap-2 cursor-pointer">
          <Checkbox
            checked={filters.showArchived}
            onCheckedChange={handleShowArchivedToggle}
            className="h-3.5 w-3.5"
          />
          <span className="text-xs text-[hsl(220_10%_70%)]">
            Show archived tasks
          </span>
        </label>
      </div>
    </div>
  );
});

interface PlanFilterContentProps {
  filters: GraphFilters;
  onFiltersChange: (filters: GraphFilters) => void;
  planGroups: PlanGroupInfo[];
}

const PlanFilterContent = memo(function PlanFilterContent({
  filters,
  onFiltersChange,
  planGroups,
}: PlanFilterContentProps) {
  const handlePlanToggle = useCallback(
    (planId: string) => {
      const newPlanIds = filters.planIds.includes(planId)
        ? filters.planIds.filter((id) => id !== planId)
        : [...filters.planIds, planId];
      onFiltersChange({ ...filters, planIds: newPlanIds });
    },
    [filters, onFiltersChange]
  );

  const handleClearAll = useCallback(() => {
    onFiltersChange({ ...filters, planIds: [] });
  }, [filters, onFiltersChange]);

  if (planGroups.length === 0) {
    return (
      <div className="py-4 text-center">
        <p className="text-xs text-[hsl(220_10%_50%)]">No plans available</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {filters.planIds.length > 0 && (
        <div className="flex justify-end">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearAll}
            className="h-6 px-2 text-xs text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_80%)]"
          >
            <X className="w-3 h-3 mr-1" />
            Clear all
          </Button>
        </div>
      )}

      <div className="space-y-1 max-h-[200px] overflow-y-auto">
        {planGroups.map((plan) => {
          const isSelected = filters.planIds.includes(plan.planArtifactId);
          const taskCount = plan.taskIds.length;
          const completedCount = plan.statusSummary.completed;

          return (
            <label
              key={plan.planArtifactId}
              className="flex items-center gap-2 py-1.5 px-2 cursor-pointer hover:bg-[hsl(220_10%_15%)] rounded transition-colors"
            >
              <Checkbox
                checked={isSelected}
                onCheckedChange={() => handlePlanToggle(plan.planArtifactId)}
                className="h-3.5 w-3.5"
              />
              <div className="flex-1 min-w-0">
                <p className="text-xs text-[hsl(220_10%_80%)] truncate">
                  {plan.sessionTitle ?? "Unnamed Plan"}
                </p>
                <p className="text-[10px] text-[hsl(220_10%_50%)]">
                  {completedCount}/{taskCount} tasks
                </p>
              </div>
            </label>
          );
        })}
      </div>
    </div>
  );
});

// ============================================================================
// Filter Button Component
// ============================================================================

interface FilterButtonProps {
  icon: React.ReactNode;
  label: string;
  activeCount?: number;
  onClick?: () => void;
  tooltip?: string;
  children?: React.ReactNode;
  isPopover?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  isCompact?: boolean;
}

const FilterButton = memo(function FilterButton({
  icon,
  label,
  activeCount,
  onClick,
  tooltip,
  children,
  isPopover,
  open,
  onOpenChange,
  isCompact,
}: FilterButtonProps) {
  const buttonContent = (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-1.5 w-full px-2.5 py-2 rounded-md text-xs transition-colors",
        "bg-[hsl(220_10%_14%)] hover:bg-[hsl(220_10%_18%)]",
        isCompact && "w-full h-8 p-0 justify-center",
        activeCount && activeCount > 0 && "ring-1 ring-[hsl(14_100%_55%_/_0.5)]"
      )}
    >
      <span className="text-[hsl(220_10%_50%)]">{icon}</span>
      {!isCompact && (
        <span className="text-[hsl(220_10%_70%)] flex-1 text-left">{label}</span>
      )}
      {activeCount !== undefined && activeCount > 0 && !isCompact && (
        <span className="text-[10px] text-[hsl(14_100%_55%)]">
          {activeCount}
        </span>
      )}
      {isPopover && !isCompact && (
        <ChevronDown className="w-3 h-3 text-[hsl(220_10%_50%)]" />
      )}
    </button>
  );

  if (isPopover && children && open !== undefined && onOpenChange) {
    const popoverContent = (
      <Popover open={open} onOpenChange={onOpenChange}>
        <PopoverTrigger asChild>{buttonContent}</PopoverTrigger>
        <PopoverContent
          className="w-64 p-3 bg-[hsl(220_10%_10%)] border-[hsl(220_10%_25%)]"
          align="start"
          side="right"
          sideOffset={8}
        >
          {children}
        </PopoverContent>
      </Popover>
    );

    if (tooltip || isCompact) {
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>{popoverContent}</TooltipTrigger>
            <TooltipContent side="right" className="text-xs">
              {tooltip ?? label}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      );
    }

    return popoverContent;
  }

  if (tooltip) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>{buttonContent}</TooltipTrigger>
          <TooltipContent side="right" className="text-xs">
            {tooltip}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return buttonContent;
});

// ============================================================================
// Main Component
// ============================================================================

function FloatingGraphFiltersComponent({
  filters,
  onFiltersChange,
  layoutDirection,
  onLayoutDirectionChange,
  nodeMode,
  onNodeModeChange,
  isAutoCompact,
  grouping,
  onGroupingChange,
  planGroups,
  isCompact,
}: FloatingGraphFiltersProps) {
  const [statusFilterOpen, setStatusFilterOpen] = useState(false);
  const [planFilterOpen, setPlanFilterOpen] = useState(false);
  const [groupingOpen, setGroupingOpen] = useState(false);

  const activeStatusCount = filters.statuses.length;
  const activePlanCount = filters.planIds.length;

  const handleLayoutToggle = useCallback(() => {
    onLayoutDirectionChange(layoutDirection === "TB" ? "LR" : "TB");
  }, [layoutDirection, onLayoutDirectionChange]);

  const handleNodeModeToggle = useCallback(() => {
    onNodeModeChange(nodeMode === "standard" ? "compact" : "standard");
  }, [nodeMode, onNodeModeChange]);

  const currentGroupingLabel = getGroupingLabel(grouping);

  return (
    <div
      className="absolute z-10"
      style={{
        left: "16px",
        top: "50%",
        transform: "translateY(-50%)",
      }}
      data-testid="floating-graph-filters"
    >
      <div className={cn("p-2", isCompact && "p-1.5")} style={GLASS_STYLE}>
        <div
          className={cn(
            "flex flex-col gap-1.5",
            isCompact && "gap-1",
            isCompact ? "w-[36px]" : "w-[120px]"
          )}
        >
          {/* Status Filter */}
          <FilterButton
            icon={<Filter className="w-3.5 h-3.5" />}
            label="Status"
            activeCount={activeStatusCount}
            isPopover
            open={statusFilterOpen}
            onOpenChange={setStatusFilterOpen}
            isCompact={isCompact}
          >
            <StatusFilterContent
              filters={filters}
              onFiltersChange={onFiltersChange}
            />
          </FilterButton>

          {/* Plan Filter */}
          {planGroups.length > 0 && (
            <FilterButton
              icon={<LayoutGrid className="w-3.5 h-3.5" />}
              label="Plans"
              activeCount={activePlanCount}
              isPopover
              open={planFilterOpen}
              onOpenChange={setPlanFilterOpen}
              isCompact={isCompact}
            >
              <PlanFilterContent
                filters={filters}
                onFiltersChange={onFiltersChange}
                planGroups={planGroups}
              />
            </FilterButton>
          )}

          {/* Separator */}
          <div
            className={cn(
              "h-px bg-[hsl(220_10%_25%)] my-1",
              isCompact && "my-0.5"
            )}
          />

          {/* Layout Direction Toggle */}
          <FilterButton
            icon={
              layoutDirection === "TB" ? (
                <ArrowDownFromLine className="w-3.5 h-3.5" />
              ) : (
                <ArrowRightFromLine className="w-3.5 h-3.5" />
              )
            }
            label={layoutDirection === "TB" ? "Vertical" : "Horizontal"}
            onClick={handleLayoutToggle}
            tooltip={
              layoutDirection === "TB"
                ? "Switch to horizontal layout"
                : "Switch to vertical layout"
            }
            isCompact={isCompact}
          />

          {/* Node Mode Toggle */}
          <FilterButton
            icon={
              nodeMode === "compact" ? (
                <Minimize2 className="w-3.5 h-3.5" />
              ) : (
                <Maximize2 className="w-3.5 h-3.5" />
              )
            }
            label={nodeMode === "compact" ? "Compact" : "Standard"}
            activeCount={isAutoCompact && nodeMode === "compact" ? 1 : 0}
            onClick={handleNodeModeToggle}
            tooltip={
              nodeMode === "standard"
                ? "Switch to compact nodes"
                : isAutoCompact
                  ? "Auto-compacted (50+ tasks)"
                  : "Switch to standard nodes"
            }
            isCompact={isCompact}
          />

          {/* Grouping Dropdown */}
          <Popover open={groupingOpen} onOpenChange={setGroupingOpen}>
            {(() => {
              const groupingButton = (
                <button
                  className={cn(
                    "flex items-center gap-1.5 w-full px-2.5 py-2 rounded-md text-xs transition-colors",
                    "bg-[hsl(220_10%_14%)] hover:bg-[hsl(220_10%_18%)]",
                    isCompact && "w-full h-8 p-0 justify-center"
                  )}
                >
                  <Layers className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
                  {!isCompact && (
                    <span className="text-[hsl(220_10%_70%)] flex-1 text-left">
                      {currentGroupingLabel}
                    </span>
                  )}
                  {!isCompact && (
                    <ChevronDown className="w-3 h-3 text-[hsl(220_10%_50%)]" />
                  )}
                </button>
              );

              if (!isCompact) {
                return <PopoverTrigger asChild>{groupingButton}</PopoverTrigger>;
              }

              return (
                <TooltipProvider>
                  <Tooltip>
                    <PopoverTrigger asChild>
                      <TooltipTrigger asChild>{groupingButton}</TooltipTrigger>
                    </PopoverTrigger>
                    <TooltipContent side="right" className="text-xs">
                      Grouping
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              );
            })()}
            <PopoverContent
              className="w-36 p-1 bg-[hsl(220_10%_10%)] border-[hsl(220_10%_25%)] z-50"
              align="start"
              side="right"
              sideOffset={8}
            >
              <div className="flex items-center justify-between px-2 py-1">
                <span className="text-[10px] text-[hsl(220_10%_50%)] uppercase tracking-wide">
                  Grouping
                </span>
                <button
                  onClick={() => onGroupingChange({ byPlan: false, byTier: false, showUncategorized: false })}
                  className="text-[10px] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_80%)]"
                >
                  None
                </button>
              </div>
              <div className="space-y-1 px-1 pb-1">
                {GROUPING_OPTIONS.map((option) => {
                  const isChecked = grouping[option.key];
                  const isDisabled = option.key === "showUncategorized" && !grouping.byPlan;
                  return (
                    <label
                      key={option.key}
                      className={cn(
                        "flex items-center gap-2 px-2 py-1.5 rounded text-xs transition-colors cursor-pointer",
                        "text-[hsl(220_10%_70%)] hover:bg-[hsl(220_10%_15%)]",
                        isDisabled && "opacity-50 cursor-not-allowed"
                      )}
                    >
                      <Checkbox
                        checked={isChecked}
                        onCheckedChange={(checked) => {
                          if (isDisabled) return;
                          onGroupingChange({
                            ...grouping,
                            [option.key]: Boolean(checked),
                          });
                        }}
                      />
                      {option.label}
                    </label>
                  );
                })}
              </div>
            </PopoverContent>
          </Popover>
        </div>
      </div>
    </div>
  );
}

/**
 * Memoized FloatingGraphFilters component
 */
export const FloatingGraphFilters = memo(FloatingGraphFiltersComponent);
