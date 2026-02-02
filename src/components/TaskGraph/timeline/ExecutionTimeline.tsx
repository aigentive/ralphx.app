/**
 * ExecutionTimeline - Collapsible side panel showing chronological task execution history
 *
 * Features:
 * - Chronological list of task events (status changes, plan events)
 * - Click entry to highlight corresponding node in graph
 * - Filter by event type
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
} from "lucide-react";
import { cn } from "@/lib/utils";
import { TimelineEntry } from "./TimelineEntry";
import { useExecutionTimeline, type TimelineFilters } from "../hooks/useExecutionTimeline";

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
  /** Default collapsed state */
  defaultCollapsed?: boolean;
  /** Additional className for the container */
  className?: string;
}

export interface TimelineFilterState {
  showStatusChanges: boolean;
  showPlanEvents: boolean;
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
  return (
    <div className="flex items-center gap-2 px-3 py-2 border-b border-[hsl(220_10%_25%)]">
      <button
        onClick={() =>
          onFilterChange({
            ...filters,
            showStatusChanges: !filters.showStatusChanges,
          })
        }
        className={cn(
          "px-2 py-1 text-[10px] font-medium rounded transition-colors",
          filters.showStatusChanges
            ? "bg-[hsl(var(--accent-primary))] text-white"
            : "bg-[hsl(220_10%_20%)] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_80%)]"
        )}
        aria-pressed={filters.showStatusChanges}
      >
        Status Changes
      </button>
      <button
        onClick={() =>
          onFilterChange({
            ...filters,
            showPlanEvents: !filters.showPlanEvents,
          })
        }
        className={cn(
          "px-2 py-1 text-[10px] font-medium rounded transition-colors",
          filters.showPlanEvents
            ? "bg-[hsl(var(--accent-primary))] text-white"
            : "bg-[hsl(220_10%_20%)] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_80%)]"
        )}
        aria-pressed={filters.showPlanEvents}
      >
        Plan Events
      </button>
    </div>
  );
});

interface TimelineHeaderProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  onRefresh: () => void;
  isRefreshing: boolean;
  eventCount: number;
}

const TimelineHeader = memo(function TimelineHeader({
  collapsed,
  onToggleCollapse,
  onRefresh,
  isRefreshing,
  eventCount,
}: TimelineHeaderProps) {
  return (
    <div className="flex items-center justify-between px-3 py-2 border-b border-[hsl(220_10%_25%)]">
      <div className="flex items-center gap-2">
        <Clock className="w-4 h-4 text-[hsl(220_10%_60%)]" />
        {!collapsed && (
          <>
            <span className="text-sm font-medium text-[hsl(220_10%_90%)]">
              Timeline
            </span>
            <span className="text-xs text-[hsl(220_10%_50%)]">
              ({eventCount})
            </span>
          </>
        )}
      </div>
      {!collapsed && (
        <div className="flex items-center gap-1">
          <button
            onClick={onRefresh}
            disabled={isRefreshing}
            className="p-1 rounded hover:bg-[hsl(220_10%_20%)] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_90%)] transition-colors disabled:opacity-50"
            title="Refresh timeline"
          >
            <RefreshCw
              className={cn("w-3.5 h-3.5", isRefreshing && "animate-spin")}
            />
          </button>
          <button
            onClick={onToggleCollapse}
            className="p-1 rounded hover:bg-[hsl(220_10%_20%)] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_90%)] transition-colors"
            title="Collapse timeline"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>
      )}
      {collapsed && (
        <button
          onClick={onToggleCollapse}
          className="p-1 rounded hover:bg-[hsl(220_10%_20%)] text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_90%)] transition-colors"
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
        className="w-full py-2 text-xs font-medium text-[hsl(220_10%_60%)] hover:text-[hsl(220_10%_90%)] hover:bg-[hsl(220_10%_20%)] rounded transition-colors disabled:opacity-50"
      >
        {isFetching ? (
          <span className="flex items-center justify-center gap-2">
            <Loader2 className="w-3 h-3 animate-spin" />
            Loading...
          </span>
        ) : (
          "Load more events"
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
}: ExecutionTimelineProps) {
  // Panel state
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  // Filter state
  const [filterState, setFilterState] = useState<TimelineFilterState>({
    showStatusChanges: true,
    showPlanEvents: true,
  });

  // Convert filter state to API filter format
  const apiFilters = useMemo((): TimelineFilters => {
    const eventTypes: TimelineFilters["eventTypes"] = [];
    if (filterState.showStatusChanges) {
      eventTypes.push("status_change");
    }
    if (filterState.showPlanEvents) {
      eventTypes.push("plan_accepted", "plan_completed");
    }
    // If both are off, show nothing (return empty array)
    // If both are on, return null to get all events
    if (eventTypes.length === 3) {
      return {};
    }
    return { eventTypes: eventTypes.length > 0 ? eventTypes : ["status_change"] };
  }, [filterState]);

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

  // Flatten paginated results
  const events = useMemo(
    () => data?.pages.flatMap((page) => page.events) ?? [],
    [data?.pages]
  );

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

  // Render collapsed state
  if (collapsed) {
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
        "flex flex-col h-full bg-[hsl(220_10%_10%_/_0.95)] backdrop-blur-sm",
        "border-l border-[hsl(220_10%_25%)]",
        className
      )}
      style={{ width: PANEL_WIDTH_EXPANDED }}
      data-testid="execution-timeline"
    >
      {/* Header */}
      <TimelineHeader
        collapsed={false}
        onToggleCollapse={handleToggleCollapse}
        onRefresh={refresh}
        isRefreshing={isFetching && !isFetchingNextPage}
        eventCount={totalCount}
      />

      {/* Filter bar */}
      <TimelineFilterBar filters={filterState} onFilterChange={setFilterState} />

      {/* Content area */}
      <div className="flex-1 overflow-y-auto">
        {/* Loading state */}
        {isLoading && (
          <div className="flex items-center justify-center h-32">
            <Loader2 className="w-6 h-6 animate-spin text-[hsl(220_10%_50%)]" />
          </div>
        )}

        {/* Error state */}
        {error && (
          <div className="flex flex-col items-center justify-center h-32 px-4 text-center">
            <AlertCircle className="w-6 h-6 text-destructive mb-2" />
            <p className="text-xs text-destructive mb-2">
              Failed to load timeline
            </p>
            <button
              onClick={refresh}
              className="text-xs text-[hsl(var(--accent-primary))] hover:underline"
            >
              Try again
            </button>
          </div>
        )}

        {/* Empty state */}
        {!isLoading && !error && events.length === 0 && (
          <div className="flex flex-col items-center justify-center h-32 px-4 text-center">
            <Clock className="w-6 h-6 text-[hsl(220_10%_40%)] mb-2" />
            <p className="text-xs text-[hsl(220_10%_50%)]">
              No timeline events yet
            </p>
            <p className="text-[10px] text-[hsl(220_10%_40%)] mt-1">
              Events will appear here as tasks progress
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
