/**
 * BashWidget — Terminal Output Card
 *
 * Design reference: mockups/tool-call-widgets.html (Widget 11)
 * Replaces generic auto-expanded bash renderer with a compact terminal card.
 *
 * - Header: terminal icon + description (or truncated command) + optional duration + exit code badge
 * - Body: command with green $ prompt, output on darker bg with gradient fade
 * - Collapsed by default (~3 lines), auto-expand on non-zero exit code
 * - Strips ANSI codes from output
 * - Shows live elapsed timer during execution; static final duration after completion
 */

import React, { useContext, useEffect, useMemo, useState } from "react";
import { Terminal } from "lucide-react";

import { WidgetCard, WidgetHeader, Badge } from "./shared";
import { colors, truncate } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";
import { stripAnsiCodes } from "../ToolCallIndicator.helpers";
import { ToolCallStoreKeyContext } from "./ToolCallStoreKeyContext";
import { useChatStore, selectToolCallStartTimes, selectToolCallCompletionTimestamps } from "@/stores/chatStore";
import { formatDuration } from "@/components/tasks/detail-views/shared/DurationDisplay";

// ============================================================================
// Helpers
// ============================================================================

interface BashArgs {
  command?: string;
  description?: string;
}

interface ParsedBash {
  command: string;
  description: string;
  output: string;
  exitCode: number | null;
  hasError: boolean;
}

/** Extract command, description, output, and exit code from tool call data */
function parseBashToolCall(toolCall: ToolCallWidgetProps["toolCall"]): ParsedBash {
  const args = (toolCall.arguments ?? {}) as BashArgs;
  const command = args.command ?? "";
  const description = args.description ?? "";

  // Result is a string (stdout/stderr output) or an object with text
  let output = "";
  if (typeof toolCall.result === "string") {
    output = stripAnsiCodes(toolCall.result);
  } else if (toolCall.result != null) {
    // MCP result array wrapper: [{text: "..."}]
    const resultObj = toolCall.result as Record<string, unknown>;
    if (Array.isArray(resultObj)) {
      const texts = resultObj
        .filter((item): item is { text: string } => typeof item === "object" && item !== null && typeof (item as Record<string, unknown>).text === "string")
        .map((item) => item.text);
      output = stripAnsiCodes(texts.join("\n"));
    } else if (typeof resultObj.text === "string") {
      output = stripAnsiCodes(resultObj.text);
    } else {
      try {
        output = stripAnsiCodes(JSON.stringify(toolCall.result, null, 2));
      } catch {
        output = stripAnsiCodes(String(toolCall.result));
      }
    }
  }

  // Exit code: look for it in the result text (common pattern: "exit code: N")
  // or infer from error state
  let exitCode: number | null = null;
  if (toolCall.error) {
    // Try to extract exit code from error message
    const match = toolCall.error.match(/exit (?:code|status)[:\s]*(\d+)/i);
    exitCode = match?.[1] ? parseInt(match[1], 10) : 1;
  } else {
    // Check output for exit code patterns
    const match = output.match(/exit (?:code|status)[:\s]*(\d+)/i);
    if (match?.[1]) {
      exitCode = parseInt(match[1], 10);
    } else {
      // If result is present and no error → exit 0
      exitCode = toolCall.result != null ? 0 : null;
    }
  }

  const hasError = Boolean(toolCall.error) || (exitCode !== null && exitCode !== 0);

  return { command, description, output, exitCode, hasError };
}

// ============================================================================
// Duration display hook
// ============================================================================

/**
 * Returns a formatted duration string for a tool call:
 * - In-progress: live elapsed time ticking every second
 * - Completed: static final duration
 * - Neither: null (backward compat — historical messages without timing)
 */
function useBashDuration(toolCallId: string, storeKey: string | null): string | null {
  const startTimes = useChatStore(selectToolCallStartTimes(storeKey ?? ""));
  const completionTimestamps = useChatStore(selectToolCallCompletionTimestamps(storeKey ?? ""));

  const startTime = storeKey ? (startTimes[toolCallId] ?? null) : null;
  const completedAt = storeKey ? (completionTimestamps[toolCallId] ?? null) : null;

  // Live elapsed timer when in-progress (startTime set, no completedAt)
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (startTime === null || completedAt !== null) return;
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, [startTime, completedAt]);

  if (startTime === null) return null;

  if (completedAt !== null) {
    // Static final duration
    const seconds = Math.round((completedAt - startTime) / 1000);
    return formatDuration(seconds);
  }

  // Live elapsed
  const elapsed = Math.round((now - startTime) / 1000);
  return formatDuration(elapsed);
}

// ============================================================================
// Component
// ============================================================================

export const BashWidget = React.memo(function BashWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const parsed = useMemo(() => parseBashToolCall(toolCall), [toolCall]);
  const storeKey = useContext(ToolCallStoreKeyContext);
  const duration = useBashDuration(toolCall.id, storeKey);

  // Header title: prefer description, fall back to truncated command
  const headerTitle = parsed.description || truncate(parsed.command, 60) || "Ran command";

  // Exit code badge
  const exitBadge = parsed.exitCode !== null ? (
    <Badge variant={parsed.hasError ? "error" : "success"} compact>
      {parsed.hasError ? `exit ${parsed.exitCode}` : "exit 0"}
    </Badge>
  ) : null;

  // Duration + exit badge composed together
  const headerBadge = (
    <>
      {duration !== null && (
        <span
          style={{
            fontSize: 10.5,
            color: colors.textMuted,
            flexShrink: 0,
            whiteSpace: "nowrap",
          }}
          data-testid="bash-duration"
        >
          {duration}
        </span>
      )}
      {exitBadge}
    </>
  );

  // Auto-expand on non-zero exit code
  const defaultExpanded = parsed.hasError;

  return (
    <WidgetCard
      className={className}
      compact={compact}
      defaultExpanded={defaultExpanded}
      header={
        <WidgetHeader
          icon={<Terminal size={14} />}
          title={headerTitle}
          badge={headerBadge}
          compact={compact}
        />
      }
    >
      {/* Command line with green $ prompt */}
      {parsed.command && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: compact ? 10 : 10.5,
            color: colors.textMuted,
            padding: "4px 0 2px",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          <span style={{ color: colors.success, marginRight: 6 }}>$</span>
          {parsed.command}
        </div>
      )}

      {/* Terminal output on darker background */}
      {parsed.output && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: compact ? 10 : 11,
            lineHeight: 1.55,
            color: parsed.hasError ? colors.error : colors.textSecondary,
            background: colors.bgTerminal,
            borderRadius: 4,
            padding: "6px 8px",
            marginTop: 4,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {parsed.output}
        </div>
      )}
    </WidgetCard>
  );
});
