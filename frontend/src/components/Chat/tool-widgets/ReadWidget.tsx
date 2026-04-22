/**
 * ReadWidget — File Preview Card
 *
 * Replaces the generic Read tool call renderer with a compact file preview.
 * Header: file icon + normalized repo-relative path + line count badge
 * Body: parsed lines (prefixes stripped) in monospace with line numbers + gradient fade
 * Collapse/expand interaction matches Edit diff pattern.
 *
 * Uses parseReadOutput() for prefix stripping, start-line inference, and error extraction.
 * Uses normalizeDisplayPath() + shortenPath() for repo-relative header display.
 *
 * Design reference: Widget 6 in mockups/tool-call-widgets.html
 */

import React, { useMemo } from "react";
import { FileText } from "lucide-react";
import {
  WidgetCard,
  WidgetHeader,
  CodePreview,
  Badge,
  type ToolCallWidgetProps,
} from "./shared";
import {
  shortenPath,
  normalizeDisplayPath,
  parseReadOutput,
  getNumber,
} from "./shared.constants";

/** Extract file path from tool call arguments */
function extractFilePath(args: unknown): string {
  if (args && typeof args === "object") {
    const a = args as Record<string, unknown>;
    if (typeof a.file_path === "string") return a.file_path;
    if (typeof a.path === "string") return a.path;
  }
  return "file";
}

/** Compute line count badge text from parsed lines and/or arguments */
function computeLineCountLabel(lineCount: number, args: unknown): string | null {
  if (lineCount > 0) {
    return `${lineCount} line${lineCount !== 1 ? "s" : ""}`;
  }

  // If arguments specify a limit, show that
  if (args && typeof args === "object") {
    const a = args as Record<string, unknown>;
    if (typeof a.limit === "number") return `${a.limit} lines`;
    if (typeof a.start_line === "number" && typeof a.end_line === "number") {
      const requested = a.end_line - a.start_line + 1;
      if (requested > 0) return `${requested} lines`;
    }
  }

  return null;
}

export const ReadWidget = React.memo(function ReadWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const rawFilePath = useMemo(() => extractFilePath(toolCall.arguments), [toolCall.arguments]);
  const displayPath = useMemo(
    () => shortenPath(normalizeDisplayPath(rawFilePath), compact ? 40 : 50),
    [rawFilePath, compact],
  );

  const offset = useMemo(() => getNumber(toolCall.arguments, "offset"), [toolCall.arguments]);

  const parsed = useMemo(
    () => parseReadOutput(toolCall.result, offset),
    [toolCall.result, offset],
  );

  const lineCountLabel = useMemo(
    () => computeLineCountLabel(parsed.lines.length, toolCall.arguments),
    [parsed.lines.length, toolCall.arguments],
  );

  const hasError = Boolean(toolCall.error) || Boolean(parsed.error);
  const hasContent = parsed.lines.length > 0;
  const errorText = parsed.error || toolCall.error;

  // For error or no-content cases, still show the card header
  const header = (
    <WidgetHeader
      icon={<FileText size={14} />}
      title={displayPath}
      mono
      compact={compact}
      badge={
        hasError ? (
          <Badge variant="error" compact>error</Badge>
        ) : lineCountLabel ? (
          <Badge variant="muted" compact>{lineCountLabel}</Badge>
        ) : undefined
      }
    />
  );

  // If no content (still loading or error), show collapsed card with just header
  if (!hasContent && !hasError) {
    return (
      <WidgetCard header={header} compact={compact} className={className}>
        <div style={{ fontSize: 10.5, color: "var(--text-muted)", padding: "4px 0" }}>
          Reading...
        </div>
      </WidgetCard>
    );
  }

  if (hasError) {
    return (
      <WidgetCard header={header} compact={compact} className={className}>
        <div style={{ fontSize: 11, color: "var(--status-error)", fontFamily: "var(--font-mono)", padding: "4px 0" }}>
          {errorText}
        </div>
      </WidgetCard>
    );
  }

  return (
    <WidgetCard header={header} compact={compact} className={className}>
      <CodePreview lines={parsed.lines} startLine={parsed.inferredStartLine} compact={compact} />
    </WidgetCard>
  );
});
