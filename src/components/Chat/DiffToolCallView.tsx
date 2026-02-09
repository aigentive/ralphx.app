/**
 * DiffToolCallView - Renders Edit/Write tool calls as inline diff cards
 *
 * Features:
 * - Collapsed: ~3.65 line preview (73px) with gradient blur fade
 * - Expanded: full diff with line numbers, red/green line highlighting
 * - Header: chevron + tool icon + file path + additions/deletions stats
 * - Falls back to null if no file_path or error, letting parent render generic view
 */

import React, { useState, useMemo } from "react";
import { ChevronDown, ChevronRight, FileEdit, FileText } from "lucide-react";
import type { ToolCall } from "./ToolCallIndicator";
import {
  type DiffLine,
  type DiffResult,
  extractEditDiff,
  extractWriteDiff,
  getLineBackground,
  getLineNumColor,
  getLinePrefix,
  getPrefixColor,
} from "./DiffToolCallView.utils";

// ============================================================================
// Constants
// ============================================================================

/** Height for ~3.65 lines at 20px line-height */
const COLLAPSED_HEIGHT = 73;
/** Lines threshold: if total diff lines <= this, show fully (no blur) */
const MIN_LINES_FOR_COLLAPSE = 4;
/** Height of gradient blur overlay */
const GRADIENT_HEIGHT = 24;

// ============================================================================
// Types
// ============================================================================

interface DiffToolCallViewProps {
  toolCall: ToolCall;
  isStreaming?: boolean;
  className?: string;
  /** Compact mode for rendering inside task cards — smaller padding, text, icons */
  compact?: boolean;
}

// ============================================================================
// Helpers
// ============================================================================

function extractDiff(toolCall: ToolCall): DiffResult | null {
  const name = toolCall.name.toLowerCase();
  if (name === "edit") return extractEditDiff(toolCall);
  if (name === "write") return extractWriteDiff(toolCall);
  return null;
}

function shortenPath(filePath: string): string {
  const parts = filePath.split("/");
  if (parts.length <= 3) return filePath;
  return ".../" + parts.slice(-3).join("/");
}

// ============================================================================
// Component
// ============================================================================

export const DiffToolCallView = React.memo(function DiffToolCallView({
  toolCall,
  isStreaming,
  className = "",
  compact = false,
}: DiffToolCallViewProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const diff = useMemo(() => extractDiff(toolCall), [toolCall]);

  // Fall back to null so parent can render generic view
  if (!diff) return null;

  const { lines, filePath, additions, deletions } = diff;
  const isEdit = toolCall.name.toLowerCase() === "edit";
  const needsCollapse = lines.length > MIN_LINES_FOR_COLLAPSE;
  const showFull = isExpanded || !needsCollapse;

  const iconSize = compact ? 12 : 14;

  return (
    <div
      data-testid="diff-tool-call-view"
      className={`${compact ? "rounded-md" : "rounded-lg"} overflow-hidden max-w-full ${compact ? "mb-1" : ""} ${className}`}
      style={{
        backgroundColor: "hsl(220 10% 14%)",
        border: "none",
      }}
    >
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={`w-full flex items-center gap-2 ${compact ? "px-2 py-1.5" : "px-3 py-2"} text-left hover:opacity-80 transition-opacity`}
        aria-expanded={isExpanded}
        aria-label={`${toolCall.name} ${filePath}. ${additions} additions, ${deletions} deletions. Click to ${isExpanded ? "collapse" : "expand"}.`}
      >
        {/* Chevron */}
        {isExpanded ? (
          <ChevronDown size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220 10% 45%)" }} />
        ) : (
          <ChevronRight size={iconSize} className="flex-shrink-0" style={{ color: "hsl(220 10% 45%)" }} />
        )}

        {/* Tool icon */}
        {isEdit ? (
          <FileEdit size={iconSize} className="flex-shrink-0" style={{ color: "hsl(14 100% 60%)" }} />
        ) : (
          <FileText size={iconSize} className="flex-shrink-0" style={{ color: "hsl(14 100% 60%)" }} />
        )}

        {/* Tool name badge */}
        <span
          className={`${compact ? "text-[9px]" : "text-[10px]"} px-1.5 py-0.5 rounded flex-shrink-0`}
          style={{
            backgroundColor: "hsl(220 10% 10%)",
            color: "hsl(220 10% 55%)",
            fontFamily: "var(--font-mono)",
          }}
        >
          {toolCall.name}
        </span>

        {/* File path */}
        <span
          className={`${compact ? "text-[11px]" : "text-xs"} truncate font-mono flex-1 min-w-0`}
          style={{ color: "hsl(220 10% 75%)" }}
        >
          {shortenPath(filePath)}
        </span>

        {/* Stats badge */}
        <span className={`flex-shrink-0 flex items-center gap-1 ${compact ? "text-[9px]" : "text-[10px]"} font-mono`}>
          {additions > 0 && (
            <span style={{ color: "#34c759" }}>+{additions}</span>
          )}
          {deletions > 0 && (
            <span style={{ color: "#ff453a" }}>-{deletions}</span>
          )}
        </span>

        {/* Streaming indicator */}
        {isStreaming && (
          <span
            className={`${compact ? "text-[9px]" : "text-[10px]"} px-1.5 py-0.5 rounded flex-shrink-0 animate-pulse`}
            style={{
              backgroundColor: "hsla(14 100% 60% / 0.15)",
              color: "hsl(14 100% 60%)",
            }}
          >
            writing...
          </span>
        )}
      </button>

      {/* Diff content */}
      <div
        style={{
          position: "relative",
          overflow: "hidden",
          maxHeight: showFull ? "none" : `${COLLAPSED_HEIGHT}px`,
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: "11px",
            lineHeight: "20px",
          }}
        >
          {lines.map((line: DiffLine, i: number) => (
            <DiffLineRow key={i} line={line} />
          ))}
        </div>

        {/* Gradient blur overlay (collapsed only) */}
        {!showFull && (
          <div
            style={{
              position: "absolute",
              bottom: 0,
              left: 0,
              right: 0,
              height: `${GRADIENT_HEIGHT}px`,
              background: "linear-gradient(transparent, hsl(220 10% 14%))",
              pointerEvents: "none",
            }}
          />
        )}
      </div>
    </div>
  );
});

// ============================================================================
// Sub-components
// ============================================================================

const DiffLineRow = React.memo(function DiffLineRow({ line }: { line: DiffLine }) {
  return (
    <div
      className="flex"
      style={{
        backgroundColor: getLineBackground(line.type),
        paddingLeft: "8px",
        paddingRight: "8px",
      }}
    >
      {/* Old line number */}
      <span
        className="select-none text-right flex-shrink-0"
        style={{
          width: "32px",
          color: getLineNumColor(line.type),
          userSelect: "none",
        }}
      >
        {line.oldLineNum ?? ""}
      </span>

      {/* New line number */}
      <span
        className="select-none text-right flex-shrink-0"
        style={{
          width: "32px",
          color: getLineNumColor(line.type),
          userSelect: "none",
          marginRight: "4px",
        }}
      >
        {line.newLineNum ?? ""}
      </span>

      {/* Prefix (+/-/space) */}
      <span
        className="flex-shrink-0"
        style={{
          width: "16px",
          color: getPrefixColor(line.type),
          textAlign: "center",
        }}
      >
        {getLinePrefix(line.type)}
      </span>

      {/* Content */}
      <span
        className="whitespace-pre overflow-hidden text-ellipsis"
        style={{ color: "hsl(220 10% 80%)" }}
      >
        {line.content}
      </span>
    </div>
  );
});
