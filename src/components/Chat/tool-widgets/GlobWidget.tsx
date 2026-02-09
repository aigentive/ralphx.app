/**
 * GlobWidget — File search result card for Glob tool calls.
 *
 * Design: folder-search icon + glob pattern in monospace + "N matches" badge.
 * Body: matched file paths listed.
 * Inline (no collapse) when ≤3 results, collapsible when more.
 *
 * Reference: Widget 9 in mockups/tool-call-widgets.html
 */

import React from "react";
import { FolderSearch } from "lucide-react";
import { WidgetCard, WidgetHeader, Badge } from "./shared";
import type { ToolCallWidgetProps } from "./shared";
import { colors } from "./shared.constants";
import { FileList } from "./GrepWidget";

const MAX_INLINE_RESULTS = 3;

/** Parse Glob arguments from tool call */
function parseGlobArgs(args: unknown): {
  pattern: string;
  path: string | undefined;
} {
  const typed = args as {
    pattern?: string;
    path?: string;
  } | undefined;
  return {
    pattern: typed?.pattern || "**/*",
    path: typed?.path,
  };
}

/**
 * Parse Glob result into file paths.
 * Glob returns a text string with one file path per line,
 * or an MCP result wrapper [{text: "..."}], or an array of strings.
 */
function parseGlobResult(result: unknown): string[] {
  if (!result) return [];

  let text = "";

  if (typeof result === "string") {
    text = result;
  } else if (Array.isArray(result)) {
    // MCP result wrapper: [{type: "text", text: "..."}]
    const first = result[0];
    if (first && typeof first === "object" && "text" in first) {
      text = String((first as { text: string }).text);
    } else {
      // Array of strings
      return result.filter((item): item is string => typeof item === "string");
    }
  } else if (typeof result === "object" && result !== null && "text" in result) {
    text = String((result as { text: string }).text);
  }

  if (!text) return [];

  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export const GlobWidget = React.memo(function GlobWidget({
  toolCall,
  compact = false,
}: ToolCallWidgetProps) {
  const { pattern, path } = parseGlobArgs(toolCall.arguments);
  const files = parseGlobResult(toolCall.result);
  const fileCount = files.length;
  const isInline = fileCount <= MAX_INLINE_RESULTS;

  // Build title: glob pattern in monospace
  const title = path ? `${pattern} in ${path}` : pattern;

  // Badge text
  const badgeText = fileCount === 0
    ? "no matches"
    : fileCount === 1
      ? "1 match"
      : `${fileCount} matches`;

  const header = (
    <WidgetHeader
      icon={<FolderSearch size={14} />}
      title={title}
      mono
      compact={compact}
      badge={
        <Badge variant="muted">
          {badgeText}
        </Badge>
      }
    />
  );

  // No results: just header with message
  if (fileCount === 0 && toolCall.result !== undefined) {
    return (
      <WidgetCard
        header={header}
        compact={compact}
        alwaysExpanded
      >
        <span style={{ fontSize: 10.5, color: colors.textMuted }}>
          No files matched
        </span>
      </WidgetCard>
    );
  }

  // Pending result (tool still running)
  if (toolCall.result === undefined) {
    return (
      <WidgetCard header={header} compact={compact} alwaysExpanded>
        <span style={{ fontSize: 10.5, color: colors.textMuted }}>
          Searching...
        </span>
      </WidgetCard>
    );
  }

  return (
    <WidgetCard
      header={header}
      compact={compact}
      alwaysExpanded={isInline}
    >
      <FileList files={files} />
    </WidgetCard>
  );
});
