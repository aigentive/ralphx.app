/**
 * ToolCallIndicator - Displays tool calls made by Claude during chat
 *
 * Features:
 * - Collapsible view (summary by default, expand for details)
 * - Shows tool name, icon, and brief summary
 * - Expands to show full arguments and result
 * - Styled with design system tokens for consistency
 */

import { useState } from "react";
import { Wrench, ChevronDown, ChevronRight } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

/**
 * Tool call structure from Claude CLI stream-json output
 */
export interface ToolCall {
  /** Unique identifier for this tool call */
  id: string;
  /** Name of the tool that was called */
  name: string;
  /** Input arguments passed to the tool (can be any JSON value) */
  input: unknown;
  /** Result returned from the tool (can be any JSON value) */
  result?: unknown;
  /** Error message if tool call failed */
  error?: string;
}

interface ToolCallIndicatorProps {
  /** The tool call to display */
  toolCall: ToolCall;
  /** Optional additional className for container */
  className?: string;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Create a brief summary of the tool call for collapsed view
 */
function createSummary(toolCall: ToolCall): string {
  const { name, input } = toolCall;

  // Special formatting for common tools
  switch (name) {
    case "bash":
      return `Ran command: ${truncate(String((input as { command?: string })?.command || ""), 60)}`;
    case "read":
      return `Read file: ${(input as { file_path?: string })?.file_path || "unknown"}`;
    case "write":
      return `Wrote file: ${(input as { file_path?: string })?.file_path || "unknown"}`;
    case "edit":
      return `Edited file: ${(input as { file_path?: string })?.file_path || "unknown"}`;
    case "create_task_proposal":
      return `Created proposal: ${(input as { title?: string })?.title || "untitled"}`;
    case "update_task_proposal":
      return `Updated proposal: ${(input as { proposal_id?: string })?.proposal_id || "unknown"}`;
    case "delete_task_proposal":
      return `Deleted proposal: ${(input as { proposal_id?: string })?.proposal_id || "unknown"}`;
    case "update_task":
      return `Updated task: ${(input as { task_id?: string })?.task_id || "unknown"}`;
    case "add_task_note":
      return `Added note to task: ${(input as { task_id?: string })?.task_id || "unknown"}`;
    default:
      return `Called ${name}`;
  }
}

/**
 * Truncate text with ellipsis
 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}

/**
 * Format JSON for display
 */
function formatJSON(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

// ============================================================================
// Component
// ============================================================================

export function ToolCallIndicator({ toolCall, className = "" }: ToolCallIndicatorProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const summary = createSummary(toolCall);
  const hasError = Boolean(toolCall.error);

  return (
    <div
      data-testid="tool-call-indicator"
      className={`rounded-md border ${className}`}
      style={{
        backgroundColor: hasError ? "var(--status-error)" : "var(--bg-elevated)",
        borderColor: hasError
          ? "rgba(239, 68, 68, 0.3)"
          : "var(--border-subtle)",
        opacity: hasError ? 0.9 : 1,
      }}
    >
      {/* Header - Always visible */}
      <button
        data-testid="tool-call-toggle"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:opacity-80 transition-opacity"
        aria-expanded={isExpanded}
        aria-label={`Tool call: ${toolCall.name}. Click to ${isExpanded ? "collapse" : "expand"} details.`}
      >
        {/* Expand/collapse icon */}
        {isExpanded ? (
          <ChevronDown
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--text-muted)" }}
          />
        ) : (
          <ChevronRight
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--text-muted)" }}
          />
        )}

        {/* Tool icon */}
        <Wrench
          size={14}
          className="flex-shrink-0"
          style={{ color: hasError ? "var(--text-primary)" : "var(--accent-primary)" }}
        />

        {/* Summary text */}
        <span
          className="text-xs flex-1"
          style={{
            color: hasError ? "var(--text-primary)" : "var(--text-secondary)",
            fontFamily: "var(--font-body)",
          }}
        >
          {summary}
        </span>

        {/* Error indicator */}
        {hasError && (
          <span
            className="text-xs font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded details */}
      {isExpanded && (
        <div
          data-testid="tool-call-details"
          className="px-3 pb-3 space-y-2 border-t"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          {/* Tool name */}
          <div>
            <div
              className="text-xs font-medium mb-1"
              style={{ color: "var(--text-muted)" }}
            >
              Tool
            </div>
            <code
              className="text-xs px-2 py-1 rounded block"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-primary)",
                fontFamily: "var(--font-mono)",
              }}
            >
              {toolCall.name}
            </code>
          </div>

          {/* Arguments */}
          <div>
            <div
              className="text-xs font-medium mb-1"
              style={{ color: "var(--text-muted)" }}
            >
              Arguments
            </div>
            <pre
              className="text-xs px-2 py-1 rounded overflow-x-auto"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-primary)",
                fontFamily: "var(--font-mono)",
              }}
            >
              {formatJSON(toolCall.input)}
            </pre>
          </div>

          {/* Result (if present) */}
          {toolCall.result !== undefined && !hasError && (
            <div>
              <div
                className="text-xs font-medium mb-1"
                style={{ color: "var(--text-muted)" }}
              >
                Result
              </div>
              <pre
                className="text-xs px-2 py-1 rounded overflow-x-auto"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: "var(--text-primary)",
                  fontFamily: "var(--font-mono)",
                }}
              >
                {formatJSON(toolCall.result)}
              </pre>
            </div>
          )}

          {/* Error (if present) */}
          {hasError && (
            <div>
              <div
                className="text-xs font-medium mb-1"
                style={{ color: "var(--text-primary)" }}
              >
                Error
              </div>
              <pre
                className="text-xs px-2 py-1 rounded overflow-x-auto"
                style={{
                  backgroundColor: "rgba(0, 0, 0, 0.2)",
                  color: "var(--text-primary)",
                  fontFamily: "var(--font-mono)",
                }}
              >
                {toolCall.error}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
