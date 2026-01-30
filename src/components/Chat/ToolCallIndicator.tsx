/**
 * ToolCallIndicator - Displays tool calls made by Claude during chat (final render)
 *
 * Features:
 * - Collapsible view (summary by default, expand for raw data)
 * - Shows tool name and smart summary extracted from arguments
 * - Expands to show formatted JSON (no nested collapse)
 * - Compact, readable design
 */

import React, { useState, useMemo } from "react";
import { Wrench, ChevronDown, ChevronRight, FileText, Terminal, FileEdit, Search, FolderSearch } from "lucide-react";
import { createSummary, formatValue, isArtifactContextTool, renderArtifactPreview } from "./ToolCallIndicator.helpers";

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
  /** Arguments passed to the tool (can be any JSON value) */
  arguments: unknown;
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
// Component
// ============================================================================

/**
 * Render tool icon based on tool name
 */
function ToolIcon({ name, hasError }: { name: string; hasError: boolean }) {
  const style = { color: hasError ? "var(--text-primary)" : "var(--accent-primary)" };
  const className = "flex-shrink-0";

  switch (name) {
    case "bash":
      return <Terminal size={14} className={className} style={style} />;
    case "read":
    case "write":
      return <FileText size={14} className={className} style={style} />;
    case "edit":
      return <FileEdit size={14} className={className} style={style} />;
    case "glob":
      return <FolderSearch size={14} className={className} style={style} />;
    case "grep":
      return <Search size={14} className={className} style={style} />;
    default:
      return <Wrench size={14} className={className} style={style} />;
  }
}

export const ToolCallIndicator = React.memo(function ToolCallIndicator({ toolCall, className = "" }: ToolCallIndicatorProps) {
  // Bash tool calls are expanded by default for that terminal feel
  const [isExpanded, setIsExpanded] = useState(toolCall.name.toLowerCase() === "bash");
  const summary = useMemo(() => createSummary(toolCall), [toolCall]);
  const hasError = Boolean(toolCall.error);

  return (
    <div
      data-testid="tool-call-indicator"
      className={`rounded-md border overflow-hidden max-w-full ${className}`}
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
        <ToolIcon name={toolCall.name} hasError={hasError} />

        {/* Tool name badge */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
          style={{
            backgroundColor: hasError ? "rgba(0,0,0,0.2)" : "var(--bg-base)",
            color: hasError ? "var(--text-primary)" : "var(--text-muted)",
            fontFamily: "var(--font-mono)",
          }}
        >
          {toolCall.name}
        </span>

        {/* Summary text */}
        <div className="flex-1 min-w-0 flex flex-col">
          <span
            className="text-xs truncate font-mono"
            style={{
              color: hasError ? "var(--text-primary)" : "var(--text-secondary)",
            }}
          >
            {summary.title}
          </span>
          {summary.subtitle && (
            <span
              className="text-[10px] truncate"
              style={{ color: "var(--text-muted)" }}
            >
              {summary.subtitle}
            </span>
          )}
        </div>

        {/* Error indicator */}
        {hasError && (
          <span
            className="text-[10px] font-medium px-1.5 py-0.5 rounded"
            style={{
              backgroundColor: "rgba(0,0,0,0.2)",
              color: "var(--text-primary)",
            }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded details - NO nested collapse, show raw data directly */}
      {isExpanded && (
        <div
          data-testid="tool-call-details"
          className="px-3 pb-3 space-y-2 border-t pt-3"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          {/* Artifact preview for context tools */}
          {isArtifactContextTool(toolCall.name) && toolCall.result && !hasError ? (
            <div data-testid="artifact-preview" className="mb-3">
              {renderArtifactPreview(toolCall)}
            </div>
          ) : null}

          {/* Arguments - shown directly */}
          <div>
            <div
              className="text-[10px] font-medium mb-1 uppercase tracking-wide"
              style={{ color: "var(--text-muted)" }}
            >
              Arguments
            </div>
            <pre
              className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-primary)",
                fontFamily: "var(--font-mono)",
                wordBreak: "break-word",
                whiteSpace: "pre-wrap",
              }}
            >
              {formatValue(toolCall.arguments).text}
            </pre>
          </div>

          {/* Result - shown directly (if present and not null) */}
          {toolCall.result != null && !hasError && (
            <div>
              <div
                className="text-[10px] font-medium mb-1 uppercase tracking-wide"
                style={{ color: "var(--text-muted)" }}
              >
                Result
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: "var(--text-primary)",
                  fontFamily: "var(--font-mono)",
                  wordBreak: "break-word",
                  whiteSpace: "pre-wrap",
                }}
              >
                {formatValue(toolCall.result).text}
              </pre>
            </div>
          )}

          {/* Error (if present) */}
          {hasError && (
            <div>
              <div
                className="text-[10px] font-medium mb-1 uppercase tracking-wide"
                style={{ color: "var(--text-primary)" }}
              >
                Error
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto"
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
});
