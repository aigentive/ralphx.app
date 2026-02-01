/**
 * Utility functions for ActivityView components
 */

import type { AgentMessageEvent } from "@/types/events";
import type { ActivityEventResponse } from "@/api/activity-events.types";
import type { UnifiedActivityMessage } from "./ActivityView.types";
import {
  Brain,
  Terminal,
  CheckCircle,
  MessageSquare,
  AlertCircle,
} from "lucide-react";
import { createElement } from "react";

// ============================================================================
// Icon & Color Utilities
// ============================================================================

export function getMessageIcon(type: AgentMessageEvent["type"]) {
  switch (type) {
    case "thinking":
      return createElement(Brain, { className: "w-4 h-4 thinking-icon" });
    case "tool_call":
      return createElement(Terminal, { className: "w-4 h-4" });
    case "tool_result":
      return createElement(CheckCircle, { className: "w-4 h-4" });
    case "text":
      return createElement(MessageSquare, { className: "w-4 h-4" });
    case "error":
      return createElement(AlertCircle, { className: "w-4 h-4" });
  }
}

export function getMessageColor(type: AgentMessageEvent["type"]) {
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

export function getMessageBgColor(type: AgentMessageEvent["type"]) {
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

// ============================================================================
// Formatting Utilities
// ============================================================================

export function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function getToolName(content: string): string | null {
  // Try to extract tool name from content like "Using tool: Read" or "Read(..."
  const toolMatch = content.match(/^(?:Using tool:\s*)?(\w+)(?:\(|:)/);
  return toolMatch?.[1] ?? null;
}

/**
 * Strip MCP server prefixes from tool names for cleaner display.
 * Examples:
 *   - "mcp__ralphx__get_task_steps" -> "get_task_steps"
 *   - "mcp__plugin_context7_context7__resolve-library-id" -> "resolve-library-id"
 *   - "Read" -> "Read"
 */
export function cleanToolName(rawName: string): string {
  // Match mcp__<server>__<toolName> pattern
  const mcpMatch = rawName.match(/^mcp__[^_]+(?:_[^_]+)*__(.+)$/);
  if (mcpMatch && mcpMatch[1]) {
    return mcpMatch[1];
  }
  return rawName;
}

/**
 * Format tool arguments for display as key-value pairs.
 * Returns an array of { key, value } for rendering.
 */
export function formatToolArguments(
  metadata: Record<string, unknown> | undefined
): Array<{ key: string; value: string }> {
  if (!metadata || typeof metadata !== "object") {
    return [];
  }

  return Object.entries(metadata).map(([key, value]) => {
    let displayValue: string;
    if (typeof value === "string") {
      // Truncate long strings
      displayValue = value.length > 80 ? value.slice(0, 80) + "…" : value;
    } else if (value === null || value === undefined) {
      displayValue = "null";
    } else if (typeof value === "object") {
      // For objects/arrays, show a compact preview
      const json = JSON.stringify(value);
      displayValue = json.length > 60 ? json.slice(0, 60) + "…" : json;
    } else {
      displayValue = String(value);
    }
    return { key, value: displayValue };
  });
}

export function generateMessageKey(msg: UnifiedActivityMessage, index: number): string {
  return msg.id || `${msg.taskId || msg.sessionId}-${msg.timestamp}-${index}`;
}

// ============================================================================
// Safe JSON Parsing
// ============================================================================

export interface SafeJsonParseResult<T = unknown> {
  data: T;
  error: boolean;
}

/**
 * Safely parse JSON without throwing errors.
 * Returns the parsed data and an error flag.
 * On parse failure, returns the original string as data.
 */
export function safeJsonParse<T = unknown>(str: string): SafeJsonParseResult<T | string> {
  try {
    return { data: JSON.parse(str) as T, error: false };
  } catch {
    return { data: str, error: true };
  }
}

// ============================================================================
// JSON Highlighting
// ============================================================================

/**
 * Simple JSON syntax highlighter
 * Colorizes keys, strings, numbers, booleans, and null values
 */
export function highlightJSON(json: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  let key = 0;

  // Match patterns: strings, numbers, booleans, null, keys, brackets/braces
  const regex = /("(?:[^"\\]|\\.)*")\s*:|("(?:[^"\\]|\\.)*")|(-?\d+\.?\d*(?:[eE][+-]?\d+)?)|(\btrue\b|\bfalse\b)|(\bnull\b)|([[\]{}:,])/g;
  let lastIndex = 0;
  let match;

  while ((match = regex.exec(json)) !== null) {
    // Add any text before the match
    if (match.index > lastIndex) {
      parts.push(createElement("span", { key: key++ }, json.slice(lastIndex, match.index)));
    }

    if (match[1]) {
      // Key (with colon)
      parts.push(
        createElement("span", { key: key++, style: { color: "#f0f0f0" } }, match[1])
      );
      parts.push(
        createElement("span", { key: key++, style: { color: "var(--text-muted)" } }, ":")
      );
    } else if (match[2]) {
      // String value
      parts.push(
        createElement("span", { key: key++, style: { color: "#a5d6a7" } }, match[2])
      );
    } else if (match[3]) {
      // Number
      parts.push(
        createElement("span", { key: key++, style: { color: "#ffcc80" } }, match[3])
      );
    } else if (match[4]) {
      // Boolean
      parts.push(
        createElement("span", { key: key++, style: { color: "#81d4fa" } }, match[4])
      );
    } else if (match[5]) {
      // Null
      parts.push(
        createElement("span", { key: key++, style: { color: "#ce93d8" } }, match[5])
      );
    } else if (match[6]) {
      // Brackets, braces, colons, commas
      parts.push(
        createElement("span", { key: key++, style: { color: "var(--text-muted)" } }, match[6])
      );
    }

    lastIndex = regex.lastIndex;
  }

  // Add any remaining text
  if (lastIndex < json.length) {
    parts.push(createElement("span", { key: key++ }, json.slice(lastIndex)));
  }

  return parts;
}

// ============================================================================
// Message Conversion Utilities
// ============================================================================

/**
 * Convert a historical ActivityEventResponse to UnifiedActivityMessage
 */
export function toUnifiedMessage(event: ActivityEventResponse): UnifiedActivityMessage {
  // Safely parse metadata - if it fails, log and continue without metadata
  let parsedMetadata: Record<string, unknown> | undefined;
  if (event.metadata) {
    const result = safeJsonParse<Record<string, unknown>>(event.metadata);
    if (!result.error && typeof result.data === "object" && result.data !== null) {
      parsedMetadata = result.data as Record<string, unknown>;
    }
    // On error, metadata is left undefined (graceful degradation)
  }

  return {
    id: event.id,
    type: event.eventType as AgentMessageEvent["type"],
    content: event.content,
    timestamp: new Date(event.createdAt).getTime(),
    metadata: parsedMetadata,
    taskId: event.taskId ?? undefined,
    sessionId: event.ideationSessionId ?? undefined,
    internalStatus: event.internalStatus,
    role: event.role ?? undefined,
  };
}

/**
 * Convert a real-time AgentMessageEvent to UnifiedActivityMessage
 */
export function fromRealtimeMessage(msg: AgentMessageEvent, index: number): UnifiedActivityMessage {
  return {
    id: `realtime-${msg.taskId}-${msg.timestamp}-${index}`,
    type: msg.type,
    content: msg.content,
    timestamp: msg.timestamp,
    metadata: msg.metadata,
    taskId: msg.taskId,
  };
}
