/**
 * ActivityView - Real-time agent execution monitoring
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass header with backdrop-blur
 * - Flat translucent surfaces
 * - Ambient orange glow background
 * - Clean, minimal aesthetic
 */

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useInView } from "react-intersection-observer";
import { useActivityStore } from "@/stores/activityStore";
import { useUiStore } from "@/stores/uiStore";
import {
  useTaskActivityEvents,
  useSessionActivityEvents,
  useAllActivityEvents,
  flattenActivityPages,
} from "@/hooks/useActivityEvents";
import type { ActivityEventFilter, ActivityEventType } from "@/api/activity-events.types";
import { Activity, Trash2, ChevronDown, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";

// Local imports from extracted modules
import type {
  ViewMode,
  MessageTypeFilter,
  ExpandedState,
  CopiedState,
  UnifiedActivityMessage,
  RoleFilterValue,
} from "./ActivityView.types";
import {
  toUnifiedMessage,
  fromRealtimeMessage,
  generateMessageKey,
  getToolName,
} from "./ActivityView.utils";
import { ActivityMessage } from "./ActivityMessage";
import {
  ViewModeToggle,
  StatusFilter,
  RoleFilter,
  FilterTabs,
  SearchBar,
  EmptyState,
} from "./ActivityFilters";

// ============================================================================
// Main Component
// ============================================================================

export interface ActivityViewProps {
  /** Optional task ID to filter messages by */
  taskId?: string;
  /** Optional session ID to filter messages by */
  sessionId?: string;
  /** Whether to show the header with title */
  showHeader?: boolean;
  /** Force a specific view mode */
  initialMode?: ViewMode;
}

export function ActivityView({
  taskId,
  sessionId,
  showHeader = true,
  initialMode,
}: ActivityViewProps) {
  const realtimeMessages = useActivityStore((s) => s.messages);
  const alerts = useActivityStore((s) => s.alerts);
  const clearMessages = useActivityStore((s) => s.clearMessages);
  const clearActivityFilter = useUiStore((s) => s.clearActivityFilter);

  // Default to historical mode (shows all events) - only use realtime if explicitly requested
  const [viewMode, setViewMode] = useState<ViewMode>(initialMode ?? "historical");
  const [typeFilter, setTypeFilter] = useState<MessageTypeFilter>("all");
  const [statusFilter, setStatusFilter] = useState<string[]>([]);
  const [roleFilter, setRoleFilter] = useState<RoleFilterValue[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const [copied, setCopied] = useState<CopiedState>({});
  const [autoScroll, setAutoScroll] = useState(true);

  // Auto-switch to historical mode when navigating from StatusActivityBadge with context
  useEffect(() => {
    if (taskId || sessionId) {
      setViewMode("historical");
    }
  }, [taskId, sessionId]);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Infinite scroll sentinel
  const { ref: loadMoreRef, inView } = useInView({
    threshold: 0,
    rootMargin: "100px",
  });

  // Build filter for historical queries
  const historicalFilter: ActivityEventFilter | undefined = useMemo(() => {
    const filter: ActivityEventFilter = {};
    if (typeFilter !== "all") {
      filter.eventTypes = [typeFilter as ActivityEventType];
    }
    if (statusFilter.length > 0) {
      filter.statuses = statusFilter;
    }
    if (roleFilter.length > 0) {
      filter.roles = roleFilter;
    }
    return Object.keys(filter).length > 0 ? filter : undefined;
  }, [typeFilter, statusFilter, roleFilter]);

  // Historical queries - task-specific, session-specific, or global (all events)
  const taskHistoryQuery = useTaskActivityEvents({
    taskId: taskId ?? "",
    ...(historicalFilter !== undefined && { filter: historicalFilter }),
    limit: 50,
  });

  const sessionHistoryQuery = useSessionActivityEvents({
    sessionId: sessionId ?? "",
    ...(historicalFilter !== undefined && { filter: historicalFilter }),
    limit: 50,
  });

  // Global query for all events (when no task/session context)
  const globalHistoryQuery = useAllActivityEvents({
    ...(historicalFilter !== undefined && { filter: historicalFilter }),
    limit: 50,
  });

  // Select the appropriate historical query based on context
  // If taskId provided → task query; if sessionId → session query; otherwise → global query
  const historyQuery = taskId ? taskHistoryQuery : sessionId ? sessionHistoryQuery : globalHistoryQuery;
  const isHistoricalMode = viewMode === "historical";

  // Extract query data for stable dependencies
  const historyData = historyQuery?.data;
  const historyHasNextPage = historyQuery?.hasNextPage;
  const historyIsFetchingNextPage = historyQuery?.isFetchingNextPage;
  const historyFetchNextPage = historyQuery?.fetchNextPage;

  // Load more when sentinel is in view
  useEffect(() => {
    if (isHistoricalMode && inView && historyHasNextPage && !historyIsFetchingNextPage) {
      historyFetchNextPage?.();
    }
  }, [inView, isHistoricalMode, historyHasNextPage, historyIsFetchingNextPage, historyFetchNextPage]);

  // Convert messages to unified format based on mode
  const unifiedMessages = useMemo((): UnifiedActivityMessage[] => {
    if (isHistoricalMode && historyData) {
      const historicalEvents = flattenActivityPages(historyData);
      return historicalEvents.map(toUnifiedMessage);
    } else {
      return realtimeMessages.map((msg, index) => fromRealtimeMessage(msg, index));
    }
  }, [isHistoricalMode, historyData, realtimeMessages]);

  // Filter messages (for real-time mode - historical mode is filtered server-side)
  const filteredMessages = useMemo(() => {
    let filtered = unifiedMessages;

    if (!isHistoricalMode && taskId) {
      filtered = filtered.filter((m) => m.taskId === taskId);
    }

    if (!isHistoricalMode && typeFilter !== "all") {
      filtered = filtered.filter((m) => m.type === typeFilter);
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (m) =>
          m.content.toLowerCase().includes(query) ||
          m.type.toLowerCase().includes(query) ||
          (getToolName(m.content)?.toLowerCase().includes(query) ?? false)
      );
    }

    return filtered;
  }, [unifiedMessages, isHistoricalMode, taskId, typeFilter, searchQuery]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (autoScroll && messagesEndRef.current && typeof messagesEndRef.current.scrollIntoView === "function") {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [filteredMessages.length, autoScroll]);

  // Detect manual scrolling to disable auto-scroll
  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(isAtBottom);
  }, []);

  const toggleExpanded = useCallback((key: string) => {
    setExpanded((prev) => ({ ...prev, [key]: !prev[key] }));
  }, []);

  const handleCopy = useCallback((key: string) => {
    setCopied((prev) => ({ ...prev, [key]: true }));
    setTimeout(() => setCopied((prev) => ({ ...prev, [key]: false })), 2000);
  }, []);

  const handleClearSearch = useCallback(() => setSearchQuery(""), []);

  const handleViewModeChange = useCallback((mode: ViewMode) => {
    setViewMode(mode);
    if (mode === "realtime") clearActivityFilter();
  }, [clearActivityFilter]);

  const handleTypeFilterChange = useCallback((filter: MessageTypeFilter) => {
    setTypeFilter(filter);
    if (taskId || sessionId) clearActivityFilter();
  }, [taskId, sessionId, clearActivityFilter]);

  const handleStatusFilterChange = useCallback((statuses: string[]) => {
    setStatusFilter(statuses);
    if (taskId || sessionId) clearActivityFilter();
  }, [taskId, sessionId, clearActivityFilter]);

  const handleRoleFilterChange = useCallback((roles: RoleFilterValue[]) => {
    setRoleFilter(roles);
    if (taskId || sessionId) clearActivityFilter();
  }, [taskId, sessionId, clearActivityFilter]);

  const hasFilter = typeFilter !== "all" || searchQuery.trim() !== "";
  const isEmpty = filteredMessages.length === 0;
  const alertCount = alerts.filter((a) => a.severity === "high" || a.severity === "critical").length;

  return (
    <div
      data-testid="activity-view"
      className="flex flex-col h-full"
      style={{
        background: `
          radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.06) 0%, transparent 50%),
          radial-gradient(ellipse 60% 40% at 80% 100%, rgba(255,107,53,0.03) 0%, transparent 50%),
          var(--bg-base)
        `,
      }}
    >
      {/* Header - Frosted Glass */}
      {showHeader && (
        <div
          className="flex items-center justify-between px-4 py-3 border-b"
          style={{
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
            WebkitBackdropFilter: "blur(20px)",
            borderColor: "rgba(255,255,255,0.06)",
          }}
        >
          <div className="flex items-center gap-3">
            <div
              className="p-1.5 rounded-lg"
              style={{ background: "rgba(255,107,53,0.1)", border: "1px solid rgba(255,107,53,0.2)" }}
            >
              <Activity className="w-5 h-5 text-[var(--accent-primary)]" />
            </div>
            <h2 className="text-lg font-semibold tracking-tight text-[var(--text-primary)]">Activity</h2>
            {alertCount > 0 && (
              <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-[var(--status-error)] text-white">
                {alertCount} alert{alertCount > 1 ? "s" : ""}
              </span>
            )}
          </div>
          <Button
            data-testid="activity-clear"
            variant="ghost"
            size="sm"
            onClick={clearMessages}
            disabled={realtimeMessages.length === 0}
            className="text-[var(--text-muted)] hover:text-[var(--text-primary)] disabled:opacity-50"
          >
            <Trash2 className="w-4 h-4 mr-1.5" />
            Clear
          </Button>
        </div>
      )}

      {/* Search and Filters */}
      <div className="px-4 py-3 border-b space-y-3" style={{ borderColor: "rgba(255,255,255,0.06)" }}>
        <div className="flex items-center gap-3">
          <div className="flex-1">
            <SearchBar value={searchQuery} onChange={setSearchQuery} onClear={handleClearSearch} />
          </div>
          <ViewModeToggle mode={viewMode} onChange={handleViewModeChange} />
        </div>
        <div className="flex items-center gap-2">
          <FilterTabs active={typeFilter} onChange={handleTypeFilterChange} />
          {viewMode === "historical" && (
            <>
              <StatusFilter selectedStatuses={statusFilter} onChange={handleStatusFilterChange} />
              <RoleFilter selectedRoles={roleFilter} onChange={handleRoleFilterChange} />
            </>
          )}
        </div>
      </div>

      {/* Messages List */}
      <ScrollArea ref={containerRef} data-testid="activity-messages" className="flex-1" onScroll={handleScroll}>
        <div className="p-4 space-y-2">
          {isHistoricalMode && historyQuery?.isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-primary)]" />
              <span className="ml-2 text-sm text-[var(--text-muted)]">Loading activity history...</span>
            </div>
          )}

          {!isHistoricalMode && isEmpty && <EmptyState hasFilter={hasFilter} />}
          {isHistoricalMode && !historyQuery?.isLoading && isEmpty && <EmptyState hasFilter={hasFilter} />}

          {!isEmpty && (
            <>
              {filteredMessages.map((msg, index) => {
                const key = generateMessageKey(msg, index);
                return (
                  <ActivityMessage
                    key={key}
                    message={msg}
                    isExpanded={expanded[key] ?? false}
                    onToggle={() => toggleExpanded(key)}
                    copied={copied[key] ?? false}
                    onCopy={() => handleCopy(key)}
                  />
                );
              })}

              {isHistoricalMode && historyHasNextPage && (
                <div ref={loadMoreRef} className="py-4 flex items-center justify-center">
                  {historyIsFetchingNextPage ? (
                    <Loader2 className="w-5 h-5 animate-spin text-[var(--accent-primary)]" />
                  ) : (
                    <span className="text-xs text-[var(--text-muted)]">Scroll for more</span>
                  )}
                </div>
              )}

              {isHistoricalMode && !historyHasNextPage && filteredMessages.length > 0 && (
                <div className="py-4 text-center text-xs text-[var(--text-muted)]">
                  — End of activity history —
                </div>
              )}

              <div ref={messagesEndRef} />
            </>
          )}
        </div>
      </ScrollArea>

      {/* Scroll to Bottom Banner */}
      {!autoScroll && filteredMessages.length > 0 && (
        <div className="border-t border-[var(--border-subtle)] px-4 py-2">
          <Button
            data-testid="activity-scroll-to-bottom"
            variant="ghost"
            className="w-full text-sm text-[var(--accent-primary)] hover:bg-[var(--bg-hover)]"
            onClick={() => {
              setAutoScroll(true);
              messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
            }}
          >
            <ChevronDown className="w-4 h-4 mr-1.5" />
            Scroll to latest
          </Button>
        </div>
      )}

      {/* Thinking Animation Styles */}
      <style>{`
        @keyframes thinking-pulse {
          0%, 100% { opacity: 0.5; }
          50% { opacity: 1; }
        }
        .thinking-icon {
          animation: thinking-pulse 1.5s ease-in-out infinite;
        }
      `}</style>
    </div>
  );
}
