/**
 * GrepWidget — Search result card for Grep tool calls.
 *
 * Design: search icon + pattern in monospace + scope path + "N files" badge.
 * Body: matched file paths listed.
 * Inline (no collapse) when ≤3 results, collapsible when more.
 *
 * Reference: Widget 10 in mockups/tool-call-widgets.html
 */

import React from "react";
import { Search, FileText } from "lucide-react";
import { WidgetCard, WidgetHeader, Badge } from "./shared";
import type { ToolCallWidgetProps } from "./shared";
import { colors, parseToolResultAsLines } from "./shared.constants";

const MAX_INLINE_RESULTS = 3;

/** Parse Grep arguments from tool call */
function parseGrepArgs(args: unknown): {
  pattern: string;
  path: string | undefined;
  outputMode: string | undefined;
} {
  const typed = args as {
    pattern?: string;
    path?: string;
    output_mode?: string;
  } | undefined;
  return {
    pattern: typed?.pattern || "search",
    path: typed?.path,
    outputMode: typed?.output_mode,
  };
}

export const GrepWidget = React.memo(function GrepWidget({
  toolCall,
  compact = false,
}: ToolCallWidgetProps) {
  const { pattern, path } = parseGrepArgs(toolCall.arguments);
  const files = parseToolResultAsLines(toolCall.result);
  const fileCount = files.length;
  const isInline = fileCount <= MAX_INLINE_RESULTS;

  // Build title: pattern in monospace, optionally with scope path
  const title = path ? `"${pattern}" in ${path}` : `"${pattern}"`;

  // Badge text
  const badgeText = fileCount === 0
    ? "no results"
    : fileCount === 1
      ? "1 file"
      : `${fileCount} files`;

  const header = (
    <WidgetHeader
      icon={<Search size={14} />}
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

  // No results: just header, no body
  if (fileCount === 0 && toolCall.result !== undefined) {
    return (
      <WidgetCard
        header={header}
        compact={compact}
        alwaysExpanded
      >
        <span style={{ fontSize: 10.5, color: colors.textMuted }}>
          No matches found
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

/** File list body shared between Grep and Glob */
function FileList({ files }: { files: string[] }) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        lineHeight: 1.6,
        color: colors.textSecondary,
        padding: "4px 0",
      }}
    >
      {files.map((filePath, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "1px 0",
          }}
        >
          <FileText
            size={12}
            style={{ color: colors.textMuted, flexShrink: 0 }}
          />
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {filePath}
          </span>
        </div>
      ))}
    </div>
  );
}

export { FileList };
