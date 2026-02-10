/**
 * ReadWidget — File Preview Card
 *
 * Replaces the generic Read tool call renderer with a compact file preview.
 * Header: file icon + shortened path + line count badge
 * Body: first ~5 lines of file content in monospace with line numbers + gradient fade
 * Collapse/expand interaction matches Edit diff pattern.
 *
 * Handles both direct string results and MCP array wrapper [{text: "..."}] format.
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
import { shortenPath } from "./shared.constants";

/** Extract file content string from tool call result */
function extractContent(result: unknown): string | null {
  if (typeof result === "string") return result;

  // MCP array wrapper: [{type: "text", text: "..."}, ...]
  if (Array.isArray(result)) {
    const texts: string[] = [];
    for (const item of result) {
      if (item && typeof item === "object" && "text" in item && typeof (item as { text: unknown }).text === "string") {
        texts.push((item as { text: string }).text);
      }
    }
    if (texts.length > 0) return texts.join("\n");
  }

  // Object with content/text field
  if (result && typeof result === "object") {
    const obj = result as Record<string, unknown>;
    if (typeof obj.text === "string") return obj.text;
    if (typeof obj.content === "string") return obj.content;
  }

  return null;
}

/** Extract file path from tool call arguments */
function extractFilePath(args: unknown): string {
  if (args && typeof args === "object") {
    const a = args as Record<string, unknown>;
    if (typeof a.file_path === "string") return a.file_path;
  }
  return "file";
}

/** Extract start line from arguments (offset param) */
function extractStartLine(args: unknown): number {
  if (args && typeof args === "object") {
    const a = args as Record<string, unknown>;
    if (typeof a.offset === "number" && a.offset > 0) return a.offset;
  }
  return 1;
}

/** Compute line count badge text from content and/or arguments */
function computeLineCountLabel(content: string | null, args: unknown): string | null {
  // If we have content, count its lines
  if (content) {
    const count = content.split("\n").length;
    return `${count} line${count !== 1 ? "s" : ""}`;
  }

  // If arguments specify a limit, show that
  if (args && typeof args === "object") {
    const a = args as Record<string, unknown>;
    if (typeof a.limit === "number") return `${a.limit} lines`;
  }

  return null;
}

export const ReadWidget = React.memo(function ReadWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const filePath = useMemo(() => extractFilePath(toolCall.arguments), [toolCall.arguments]);
  const content = useMemo(() => extractContent(toolCall.result), [toolCall.result]);
  const startLine = useMemo(() => extractStartLine(toolCall.arguments), [toolCall.arguments]);
  const lineCountLabel = useMemo(
    () => computeLineCountLabel(content, toolCall.arguments),
    [content, toolCall.arguments],
  );

  const allLines = useMemo(() => {
    if (!content) return [];
    return content.split("\n");
  }, [content]);

  const hasError = Boolean(toolCall.error);
  const hasContent = allLines.length > 0;

  // For error or no-content cases, still show the card header
  const header = (
    <WidgetHeader
      icon={<FileText size={14} />}
      title={shortenPath(filePath, compact ? 40 : 50)}
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
        <div style={{ fontSize: 10.5, color: "hsl(220 10% 45%)", padding: "4px 0" }}>
          Reading...
        </div>
      </WidgetCard>
    );
  }

  if (hasError) {
    return (
      <WidgetCard header={header} compact={compact} className={className}>
        <div style={{ fontSize: 11, color: "hsl(0 70% 65%)", fontFamily: "var(--font-mono)", padding: "4px 0" }}>
          {toolCall.error}
        </div>
      </WidgetCard>
    );
  }

  return (
    <WidgetCard header={header} compact={compact} className={className}>
      <CodePreview lines={allLines} startLine={startLine} compact={compact} />
    </WidgetCard>
  );
});
