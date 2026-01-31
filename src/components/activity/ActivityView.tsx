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
import {
  useTaskActivityEvents,
  useSessionActivityEvents,
  flattenActivityPages,
} from "@/hooks/useActivityEvents";
import type { AgentMessageEvent } from "@/types/events";
import type { ActivityEventResponse, ActivityEventFilter, ActivityEventType } from "@/api/activity-events.types";
import {
  Activity,
  Brain,
  Terminal,
  CheckCircle,
  MessageSquare,
  AlertCircle,
  Search,
  X,
  Copy,
  Check,
  ChevronDown,
  Trash2,
  History,
  Radio,
  Loader2,
} from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

type MessageTypeFilter = "all" | "thinking" | "tool_call" | "tool_result" | "text" | "error";

/** View mode: real-time (Zustand) vs historical (database) */
type ViewMode = "realtime" | "historical";

interface ExpandedState {
  [key: string]: boolean;
}

interface CopiedState {
  [key: string]: boolean;
}

/**
 * Unified message type that can represent both real-time and historical events
 */
interface UnifiedActivityMessage {
  id: string;
  type: AgentMessageEvent["type"];
  content: string;
  timestamp: number;
  metadata?: Record<string, unknown> | undefined;
  taskId?: string | undefined;
  sessionId?: string | undefined;
  internalStatus?: string | null | undefined;
}

// ============================================================================
// Constants
// ============================================================================

const MESSAGE_TYPES: { key: MessageTypeFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "thinking", label: "Thinking" },
  { key: "tool_call", label: "Tool Calls" },
  { key: "tool_result", label: "Results" },
  { key: "text", label: "Text" },
  { key: "error", label: "Errors" },
];

// Status options for filtering (aligned with internal status values)
const STATUS_OPTIONS: { value: string; label: string }[] = [
  { value: "Ready", label: "Ready" },
  { value: "Queued", label: "Queued" },
  { value: "WorkerActive", label: "Worker Active" },
  { value: "WorkerDone", label: "Worker Done" },
  { value: "Reviewing", label: "Reviewing" },
  { value: "Approved", label: "Approved" },
  { value: "FixingRejection", label: "Fixing Rejection" },
  { value: "Escalated", label: "Escalated" },
  { value: "Done", label: "Done" },
];

// ============================================================================
// Utility Functions
// ============================================================================

function getMessageIcon(type: AgentMessageEvent["type"]) {
  switch (type) {
    case "thinking":
      return <Brain className="w-4 h-4 thinking-icon" />;
    case "tool_call":
      return <Terminal className="w-4 h-4" />;
    case "tool_result":
      return <CheckCircle className="w-4 h-4" />;
    case "text":
      return <MessageSquare className="w-4 h-4" />;
    case "error":
      return <AlertCircle className="w-4 h-4" />;
  }
}

function getMessageColor(type: AgentMessageEvent["type"]) {
  switch (type) {
    case "thinking":
      return "var(--text-muted)";
    case "tool_call":
      return "var(--accent-primary)";
    case "tool_result":
      return "var(--status-success)";
    case "text":
      return "var(--text-secondary)";
    case "error":
      return "var(--status-error)";
  }
}

function getMessageBgColor(type: AgentMessageEvent["type"]) {
  switch (type) {
    case "thinking":
      return "rgba(128, 128, 128, 0.08)";
    case "tool_call":
      return "rgba(255, 107, 53, 0.08)";
    case "tool_result":
      return "rgba(34, 197, 94, 0.08)";
    case "text":
      return "rgba(128, 128, 128, 0.04)";
    case "error":
      return "rgba(239, 68, 68, 0.1)";
  }
}

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function getToolName(content: string): string | null {
  // Try to extract tool name from content like "Using tool: Read" or "Read(..."
  const toolMatch = content.match(/^(?:Using tool:\s*)?(\w+)(?:\(|:)/);
  return toolMatch?.[1] ?? null;
}

function generateMessageKey(msg: UnifiedActivityMessage, index: number): string {
  return msg.id || `${msg.taskId || msg.sessionId}-${msg.timestamp}-${index}`;
}

/**
 * Convert a historical ActivityEventResponse to UnifiedActivityMessage
 */
function toUnifiedMessage(event: ActivityEventResponse): UnifiedActivityMessage {
  return {
    id: event.id,
    type: event.eventType as AgentMessageEvent["type"],
    content: event.content,
    timestamp: new Date(event.createdAt).getTime(),
    metadata: event.metadata ? JSON.parse(event.metadata) : undefined,
    taskId: event.taskId ?? undefined,
    sessionId: event.ideationSessionId ?? undefined,
    internalStatus: event.internalStatus,
  };
}

/**
 * Convert a real-time AgentMessageEvent to UnifiedActivityMessage
 */
function fromRealtimeMessage(msg: AgentMessageEvent, index: number): UnifiedActivityMessage {
  return {
    id: `realtime-${msg.taskId}-${msg.timestamp}-${index}`,
    type: msg.type,
    content: msg.content,
    timestamp: msg.timestamp,
    metadata: msg.metadata,
    taskId: msg.taskId,
  };
}

/**
 * Simple JSON syntax highlighter
 * Colorizes keys, strings, numbers, booleans, and null values
 */
function highlightJSON(json: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  let key = 0;

  // Match patterns: strings, numbers, booleans, null, keys, brackets/braces
  const regex = /("(?:[^"\\]|\\.)*")\s*:|("(?:[^"\\]|\\.)*")|(-?\d+\.?\d*(?:[eE][+-]?\d+)?)|(\btrue\b|\bfalse\b)|(\bnull\b)|([[\]{}:,])/g;
  let lastIndex = 0;
  let match;

  while ((match = regex.exec(json)) !== null) {
    // Add any text before the match
    if (match.index > lastIndex) {
      parts.push(<span key={key++}>{json.slice(lastIndex, match.index)}</span>);
    }

    if (match[1]) {
      // Key (with colon)
      parts.push(
        <span key={key++} style={{ color: "#f0f0f0" }}>
          {match[1]}
        </span>
      );
      parts.push(<span key={key++} style={{ color: "var(--text-muted)" }}>:</span>);
    } else if (match[2]) {
      // String value
      parts.push(
        <span key={key++} style={{ color: "#a5d6a7" }}>
          {match[2]}
        </span>
      );
    } else if (match[3]) {
      // Number
      parts.push(
        <span key={key++} style={{ color: "#ffcc80" }}>
          {match[3]}
        </span>
      );
    } else if (match[4]) {
      // Boolean
      parts.push(
        <span key={key++} style={{ color: "#81d4fa" }}>
          {match[4]}
        </span>
      );
    } else if (match[5]) {
      // Null
      parts.push(
        <span key={key++} style={{ color: "#ce93d8" }}>
          {match[5]}
        </span>
      );
    } else if (match[6]) {
      // Brackets, braces, colons, commas
      parts.push(
        <span key={key++} style={{ color: "var(--text-muted)" }}>
          {match[6]}
        </span>
      );
    }

    lastIndex = regex.lastIndex;
  }

  // Add any remaining text
  if (lastIndex < json.length) {
    parts.push(<span key={key++}>{json.slice(lastIndex)}</span>);
  }

  return parts;
}

// ============================================================================
// Sub-components
// ============================================================================

function ViewModeToggle({
  mode,
  onChange,
  disabled,
}: {
  mode: ViewMode;
  onChange: (mode: ViewMode) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex gap-1 p-1 rounded-lg bg-[var(--bg-base)]">
      <button
        data-testid="activity-mode-realtime"
        onClick={() => onChange("realtime")}
        disabled={disabled}
        className={cn(
          "flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors",
          mode === "realtime"
            ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-subtle)]"
            : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-transparent",
          disabled && "opacity-50 cursor-not-allowed"
        )}
      >
        <Radio className="w-3 h-3" />
        Live
      </button>
      <button
        data-testid="activity-mode-historical"
        onClick={() => onChange("historical")}
        disabled={disabled}
        className={cn(
          "flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors",
          mode === "historical"
            ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-subtle)]"
            : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-transparent",
          disabled && "opacity-50 cursor-not-allowed"
        )}
      >
        <History className="w-3 h-3" />
        History
      </button>
    </div>
  );
}

function StatusFilter({
  selectedStatuses,
  onChange,
}: {
  selectedStatuses: string[];
  onChange: (statuses: string[]) => void;
}) {
  const handleToggle = useCallback((status: string) => {
    if (selectedStatuses.includes(status)) {
      onChange(selectedStatuses.filter((s) => s !== status));
    } else {
      onChange([...selectedStatuses, status]);
    }
  }, [selectedStatuses, onChange]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="h-8 text-xs gap-1.5 bg-[var(--bg-elevated)] border-[var(--border-default)] hover:bg-[var(--bg-hover)]"
        >
          Status
          {selectedStatuses.length > 0 && (
            <span className="px-1.5 py-0.5 rounded-full bg-[var(--accent-primary)] text-white text-[10px]">
              {selectedStatuses.length}
            </span>
          )}
          <ChevronDown className="w-3 h-3 ml-1" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-40">
        {STATUS_OPTIONS.map(({ value, label }) => (
          <DropdownMenuCheckboxItem
            key={value}
            checked={selectedStatuses.includes(value)}
            onCheckedChange={() => handleToggle(value)}
          >
            {label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function FilterTabs({
  active,
  onChange,
}: {
  active: MessageTypeFilter;
  onChange: (filter: MessageTypeFilter) => void;
}) {
  return (
    <div className="flex gap-1 p-1 rounded-lg bg-[var(--bg-base)] overflow-x-auto">
      {MESSAGE_TYPES.map(({ key, label }) => {
        const isActive = active === key;
        return (
          <button
            key={key}
            role="tab"
            data-active={isActive ? "true" : "false"}
            onClick={() => onChange(key)}
            className={cn(
              "px-3 py-1.5 text-xs font-medium rounded-md transition-colors whitespace-nowrap",
              isActive
                ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-subtle)]"
                : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-transparent"
            )}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

function SearchBar({
  value,
  onChange,
  onClear,
}: {
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
}) {
  return (
    <div className="relative">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-muted)]" />
      <Input
        type="text"
        data-testid="activity-search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search activities..."
        className="pl-10 pr-8 h-9 bg-[var(--bg-elevated)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)]/30"
      />
      {value && (
        <button
          onClick={onClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-white/5 text-[var(--text-muted)]"
          aria-label="Clear search"
        >
          <X className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}

function EmptyState({ hasFilter }: { hasFilter: boolean }) {
  return (
    <div
      data-testid="activity-empty"
      className="flex flex-col items-center justify-center h-full p-8 text-center"
    >
      <div className="mb-4 opacity-50">
        <Activity className="w-12 h-12 text-[var(--text-muted)]" strokeDasharray="4 4" />
      </div>
      <p className="text-[var(--text-secondary)]">
        {hasFilter ? "No matching activities" : "No activity yet"}
      </p>
      <p className="text-sm text-[var(--text-muted)] mt-1">
        {hasFilter
          ? "Try adjusting your search or filters"
          : "Agent activity will appear here when tasks are running"}
      </p>
    </div>
  );
}

interface ActivityMessageProps {
  message: UnifiedActivityMessage;
  isExpanded: boolean;
  onToggle: () => void;
  copied: boolean;
  onCopy: () => void;
}

function ActivityMessage({
  message,
  isExpanded,
  onToggle,
  copied,
  onCopy,
}: ActivityMessageProps) {
  const { type, content, timestamp, metadata, internalStatus } = message;
  const hasDetails = type === "tool_call" || type === "tool_result" || metadata;
  const toolName = getToolName(content);

  // Parse content for display
  const displayContent = content.length > 200 && !isExpanded
    ? content.slice(0, 200) + "..."
    : content;

  const handleCopy = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (metadata) {
        navigator.clipboard.writeText(JSON.stringify(metadata, null, 2));
        onCopy();
      }
    },
    [metadata, onCopy]
  );

  return (
    <div
      data-testid="activity-message"
      data-type={type}
      className="rounded-lg transition-all"
      style={{
        backgroundColor: getMessageBgColor(type),
        borderLeft: `3px solid ${getMessageColor(type)}`,
      }}
    >
      {/* Header */}
      <div
        className={cn(
          "flex items-start gap-3 px-3 py-2.5 select-none",
          hasDetails && "cursor-pointer hover:bg-white/[0.02]"
        )}
        onClick={hasDetails ? onToggle : undefined}
      >
        {/* Expand/Collapse Icon */}
        {hasDetails && (
          <ChevronDown
            className={cn(
              "w-3 h-3 mt-1 text-[var(--text-muted)] transition-transform shrink-0",
              !isExpanded && "-rotate-90"
            )}
          />
        )}
        {!hasDetails && <span className="w-3 shrink-0" />}

        {/* Type Icon */}
        <span className="mt-0.5 shrink-0" style={{ color: getMessageColor(type) }}>
          {getMessageIcon(type)}
        </span>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            {toolName && (
              <span
                className="text-xs font-mono px-1.5 py-0.5 rounded bg-[var(--bg-base)]"
                style={{ color: getMessageColor(type) }}
              >
                {toolName}
              </span>
            )}
            <span className="text-xs text-[var(--text-muted)] capitalize">
              {type.replace("_", " ")}
            </span>
            {internalStatus && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--bg-base)] text-[var(--text-muted)]">
                {internalStatus}
              </span>
            )}
          </div>
          <p className="text-sm text-[var(--text-primary)] whitespace-pre-wrap break-words">
            {displayContent}
          </p>
        </div>

        {/* Timestamp */}
        <span className="text-xs text-[var(--text-muted)] shrink-0 ml-2">
          {formatTimestamp(timestamp)}
        </span>
      </div>

      {/* Expanded Details */}
      {hasDetails && isExpanded && metadata && (
        <div className="ml-9 mr-3 pb-3 border-t border-[var(--border-subtle)]">
          <div className="pt-3 relative">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-[var(--text-muted)]">Details</span>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 hover:bg-[var(--bg-hover)]"
                onClick={handleCopy}
              >
                {copied ? (
                  <Check className="w-3.5 h-3.5 text-[var(--status-success)]" />
                ) : (
                  <Copy className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                )}
              </Button>
            </div>
            <pre className="text-xs font-mono p-3 rounded-md bg-[var(--bg-base)] text-[var(--text-secondary)] overflow-x-auto max-h-[300px] overflow-y-auto">
              {highlightJSON(JSON.stringify(metadata, null, 2))}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}

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

  // Determine initial mode: if we have a context (taskId/sessionId), default to historical
  const defaultMode: ViewMode = taskId || sessionId ? "historical" : "realtime";
  const [viewMode, setViewMode] = useState<ViewMode>(initialMode ?? defaultMode);
  const [typeFilter, setTypeFilter] = useState<MessageTypeFilter>("all");
  const [statusFilter, setStatusFilter] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const [copied, setCopied] = useState<CopiedState>({});
  const [autoScroll, setAutoScroll] = useState(true);

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
    // Only return filter if it has properties
    return Object.keys(filter).length > 0 ? filter : undefined;
  }, [typeFilter, statusFilter]);

  // Historical queries (enabled only in historical mode with appropriate context)
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

  // Select the appropriate historical query based on context
  const historyQuery = taskId ? taskHistoryQuery : sessionId ? sessionHistoryQuery : null;
  const isHistoricalMode = viewMode === "historical" && historyQuery;

  // Extract query data for stable dependencies
  const historyData = historyQuery?.data;
  const historyHasNextPage = historyQuery?.hasNextPage;
  const historyIsFetchingNextPage = historyQuery?.isFetchingNextPage;
  const historyFetchNextPage = historyQuery?.fetchNextPage;

  // Load more when sentinel is in view
  useEffect(() => {
    if (
      isHistoricalMode &&
      inView &&
      historyHasNextPage &&
      !historyIsFetchingNextPage
    ) {
      historyFetchNextPage?.();
    }
  }, [inView, isHistoricalMode, historyHasNextPage, historyIsFetchingNextPage, historyFetchNextPage]);

  // Convert messages to unified format based on mode
  const unifiedMessages = useMemo((): UnifiedActivityMessage[] => {
    if (isHistoricalMode && historyData) {
      const historicalEvents = flattenActivityPages(historyData);
      return historicalEvents.map(toUnifiedMessage);
    } else {
      // Real-time mode
      return realtimeMessages.map((msg, index) => fromRealtimeMessage(msg, index));
    }
  }, [isHistoricalMode, historyData, realtimeMessages]);

  // Filter messages (for real-time mode - historical mode is filtered server-side)
  const filteredMessages = useMemo(() => {
    let filtered = unifiedMessages;

    // Filter by task ID if provided (real-time mode only)
    if (!isHistoricalMode && taskId) {
      filtered = filtered.filter((m) => m.taskId === taskId);
    }

    // Filter by type (real-time mode only - historical is filtered server-side)
    if (!isHistoricalMode && typeFilter !== "all") {
      filtered = filtered.filter((m) => m.type === typeFilter);
    }

    // Filter by search query (both modes)
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

  // Toggle message expansion
  const toggleExpanded = useCallback((key: string) => {
    setExpanded((prev) => ({
      ...prev,
      [key]: !prev[key],
    }));
  }, []);

  // Handle copy with visual feedback
  const handleCopy = useCallback((key: string) => {
    setCopied((prev) => ({ ...prev, [key]: true }));
    setTimeout(() => {
      setCopied((prev) => ({ ...prev, [key]: false }));
    }, 2000);
  }, []);

  // Clear search
  const handleClearSearch = useCallback(() => {
    setSearchQuery("");
  }, []);

  // Check if there are active filters
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
              style={{
                background: "rgba(255,107,53,0.1)",
                border: "1px solid rgba(255,107,53,0.2)",
              }}
            >
              <Activity className="w-5 h-5 text-[var(--accent-primary)]" />
            </div>
            <h2 className="text-lg font-semibold tracking-tight text-[var(--text-primary)]">
              Activity
            </h2>
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
      <div
        className="px-4 py-3 border-b space-y-3"
        style={{ borderColor: "rgba(255,255,255,0.06)" }}
      >
        <div className="flex items-center gap-3">
          <div className="flex-1">
            <SearchBar
              value={searchQuery}
              onChange={setSearchQuery}
              onClear={handleClearSearch}
            />
          </div>
          <ViewModeToggle
            mode={viewMode}
            onChange={setViewMode}
            disabled={!taskId && !sessionId}
          />
        </div>
        <div className="flex items-center gap-2">
          <FilterTabs active={typeFilter} onChange={setTypeFilter} />
          {viewMode === "historical" && (
            <StatusFilter
              selectedStatuses={statusFilter}
              onChange={setStatusFilter}
            />
          )}
        </div>
      </div>

      {/* Messages List */}
      <ScrollArea
        ref={containerRef}
        data-testid="activity-messages"
        className="flex-1"
        onScroll={handleScroll}
      >
        <div className="p-4 space-y-2">
          {/* Loading state for historical mode */}
          {isHistoricalMode && historyQuery?.isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-primary)]" />
              <span className="ml-2 text-sm text-[var(--text-muted)]">Loading activity history...</span>
            </div>
          )}

          {/* Empty state */}
          {!isHistoricalMode && isEmpty && <EmptyState hasFilter={hasFilter} />}
          {isHistoricalMode && !historyQuery?.isLoading && isEmpty && <EmptyState hasFilter={hasFilter} />}

          {/* Messages */}
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

              {/* Infinite scroll sentinel for historical mode */}
              {isHistoricalMode && historyHasNextPage && (
                <div ref={loadMoreRef} className="py-4 flex items-center justify-center">
                  {historyIsFetchingNextPage ? (
                    <Loader2 className="w-5 h-5 animate-spin text-[var(--accent-primary)]" />
                  ) : (
                    <span className="text-xs text-[var(--text-muted)]">Scroll for more</span>
                  )}
                </div>
              )}

              {/* End of history indicator */}
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
