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
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { DiffToolCallView } from "./DiffToolCallView";

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
  /** Diff context for Edit/Write tool calls (old file content for computing diffs) */
  diffContext?: {
    oldContent?: string;
    filePath: string;
  };
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
  /* macOS Tahoe: flat colors */
  const style = { color: hasError ? "hsl(0 70% 65%)" : "hsl(14 100% 60%)" };
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
  // Hooks must be called unconditionally (React rules-of-hooks)
  const [isExpanded, setIsExpanded] = useState(toolCall.name.toLowerCase() === "bash");
  const summary = useMemo(() => createSummary(toolCall), [toolCall]);
  const hasError = Boolean(toolCall.error);

  // Delegate Edit/Write to DiffToolCallView for inline diff rendering
  // Quick check: arguments must have file_path for diff to work (same gate as DiffToolCallView)
  if (isDiffToolCall(toolCall.name)) {
    const args = toolCall.arguments;
    const hasFilePath = args != null && typeof args === "object" && typeof (args as Record<string, unknown>).file_path === "string" && (args as Record<string, unknown>).file_path !== "";
    if (hasFilePath && !hasError) {
      return <DiffToolCallView toolCall={toolCall} className={className} />;
    }
  }

  return (
    <div
      data-testid="tool-call-indicator"
      className={`rounded-lg overflow-hidden max-w-full ${className}`}
      style={{
        /* macOS Tahoe: flat solid background, no border */
        backgroundColor: hasError ? "hsla(0 70% 55% / 0.15)" : "hsl(220 10% 14%)",
        border: "none",
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
            style={{ color: "hsl(220 10% 45%)" }}
          />
        ) : (
          <ChevronRight
            size={14}
            className="flex-shrink-0"
            style={{ color: "hsl(220 10% 45%)" }}
          />
        )}

        {/* Tool icon */}
        <ToolIcon name={toolCall.name} hasError={hasError} />

        {/* Tool name badge - macOS Tahoe flat style */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
          style={{
            /* macOS Tahoe: subtle solid background */
            backgroundColor: hasError ? "hsla(0 0% 0% / 0.2)" : "hsl(220 10% 10%)",
            color: hasError ? "hsl(220 10% 90%)" : "hsl(220 10% 55%)",
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
              color: hasError ? "hsl(0 70% 75%)" : "hsl(220 10% 75%)",
            }}
          >
            {summary.title}
          </span>
          {summary.subtitle && (
            <span
              className="text-[10px] truncate"
              style={{ color: "hsl(220 10% 50%)" }}
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
              /* macOS Tahoe: subtle error background */
              backgroundColor: "hsla(0 70% 50% / 0.2)",
              color: "hsl(0 70% 70%)",
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
          className="px-3 pb-3 space-y-2 pt-2"
          style={{
            /* macOS Tahoe: no border separator */
            borderTop: "1px solid hsla(220 10% 100% / 0.04)",
          }}
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
              style={{ color: "hsl(220 10% 45%)" }}
            >
              Arguments
            </div>
            <pre
              className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
              style={{
                /* macOS Tahoe: flat dark background */
                backgroundColor: "hsl(220 10% 10%)",
                color: "hsl(220 10% 80%)",
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
                style={{ color: "hsl(220 10% 45%)" }}
              >
                Result
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
                style={{
                  /* macOS Tahoe: flat dark background */
                  backgroundColor: "hsl(220 10% 10%)",
                  color: "hsl(220 10% 80%)",
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
                style={{ color: "hsl(0 70% 70%)" }}
              >
                Error
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto"
                style={{
                  /* macOS Tahoe: error tinted background */
                  backgroundColor: "hsla(0 70% 50% / 0.1)",
                  color: "hsl(0 70% 75%)",
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
