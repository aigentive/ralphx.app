/* eslint-disable react-refresh/only-export-components */
/**
 * GraphControls - Filter, layout direction, and grouping controls for Task Graph
 *
 * Provides controls for:
 * - Status filter (multi-select by status category)
 * - Plan filter (select specific plans)
 * - Layout direction toggle (TB ↔ LR)
 * - Grouping options (by plan, tier, status, none)
 *
 * @see specs/plans/task_graph_view.md section "Task E.3" and "Filtering & Grouping"
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
  STATUS_LEGEND_GROUPS,
  CATEGORY_LABELS,
  getCategoryColor,
  getNodeStyle,
  type StatusCategory,
} from "../nodes/nodeStyles";
import type { PlanGroupInfo } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// Types
// ============================================================================

export type LayoutDirection = "TB" | "LR";

export type GroupingState = {
  byPlan: boolean;
  byTier: boolean;
  showUncategorized: boolean;
};

export interface GraphFilters {
  /** Selected status values (empty = show all) */
  statuses: InternalStatus[];
  /** Selected plan artifact IDs (empty = show all) */
  planIds: string[];
  /** Whether to include archived tasks (fetched from backend) */
  showArchived: boolean;
}

export type NodeMode = "standard" | "compact";

export interface GraphControlsProps {
  /** Current filter state */
  filters: GraphFilters;
  /** Callback when filters change */
  onFiltersChange: (filters: GraphFilters) => void;
  /** Current layout direction */
  layoutDirection: LayoutDirection;
  /** Callback when layout direction changes */
  onLayoutDirectionChange: (direction: LayoutDirection) => void;
  /** Current grouping option */
  grouping: GroupingState;
  /** Callback when grouping changes */
  onGroupingChange: (grouping: GroupingState) => void;
  /** Current node display mode */
  nodeMode: NodeMode;
  /** Callback when node mode changes */
  onNodeModeChange: (mode: NodeMode) => void;
  /** Whether compact mode was auto-activated (50+ tasks) */
  isAutoCompact: boolean;
  /** Available plan groups for filtering */
  planGroups: PlanGroupInfo[];
  /** Additional className for the container */
  className?: string;
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
  { key: "byPlan", label: "By Plan", description: "Group tasks by originating plan" },
  { key: "byTier", label: "By Tier", description: "Group by dependency tier" },
  { key: "showUncategorized", label: "Uncategorized", description: "Include uncategorized tasks" },
] as const;

const getGroupingLabel = (grouping: GroupingState): string => {
  const active: string[] = [];
  if (grouping.byPlan) active.push("Plan");
  if (grouping.byTier) active.push("Tier");
  if (!grouping.byPlan && !grouping.byTier) return "None";
  return active.join(" + ");
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
      const categoryStatuses = STATUS_LEGEND_GROUPS[category].map((item) => item.status);
      const allSelected = categoryStatuses.every((s) => filters.statuses.includes(s));

      if (allSelected) {
        // Remove all statuses in this category
        const newStatuses = filters.statuses.filter((s) => !categoryStatuses.includes(s));
        onFiltersChange({ ...filters, statuses: newStatuses });
      } else {
        // Add all statuses in this category
        const newStatuses = [...new Set([...filters.statuses, ...categoryStatuses])];
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
      {/* Clear all button */}
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

      {/* Status categories */}
      {STATUS_CATEGORIES.map((category) => {
        const items = STATUS_LEGEND_GROUPS[category];
        const categoryColor = getCategoryColor(category);
        const categoryLabel = CATEGORY_LABELS[category];
        const someSelected = items.some((item) => filters.statuses.includes(item.status));

        return (
          <div key={category} className="space-y-1">
            {/* Category header with toggle */}
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
                  {items.filter((item) => filters.statuses.includes(item.status)).length}/{items.length}
                </span>
              )}
            </button>

            {/* Individual status items */}
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
                    <span className="text-xs text-[hsl(220_10%_70%)]">{item.label}</span>
                  </label>
                );
              })}
            </div>
          </div>
        );
      })}

      {/* Show archived toggle */}
      <div className="pt-2 border-t border-[hsl(220_10%_20%)]">
        <label className="flex items-center gap-2 cursor-pointer">
          <Checkbox
            checked={filters.showArchived}
            onCheckedChange={handleShowArchivedToggle}
            className="h-3.5 w-3.5"
          />
          <span className="text-xs text-[hsl(220_10%_70%)]">Show archived tasks</span>
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
      {/* Clear all button */}
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

      {/* Plan items */}
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

interface GroupingDropdownProps {
  grouping: GroupingState;
  onGroupingChange: (grouping: GroupingState) => void;
}

const GroupingDropdown = memo(function GroupingDropdown({
  grouping,
  onGroupingChange,
}: GroupingDropdownProps) {
  const [open, setOpen] = useState(false);

  const currentOption = getGroupingLabel(grouping);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          className={cn(
            "flex items-center gap-1.5 px-2 py-1.5 rounded text-xs transition-colors",
            "bg-[hsl(220_10%_12%)] border border-[hsl(220_10%_25%)]",
            "hover:bg-[hsl(220_10%_15%)] hover:border-[hsl(220_10%_30%)]"
          )}
        >
          <Layers className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
          <span className="text-[hsl(220_10%_70%)]">{currentOption}</span>
          <ChevronDown className="w-3 h-3 text-[hsl(220_10%_50%)]" />
        </button>
      </PopoverTrigger>
      <PopoverContent
        className="w-56 p-1 bg-[hsl(220_10%_10%)] border-[hsl(220_10%_25%)]"
        align="start"
      >
        <div className="flex items-center justify-between px-2 py-1.5">
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
                  "flex items-start gap-2 px-2 py-2 rounded transition-colors cursor-pointer",
                  "hover:bg-[hsl(220_10%_15%)]",
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
                <div className="flex-1">
                  <div className="text-xs text-[hsl(220_10%_80%)]">{option.label}</div>
                  <div className="text-[10px] text-[hsl(220_10%_50%)]">{option.description}</div>
                </div>
              </label>
            );
          })}
        </div>
      </PopoverContent>
    </Popover>
  );
});

// ============================================================================
// Main Component
// ============================================================================

function GraphControlsComponent({
  filters,
  onFiltersChange,
  layoutDirection,
  onLayoutDirectionChange,
  grouping,
  onGroupingChange,
  nodeMode,
  onNodeModeChange,
  isAutoCompact,
  planGroups,
  className = "",
}: GraphControlsProps) {
  const [statusFilterOpen, setStatusFilterOpen] = useState(false);
  const [planFilterOpen, setPlanFilterOpen] = useState(false);

  const activeStatusCount = filters.statuses.length;
  const activePlanCount = filters.planIds.length;

  const handleLayoutToggle = useCallback(() => {
    onLayoutDirectionChange(layoutDirection === "TB" ? "LR" : "TB");
  }, [layoutDirection, onLayoutDirectionChange]);

  const handleNodeModeToggle = useCallback(() => {
    onNodeModeChange(nodeMode === "standard" ? "compact" : "standard");
  }, [nodeMode, onNodeModeChange]);

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-2",
        "bg-[hsl(220_10%_10%_/_0.9)] backdrop-blur-sm",
        "border-b border-[hsl(220_10%_25%)]",
        className
      )}
      data-testid="graph-controls"
    >
      {/* Status Filter */}
      <Popover open={statusFilterOpen} onOpenChange={setStatusFilterOpen}>
        <PopoverTrigger asChild>
          <button
            className={cn(
              "flex items-center gap-1.5 px-2 py-1.5 rounded text-xs transition-colors",
              "bg-[hsl(220_10%_12%)] border border-[hsl(220_10%_25%)]",
              "hover:bg-[hsl(220_10%_15%)] hover:border-[hsl(220_10%_30%)]",
              activeStatusCount > 0 && "border-[hsl(14_100%_55%)]"
            )}
          >
            <Filter className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
            <span className="text-[hsl(220_10%_70%)]">
              Status
              {activeStatusCount > 0 && (
                <span className="ml-1 text-[hsl(14_100%_55%)]">({activeStatusCount})</span>
              )}
            </span>
            <ChevronDown className="w-3 h-3 text-[hsl(220_10%_50%)]" />
          </button>
        </PopoverTrigger>
        <PopoverContent
          className="w-64 p-3 bg-[hsl(220_10%_10%)] border-[hsl(220_10%_25%)]"
          align="start"
        >
          <StatusFilterContent filters={filters} onFiltersChange={onFiltersChange} />
        </PopoverContent>
      </Popover>

      {/* Plan Filter */}
      {planGroups.length > 0 && (
        <Popover open={planFilterOpen} onOpenChange={setPlanFilterOpen}>
          <PopoverTrigger asChild>
            <button
              className={cn(
                "flex items-center gap-1.5 px-2 py-1.5 rounded text-xs transition-colors",
                "bg-[hsl(220_10%_12%)] border border-[hsl(220_10%_25%)]",
                "hover:bg-[hsl(220_10%_15%)] hover:border-[hsl(220_10%_30%)]",
                activePlanCount > 0 && "border-[hsl(14_100%_55%)]"
              )}
            >
              <LayoutGrid className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
              <span className="text-[hsl(220_10%_70%)]">
                Plans
                {activePlanCount > 0 && (
                  <span className="ml-1 text-[hsl(14_100%_55%)]">({activePlanCount})</span>
                )}
              </span>
              <ChevronDown className="w-3 h-3 text-[hsl(220_10%_50%)]" />
            </button>
          </PopoverTrigger>
          <PopoverContent
            className="w-56 p-3 bg-[hsl(220_10%_10%)] border-[hsl(220_10%_25%)]"
            align="start"
          >
            <PlanFilterContent
              filters={filters}
              onFiltersChange={onFiltersChange}
              planGroups={planGroups}
            />
          </PopoverContent>
        </Popover>
      )}

      {/* Separator */}
      <div className="h-5 w-px bg-[hsl(220_10%_25%)]" />

      {/* Layout Direction Toggle */}
      <button
        onClick={handleLayoutToggle}
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 rounded text-xs transition-colors",
          "bg-[hsl(220_10%_12%)] border border-[hsl(220_10%_25%)]",
          "hover:bg-[hsl(220_10%_15%)] hover:border-[hsl(220_10%_30%)]"
        )}
        title={layoutDirection === "TB" ? "Switch to horizontal layout" : "Switch to vertical layout"}
      >
        {layoutDirection === "TB" ? (
          <ArrowDownFromLine className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
        ) : (
          <ArrowRightFromLine className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
        )}
        <span className="text-[hsl(220_10%_70%)]">{layoutDirection}</span>
      </button>

      {/* Node Mode Toggle */}
      <button
        onClick={handleNodeModeToggle}
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 rounded text-xs transition-colors",
          "bg-[hsl(220_10%_12%)] border border-[hsl(220_10%_25%)]",
          "hover:bg-[hsl(220_10%_15%)] hover:border-[hsl(220_10%_30%)]",
          isAutoCompact && nodeMode === "compact" && "border-[hsl(14_100%_55%_/_0.5)]"
        )}
        title={
          nodeMode === "standard"
            ? "Switch to compact nodes"
            : isAutoCompact
              ? "Auto-compacted (50+ tasks) - click to use standard nodes"
              : "Switch to standard nodes"
        }
      >
        {nodeMode === "compact" ? (
          <Minimize2 className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
        ) : (
          <Maximize2 className="w-3.5 h-3.5 text-[hsl(220_10%_50%)]" />
        )}
        <span className="text-[hsl(220_10%_70%)]">
          {nodeMode === "compact" ? "Compact" : "Standard"}
        </span>
        {isAutoCompact && nodeMode === "compact" && (
          <span className="text-[10px] text-[hsl(14_100%_55%)]">(auto)</span>
        )}
      </button>

      {/* Grouping Dropdown */}
      <GroupingDropdown grouping={grouping} onGroupingChange={onGroupingChange} />
    </div>
  );
}

/**
 * Memoized GraphControls component
 */
export const GraphControls = memo(GraphControlsComponent);

// ============================================================================
// Default Values
// ============================================================================

/**
 * Default filter state (show all)
 */
export const DEFAULT_GRAPH_FILTERS: GraphFilters = {
  statuses: [],
  planIds: [],
  showArchived: false,
};

/**
 * Default layout direction
 */
export const DEFAULT_LAYOUT_DIRECTION: LayoutDirection = "TB";

/**
 * Default grouping option
 */
export const DEFAULT_GROUPING: GroupingState = {
  byPlan: true,
  byTier: true,
  showUncategorized: true,
};

/**
 * Default node display mode
 */
export const DEFAULT_NODE_MODE: NodeMode = "standard";

/**
 * Threshold for auto-switching to compact mode
 */
export const COMPACT_MODE_THRESHOLD = 50;
