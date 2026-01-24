/**
 * ActivityView - Real-time agent execution monitoring
 *
 * Features:
 * - Agent thinking and actions display
 * - Expandable tool call details (inputs/outputs)
 * - Scrollable history with auto-scroll to new messages
 * - Search/filter by tool name or action type
 * - Similar to Claude Desktop execution panel
 */

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useActivityStore } from "@/stores/activityStore";
import type { AgentMessageEvent } from "@/types/events";

// ============================================================================
// Types
// ============================================================================

type MessageTypeFilter = "all" | "thinking" | "tool_call" | "tool_result" | "text" | "error";

interface ExpandedState {
  [key: string]: boolean;
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

// ============================================================================
// Icons
// ============================================================================

function SearchIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <circle cx="7" cy="7" r="4.5" stroke="currentColor" strokeWidth="1.5" />
      <path d="M10.5 10.5L14 14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function ChevronDownIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

// ChevronRightIcon available for future use if needed
// function ChevronRightIcon() {
//   return (
//     <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
//       <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
//     </svg>
//   );
// }

function ThinkingIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.5" strokeDasharray="3 3" />
      <circle cx="5" cy="7" r="1" fill="currentColor" />
      <circle cx="9" cy="7" r="1" fill="currentColor" />
    </svg>
  );
}

function ToolIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M8.5 5.5L12.5 1.5M12.5 1.5L11 1L12.5 1.5L13 3M12.5 1.5L11 3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M7 7L1.5 12.5L1.5 12.5C1.22 12.78 1.22 13.22 1.5 13.5C1.78 13.78 2.22 13.78 2.5 13.5L8 8" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <circle cx="7.5" cy="6.5" r="2.5" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function ResultIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="1.5" y="2.5" width="11" height="9" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M4 6h6M4 8.5h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function TextIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M2 3h10M2 7h8M2 11h6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function ErrorIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.5" />
      <path d="M7 4v3M7 9v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function ClearIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M12 4L4 12M4 4l8 8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

// ============================================================================
// Utility Functions
// ============================================================================

function getMessageIcon(type: AgentMessageEvent["type"]) {
  switch (type) {
    case "thinking":
      return <ThinkingIcon />;
    case "tool_call":
      return <ToolIcon />;
    case "tool_result":
      return <ResultIcon />;
    case "text":
      return <TextIcon />;
    case "error":
      return <ErrorIcon />;
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
      return "rgba(128, 128, 128, 0.1)";
    case "tool_call":
      return "rgba(255, 107, 53, 0.1)";
    case "tool_result":
      return "rgba(34, 197, 94, 0.1)";
    case "text":
      return "rgba(128, 128, 128, 0.05)";
    case "error":
      return "rgba(239, 68, 68, 0.1)";
  }
}

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function getToolName(content: string): string | null {
  // Try to extract tool name from content like "Using tool: Read" or "Read(..."
  const toolMatch = content.match(/^(?:Using tool:\s*)?(\w+)(?:\(|:)/);
  return toolMatch?.[1] ?? null;
}

function generateMessageKey(msg: AgentMessageEvent, index: number): string {
  return `${msg.taskId}-${msg.timestamp}-${index}`;
}

// ============================================================================
// Sub-components
// ============================================================================

function FilterTabs({
  active,
  onChange,
}: {
  active: MessageTypeFilter;
  onChange: (filter: MessageTypeFilter) => void;
}) {
  return (
    <div className="flex gap-1 p-1 rounded-lg overflow-x-auto" style={{ backgroundColor: "var(--bg-base)" }}>
      {MESSAGE_TYPES.map(({ key, label }) => {
        const isActive = active === key;
        return (
          <button
            key={key}
            role="tab"
            data-active={isActive ? "true" : "false"}
            onClick={() => onChange(key)}
            className="px-3 py-1.5 text-xs font-medium rounded-md transition-colors whitespace-nowrap border"
            style={{
              backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
              color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
              borderColor: isActive ? "var(--border-subtle)" : "transparent",
            }}
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
    <div className="relative flex-1">
      <span
        className="absolute left-3 top-1/2 -translate-y-1/2"
        style={{ color: "var(--text-muted)" }}
      >
        <SearchIcon />
      </span>
      <input
        type="text"
        data-testid="activity-search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search activities..."
        className="w-full pl-10 pr-8 py-2 text-sm rounded-lg outline-none"
        style={{
          backgroundColor: "var(--bg-elevated)",
          color: "var(--text-primary)",
          border: "1px solid var(--border-subtle)",
        }}
      />
      {value && (
        <button
          onClick={onClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-white/5"
          style={{ color: "var(--text-muted)" }}
          aria-label="Clear search"
        >
          <ClearIcon />
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
      <svg
        width="48"
        height="48"
        viewBox="0 0 48 48"
        fill="none"
        className="mb-4"
        style={{ color: "var(--text-muted)" }}
      >
        <circle
          cx="24"
          cy="24"
          r="20"
          stroke="currentColor"
          strokeWidth="2"
          strokeDasharray="4 4"
        />
        <path
          d="M14 24H34M14 18H30M14 30H26"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>
      <p style={{ color: "var(--text-secondary)" }}>
        {hasFilter ? "No matching activities" : "No activity yet"}
      </p>
      <p className="text-sm mt-1" style={{ color: "var(--text-muted)" }}>
        {hasFilter
          ? "Try adjusting your search or filters"
          : "Agent activity will appear here when tasks are running"}
      </p>
    </div>
  );
}

interface ActivityMessageProps {
  message: AgentMessageEvent;
  isExpanded: boolean;
  onToggle: () => void;
}

function ActivityMessage({ message, isExpanded, onToggle }: ActivityMessageProps) {
  const { type, content, timestamp, metadata } = message;
  const hasDetails = type === "tool_call" || type === "tool_result" || metadata;
  const toolName = getToolName(content);

  // Parse content for display
  const displayContent = content.length > 200 && !isExpanded
    ? content.slice(0, 200) + "..."
    : content;

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
        className="flex items-start gap-3 px-3 py-2 cursor-pointer select-none"
        onClick={hasDetails ? onToggle : undefined}
      >
        {/* Expand/Collapse Icon */}
        {hasDetails && (
          <span
            className="mt-0.5 transition-transform"
            style={{
              color: "var(--text-muted)",
              transform: isExpanded ? "rotate(0deg)" : "rotate(-90deg)",
            }}
          >
            <ChevronDownIcon />
          </span>
        )}
        {!hasDetails && <span className="w-3" />}

        {/* Type Icon */}
        <span className="mt-0.5" style={{ color: getMessageColor(type) }}>
          {getMessageIcon(type)}
        </span>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            {toolName && (
              <span
                className="text-xs font-mono px-1.5 py-0.5 rounded"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: getMessageColor(type),
                }}
              >
                {toolName}
              </span>
            )}
            <span
              className="text-xs capitalize"
              style={{ color: "var(--text-muted)" }}
            >
              {type.replace("_", " ")}
            </span>
          </div>
          <p
            className="text-sm whitespace-pre-wrap break-words"
            style={{ color: "var(--text-primary)" }}
          >
            {displayContent}
          </p>
        </div>

        {/* Timestamp */}
        <span
          className="text-xs shrink-0 ml-2"
          style={{ color: "var(--text-muted)" }}
        >
          {formatTimestamp(timestamp)}
        </span>
      </div>

      {/* Expanded Details */}
      {hasDetails && isExpanded && metadata && (
        <div
          className="px-3 pb-3 ml-9 mr-3"
          style={{ borderTop: "1px solid var(--border-subtle)" }}
        >
          <div className="pt-2">
            <p
              className="text-xs font-medium mb-1"
              style={{ color: "var(--text-muted)" }}
            >
              Details
            </p>
            <pre
              className="text-xs p-2 rounded overflow-x-auto"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-secondary)",
              }}
            >
              {JSON.stringify(metadata, null, 2)}
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
  /** Whether to show the header with title */
  showHeader?: boolean;
}

export function ActivityView({ taskId, showHeader = true }: ActivityViewProps) {
  const messages = useActivityStore((s) => s.messages);
  const alerts = useActivityStore((s) => s.alerts);
  const clearMessages = useActivityStore((s) => s.clearMessages);

  const [typeFilter, setTypeFilter] = useState<MessageTypeFilter>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const [autoScroll, setAutoScroll] = useState(true);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Filter messages
  const filteredMessages = useMemo(() => {
    let filtered = messages;

    // Filter by task ID if provided
    if (taskId) {
      filtered = filtered.filter((m) => m.taskId === taskId);
    }

    // Filter by type
    if (typeFilter !== "all") {
      filtered = filtered.filter((m) => m.type === typeFilter);
    }

    // Filter by search query
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
  }, [messages, taskId, typeFilter, searchQuery]);

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
      style={{ backgroundColor: "var(--bg-surface)" }}
    >
      {/* Header */}
      {showHeader && (
        <div
          className="flex items-center justify-between px-4 py-3 border-b"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <div className="flex items-center gap-3">
            <h2
              className="text-lg font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              Activity
            </h2>
            {alertCount > 0 && (
              <span
                className="px-2 py-0.5 text-xs font-medium rounded-full"
                style={{
                  backgroundColor: "var(--status-error)",
                  color: "white",
                }}
              >
                {alertCount} alert{alertCount > 1 ? "s" : ""}
              </span>
            )}
          </div>
          <button
            data-testid="activity-clear"
            onClick={clearMessages}
            className="text-sm px-3 py-1.5 rounded-md transition-colors"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-secondary)",
            }}
            disabled={messages.length === 0}
          >
            Clear
          </button>
        </div>
      )}

      {/* Search and Filters */}
      <div className="px-4 py-3 border-b space-y-3" style={{ borderColor: "var(--border-subtle)" }}>
        <SearchBar
          value={searchQuery}
          onChange={setSearchQuery}
          onClear={handleClearSearch}
        />
        <FilterTabs active={typeFilter} onChange={setTypeFilter} />
      </div>

      {/* Messages List */}
      <div
        ref={containerRef}
        data-testid="activity-messages"
        className="flex-1 overflow-y-auto p-4 space-y-2"
        onScroll={handleScroll}
      >
        {isEmpty ? (
          <EmptyState hasFilter={hasFilter} />
        ) : (
          <>
            {filteredMessages.map((msg, index) => {
              const key = generateMessageKey(msg, index);
              return (
                <ActivityMessage
                  key={key}
                  message={msg}
                  isExpanded={expanded[key] ?? false}
                  onToggle={() => toggleExpanded(key)}
                />
              );
            })}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Auto-scroll indicator */}
      {!autoScroll && filteredMessages.length > 0 && (
        <div className="px-4 py-2 border-t" style={{ borderColor: "var(--border-subtle)" }}>
          <button
            data-testid="activity-scroll-to-bottom"
            onClick={() => {
              setAutoScroll(true);
              messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
            }}
            className="w-full text-sm py-2 rounded-md transition-colors"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--accent-primary)",
            }}
          >
            Scroll to latest
          </button>
        </div>
      )}
    </div>
  );
}
