/**
 * ExecutionTimeline - Collapsible side panel showing chronological task execution history
 *
 * Features:
 * - Chronological list of task events (status changes, plan events)
 * - Click entry to highlight corresponding node in graph
 * - Filter by event type and status category (execution, reviews, escalations, QA, etc.)
 * - Real-time updates via useExecutionTimeline hook
 * - Collapsible panel
 *
 * @see specs/plans/task_graph_view.md section "Task D.4" and "Execution Timeline Panel"
 */

import { memo, useState, useCallback, useMemo } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Clock,
  Loader2,
  AlertCircle,
  RefreshCw,
  Filter,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { TimelineEntry } from "./TimelineEntry";
import { useExecutionTimeline, type TimelineFilters } from "../hooks/useExecutionTimeline";
import {
  type TimelineFilterCategory,
  TIMELINE_FILTER_OPTIONS,
  filterTimelineEvents,
  toApiFilters,
} from "./timelineFilters";

// ============================================================================
// Types
// ============================================================================

export interface ExecutionTimelineProps {
  /** Project ID to fetch timeline events for */
  projectId: string;
  /** Callback when clicking on a task entry (for node highlighting in graph) */
  onTaskClick?: (taskId: string) => void;
  /** Currently highlighted task ID (from graph selection) */
  highlightedTaskId?: string | null;
  /** Default collapsed state (ignored when embedded) */
  defaultCollapsed?: boolean;
  /** Additional className for the container */
  className?: string;
  /**
   * Embedded mode - renders without outer width/collapse controls.
   * Use when wrapping in FloatingTimeline or similar container.
   */
  embedded?: boolean;
}

export interface TimelineFilterState {
  /** Selected filter categories (empty = show all) */
  categories: TimelineFilterCategory[];
  /** Whether the filter panel is expanded */
  isExpanded: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const PANEL_WIDTH_EXPANDED = 320;
const PANEL_WIDTH_COLLAPSED = 40;

// ============================================================================
// Sub-components
// ============================================================================

interface TimelineFilterBarProps {
  filters: TimelineFilterState;
  onFilterChange: (filters: TimelineFilterState) => void;
}

const TimelineFilterBar = memo(function TimelineFilterBar({
  filters,
  onFilterChange,
}: TimelineFilterBarProps) {
  const handleToggleExpanded = useCallback(() => {
    onFilterChange({
      ...filters,
      isExpanded: !filters.isExpanded,
    });
  }, [filters, onFilterChange]);

  const handleCategoryToggle = useCallback(
    (categoryId: TimelineFilterCategory) => {
      // "all" clears other filters
      if (categoryId === "all") {
        onFilterChange({
          ...filters,
          categories: [],
        });
        return;
      }

      // Toggle the category
      const newCategories = filters.categories.includes(categoryId)
        ? filters.categories.filter((c) => c !== categoryId)
        : [...filters.categories, categoryId];

      onFilterChange({
        ...filters,
        categories: newCategories,
      });
    },
    [filters, onFilterChange]
  );

  const activeCount = filters.categories.length;
  const isShowingAll = activeCount === 0;

  return (
    <div
      style={{
        borderBottom: "1px solid hsla(220 10% 100% / 0.04)",
      }}
    >
      {/* Compact filter header with toggle */}
      <button
        onClick={handleToggleExpanded}
        className="w-full flex items-center justify-between px-3 py-2 transition-colors"
        style={{
          background: "transparent",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = "hsl(220 10% 16%)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = "transparent";
        }}
      >
        <div className="flex items-center gap-2">
          <Filter
            className="w-3.5 h-3.5"
            style={{ color: "hsl(220 10% 45%)" }}
          />
          <span
            style={{
              fontSize: "11px",
              fontWeight: 500,
              color: "hsl(220 10% 60%)",
            }}
          >
            {isShowingAll ? "All events" : `${activeCount} filter${activeCount > 1 ? "s" : ""}`}
          </span>
        </div>
        <ChevronRight
          className={cn(
            "w-3.5 h-3.5 transition-transform",
            filters.isExpanded && "rotate-90"
          )}
          style={{ color: "hsl(220 10% 45%)" }}
        />
      </button>

      {/* Expanded filter options */}
      {filters.isExpanded && (
        <div
          className="px-2 py-2 space-y-0.5"
          style={{ background: "hsl(220 10% 8%)" }}
        >
          {TIMELINE_FILTER_OPTIONS.map((option) => {
            const isActive =
              option.id === "all"
                ? isShowingAll
                : filters.categories.includes(option.id);

            return (
              <button
                key={option.id}
                onClick={() => handleCategoryToggle(option.id)}
                className="w-full flex items-center gap-2 px-2 py-1.5 rounded text-left transition-colors"
                style={{
                  background: isActive ? "hsla(220 60% 50% / 0.12)" : "transparent",
                  borderRadius: "6px",
                }}
                onMouseEnter={(e) => {
                  if (!isActive) {
                    e.currentTarget.style.background = "hsl(220 10% 14%)";
                  }
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = isActive ? "hsla(220 60% 50% / 0.12)" : "transparent";
                }}
                aria-pressed={isActive}
              >
                {/* Color indicator */}
                <div
                  className="w-2 h-2 rounded-full flex-shrink-0 transition-opacity"
                  style={{
                    backgroundColor: option.color,
                    opacity: isActive ? 1 : 0.4,
                  }}
                />

                {/* Label */}
                <span
                  className="flex-1"
                  style={{
                    fontSize: "11px",
                    fontWeight: 500,
                    color: isActive ? "hsl(220 10% 90%)" : "hsl(220 10% 55%)",
                  }}
                >
                  {option.label}
                </span>

                {/* Checkmark for active */}
                {isActive && option.id !== "all" && (
                  <span
                    style={{
                      fontSize: "10px",
                      color: "hsl(220 80% 65%)",
                    }}
                  >
                    ✓
                  </span>
                )}
              </button>
            );
          })}

          {/* Clear filters button */}
          {!isShowingAll && (
            <button
              onClick={() => handleCategoryToggle("all")}
              className="w-full mt-1 py-1.5 transition-colors"
              style={{
                fontSize: "10px",
                color: "hsl(220 10% 50%)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = "hsl(220 10% 80%)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = "hsl(220 10% 50%)";
              }}
            >
              Clear filters
            </button>
          )}
        </div>
      )}
    </div>
  );
});

interface TimelineHeaderProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  onRefresh: () => void;
  isRefreshing: boolean;
  eventCount: number;
  /** Hide collapse toggle (for embedded mode) */
  hideCollapseToggle?: boolean;
}

const TimelineHeader = memo(function TimelineHeader({
  collapsed,
  onToggleCollapse,
  onRefresh,
  isRefreshing,
  eventCount,
  hideCollapseToggle = false,
}: TimelineHeaderProps) {
  return (
    <div
      className="flex items-center justify-between px-3"
      style={{
        height: "44px",
        background: "hsla(220 15% 5% / 0.5)",
        borderBottom: "1px solid hsla(220 10% 100% / 0.04)",
      }}
    >
      <div className="flex items-center gap-2">
        <Clock
          className="w-4 h-4"
          style={{ color: "hsl(220 10% 50%)" }}
        />
        {!collapsed && (
          <>
            <span
              style={{
                fontSize: "13px",
                fontWeight: 500,
                color: "hsl(220 10% 90%)",
                letterSpacing: "-0.01em",
              }}
            >
              Timeline
            </span>
            <span
              style={{
                fontSize: "11px",
                fontWeight: 500,
                color: "hsl(220 10% 45%)",
              }}
            >
              {eventCount}
            </span>
          </>
        )}
      </div>
      {!collapsed && (
        <div className="flex items-center gap-0.5">
          <button
            onClick={onRefresh}
            disabled={isRefreshing}
            className="p-1.5 rounded-md transition-colors disabled:opacity-50"
            style={{
              color: "hsl(220 10% 50%)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsl(220 10% 16%)";
              e.currentTarget.style.color = "hsl(220 10% 85%)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "transparent";
              e.currentTarget.style.color = "hsl(220 10% 50%)";
            }}
            title="Refresh timeline"
          >
            <RefreshCw
              className={cn("w-3.5 h-3.5", isRefreshing && "animate-spin")}
            />
          </button>
          {!hideCollapseToggle && (
            <button
              onClick={onToggleCollapse}
              className="p-1.5 rounded-md transition-colors"
              style={{
                color: "hsl(220 10% 50%)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "hsl(220 10% 16%)";
                e.currentTarget.style.color = "hsl(220 10% 85%)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
                e.currentTarget.style.color = "hsl(220 10% 50%)";
              }}
              title="Collapse timeline"
            >
              <ChevronRight className="w-4 h-4" />
            </button>
          )}
        </div>
      )}
      {collapsed && !hideCollapseToggle && (
        <button
          onClick={onToggleCollapse}
          className="p-1.5 rounded-md transition-colors"
          style={{
            color: "hsl(220 10% 50%)",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "hsl(220 10% 16%)";
            e.currentTarget.style.color = "hsl(220 10% 85%)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
            e.currentTarget.style.color = "hsl(220 10% 50%)";
          }}
          title="Expand timeline"
        >
          <ChevronLeft className="w-4 h-4" />
        </button>
      )}
    </div>
  );
});

interface TimelineLoadMoreProps {
  hasMore: boolean;
  isFetching: boolean;
  onLoadMore: () => void;
}

const TimelineLoadMore = memo(function TimelineLoadMore({
  hasMore,
  isFetching,
  onLoadMore,
}: TimelineLoadMoreProps) {
  if (!hasMore) return null;

  return (
    <div className="px-3 py-2">
      <button
        onClick={onLoadMore}
        disabled={isFetching}
        className="w-full py-2 rounded-md transition-colors disabled:opacity-50"
        style={{
          fontSize: "11px",
          fontWeight: 500,
          color: "hsl(220 10% 55%)",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = "hsl(220 10% 16%)";
          e.currentTarget.style.color = "hsl(220 10% 85%)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = "transparent";
          e.currentTarget.style.color = "hsl(220 10% 55%)";
        }}
      >
        {isFetching ? (
          <span className="flex items-center justify-center gap-2">
            <Loader2 className="w-3 h-3 animate-spin" />
            Loading...
          </span>
        ) : (
          "Load more"
        )}
      </button>
    </div>
  );
});

// ============================================================================
// Main Component
// ============================================================================

export const ExecutionTimeline = memo(function ExecutionTimeline({
  projectId,
  onTaskClick,
  highlightedTaskId,
  defaultCollapsed = false,
  className,
  embedded = false,
}: ExecutionTimelineProps) {
  // Panel state (ignored when embedded)
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  // Filter state using new category-based system
  const [filterState, setFilterState] = useState<TimelineFilterState>({
    categories: [],
    isExpanded: false,
  });

  // Convert filter state to API filter format
  const apiFilters = useMemo((): TimelineFilters => {
    return toApiFilters(filterState.categories);
  }, [filterState.categories]);

  // Fetch timeline data
  const {
    data,
    isLoading,
    error,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isFetching,
    refresh,
  } = useExecutionTimeline(projectId, {
    pageSize: 25,
    filters: apiFilters,
    realTimeUpdates: true,
  });

  // Flatten paginated results and apply client-side category filtering
  const events = useMemo(() => {
    const allEvents = data?.pages.flatMap((page) => page.events) ?? [];
    // Apply client-side filtering by status category
    return filterTimelineEvents(allEvents, filterState.categories);
  }, [data?.pages, filterState.categories]);

  // Get total count from first page
  const totalCount = data?.pages[0]?.total ?? 0;

  // Handlers
  const handleToggleCollapse = useCallback(() => {
    setCollapsed((prev) => !prev);
  }, []);

  const handleLoadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  const handleTaskClick = useCallback(
    (taskId: string) => {
      onTaskClick?.(taskId);
    },
    [onTaskClick]
  );

  // Render collapsed state (not applicable in embedded mode)
  if (collapsed && !embedded) {
    return (
      <div
        className={cn(
          "flex flex-col h-full bg-[hsl(220_10%_10%_/_0.95)] backdrop-blur-sm",
          "border-l border-[hsl(220_10%_25%)]",
          className
        )}
        style={{ width: PANEL_WIDTH_COLLAPSED }}
        data-testid="execution-timeline-collapsed"
      >
        <TimelineHeader
          collapsed={true}
          onToggleCollapse={handleToggleCollapse}
          onRefresh={refresh}
          isRefreshing={isFetching}
          eventCount={totalCount}
        />
        {/* Rotated label for collapsed state */}
        <div className="flex-1 flex items-center justify-center">
          <span
            className="text-xs font-medium text-[hsl(220_10%_50%)] whitespace-nowrap"
            style={{ writingMode: "vertical-rl", transform: "rotate(180deg)" }}
          >
            Execution Timeline
          </span>
        </div>
      </div>
    );
  }

  // Render expanded state
  return (
    <div
      className={cn(
        "flex flex-col h-full",
        // Only apply backdrop styling when not embedded (FloatingTimeline handles it)
        !embedded && "bg-[hsl(220_10%_10%_/_0.95)] backdrop-blur-sm border-l border-[hsl(220_10%_25%)]",
        className
      )}
      style={embedded ? undefined : { width: PANEL_WIDTH_EXPANDED }}
      data-testid="execution-timeline"
    >
      {/* Header */}
      <TimelineHeader
        collapsed={false}
        onToggleCollapse={handleToggleCollapse}
        onRefresh={refresh}
        isRefreshing={isFetching && !isFetchingNextPage}
        eventCount={totalCount}
        hideCollapseToggle={embedded}
      />

      {/* Filter bar */}
      <TimelineFilterBar filters={filterState} onFilterChange={setFilterState} />

      {/* Content area */}
      <div className="flex-1 overflow-y-auto">
        {/* Loading state */}
        {isLoading && (
          <div className="flex items-center justify-center h-32">
            <Loader2
              className="w-5 h-5 animate-spin"
              style={{ color: "hsl(220 10% 45%)" }}
            />
          </div>
        )}

        {/* Error state */}
        {error && (
          <div className="flex flex-col items-center justify-center h-32 px-4 text-center">
            <AlertCircle
              className="w-5 h-5 mb-2"
              style={{ color: "hsl(0 70% 55%)" }}
            />
            <p
              style={{
                fontSize: "12px",
                color: "hsl(0 70% 60%)",
                marginBottom: "8px",
              }}
            >
              Failed to load timeline
            </p>
            <button
              onClick={refresh}
              style={{
                fontSize: "11px",
                color: "hsl(220 80% 65%)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.textDecoration = "underline";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.textDecoration = "none";
              }}
            >
              Try again
            </button>
          </div>
        )}

        {/* Empty state */}
        {!isLoading && !error && events.length === 0 && (
          <div className="flex flex-col items-center justify-center h-32 px-4 text-center">
            <Clock
              className="w-5 h-5 mb-2"
              style={{ color: "hsl(220 10% 35%)" }}
            />
            <p
              style={{
                fontSize: "12px",
                fontWeight: 500,
                color: "hsl(220 10% 50%)",
              }}
            >
              No events yet
            </p>
            <p
              style={{
                fontSize: "11px",
                color: "hsl(220 10% 40%)",
                marginTop: "4px",
              }}
            >
              Events appear as tasks progress
            </p>
          </div>
        )}

        {/* Timeline entries */}
        {!isLoading && !error && events.length > 0 && (
          <div className="py-2">
            {events.map((event) => (
              <TimelineEntry
                key={event.id}
                event={event}
                onTaskClick={handleTaskClick}
                isHighlighted={
                  event.taskId !== null && event.taskId === highlightedTaskId
                }
              />
            ))}

            {/* Load more */}
            <TimelineLoadMore
              hasMore={hasNextPage ?? false}
              isFetching={isFetchingNextPage}
              onLoadMore={handleLoadMore}
            />
          </div>
        )}
      </div>
    </div>
  );
});

export default ExecutionTimeline;
