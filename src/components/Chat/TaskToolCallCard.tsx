/**
 * TaskToolCallCard - Static card for completed Task tool calls in final messages.
 *
 * Renders Task subagent calls as collapsible cards showing:
 * - Header: subagent type badge, description, model badge, stats
 * - Body (collapsed by default): the subagent's final text output
 *
 * Matches the TaskSubagentCard streaming design but for persisted messages.
 */

import React, { useState, useMemo } from "react";
import { ChevronDown, ChevronRight, Bot } from "lucide-react";
import type { ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Types
// ============================================================================

interface TaskToolCallCardProps {
  toolCall: ToolCall;
  className?: string;
}

interface TaskArgs {
  description: string | undefined;
  subagent_type: string | undefined;
  model: string | undefined;
  prompt: string | undefined;
}

interface TaskStats {
  agentId: string | undefined;
  totalDurationMs: number | undefined;
  totalTokens: number | undefined;
  totalToolUseCount: number | undefined;
  textOutput: string | undefined;
}

// ============================================================================
// Helpers
// ============================================================================

/** Format milliseconds into human-readable duration */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const remainSecs = secs % 60;
  return `${mins}m ${remainSecs}s`;
}

/** Get a display-friendly color for the subagent type badge */
function getSubagentTypeColor(subagentType: string): { bg: string; text: string } {
  const type = subagentType.toLowerCase();
  switch (type) {
    case "explore":
      return { bg: "hsla(200, 70%, 50%, 0.15)", text: "hsl(200, 70%, 65%)" };
    case "plan":
      return { bg: "hsla(280, 60%, 50%, 0.15)", text: "hsl(280, 60%, 70%)" };
    case "bash":
      return { bg: "hsla(140, 60%, 40%, 0.15)", text: "hsl(140, 60%, 65%)" };
    case "general-purpose":
      return { bg: "hsla(220, 60%, 50%, 0.15)", text: "hsl(220, 60%, 70%)" };
    default:
      return { bg: "hsla(220, 10%, 50%, 0.15)", text: "hsl(220, 10%, 65%)" };
  }
}

/** Get model badge color */
function getModelColor(model: string): { bg: string; text: string } {
  const m = model.toLowerCase();
  if (m.includes("opus")) return { bg: "hsla(14, 100%, 60%, 0.15)", text: "hsl(14, 100%, 65%)" };
  if (m.includes("sonnet")) return { bg: "hsla(40, 80%, 50%, 0.15)", text: "hsl(40, 80%, 65%)" };
  if (m.includes("haiku")) return { bg: "hsla(160, 60%, 45%, 0.15)", text: "hsl(160, 60%, 65%)" };
  return { bg: "hsla(220, 10%, 50%, 0.15)", text: "hsl(220, 10%, 65%)" };
}

const EMPTY_ARGS: TaskArgs = {
  description: undefined,
  subagent_type: undefined,
  model: undefined,
  prompt: undefined,
};

/** Extract Task arguments (description, subagent_type, model) */
function extractTaskArgs(args: unknown): TaskArgs {
  if (!args || typeof args !== "object") return EMPTY_ARGS;
  const a = args as Record<string, unknown>;
  return {
    description: typeof a.description === "string" ? a.description : undefined,
    subagent_type: typeof a.subagent_type === "string" ? a.subagent_type : undefined,
    model: typeof a.model === "string" ? a.model : undefined,
    prompt: typeof a.prompt === "string" ? a.prompt : undefined,
  };
}

/**
 * Parse the Task tool result to extract stats.
 *
 * The result text typically looks like:
 * ```
 * [subagent output text here]
 * agentId: abc1234 (for resuming...)
 * <usage>total_tokens: 12345
 * tool_uses: 8
 * duration_ms: 45000</usage>
 * ```
 */
const EMPTY_STATS: TaskStats = {
  agentId: undefined,
  totalDurationMs: undefined,
  totalTokens: undefined,
  totalToolUseCount: undefined,
  textOutput: undefined,
};

function extractTaskStats(result: unknown): TaskStats {
  if (result == null) return EMPTY_STATS;

  // Result can be a string or an array of content blocks
  let text: string;
  if (typeof result === "string") {
    text = result;
  } else if (Array.isArray(result)) {
    // Array of content blocks — join text blocks
    text = result
      .filter((b: unknown) => b && typeof b === "object" && (b as Record<string, unknown>).type === "text")
      .map((b: unknown) => (b as Record<string, unknown>).text as string)
      .join("\n");
  } else if (typeof result === "object") {
    text = JSON.stringify(result);
  } else {
    return EMPTY_STATS;
  }

  let agentId: string | undefined;
  let totalDurationMs: number | undefined;
  let totalTokens: number | undefined;
  let totalToolUseCount: number | undefined;
  let textOutput: string | undefined;

  // Extract agentId
  const agentIdMatch = text.match(/agentId:\s*([a-f0-9]+)/);
  if (agentIdMatch) {
    agentId = agentIdMatch[1];
  }

  // Extract usage stats from <usage> block
  const usageMatch = text.match(/<usage>([\s\S]*?)<\/usage>/);
  if (usageMatch) {
    const usage = usageMatch[1] ?? "";
    const tokensMatch = usage.match(/total_tokens:\s*(\d+)/);
    const toolsMatch = usage.match(/tool_uses:\s*(\d+)/);
    const durationMatch = usage.match(/duration_ms:\s*(\d+)/);

    if (tokensMatch) totalTokens = parseInt(tokensMatch[1]!, 10);
    if (toolsMatch) totalToolUseCount = parseInt(toolsMatch[1]!, 10);
    if (durationMatch) totalDurationMs = parseInt(durationMatch[1]!, 10);
  }

  // Extract text output (everything before agentId/usage block)
  const outputEnd = text.search(/\nagentId:/);
  if (outputEnd > 0) {
    textOutput = text.slice(0, outputEnd).trim() || undefined;
  } else if (!usageMatch) {
    // No usage block — the whole result is text output
    textOutput = text.trim() || undefined;
  }

  return { agentId, totalDurationMs, totalTokens, totalToolUseCount, textOutput };
}

// ============================================================================
// Component
// ============================================================================

export const TaskToolCallCard = React.memo(function TaskToolCallCard({
  toolCall,
  className = "",
}: TaskToolCallCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const hasError = Boolean(toolCall.error);

  const taskArgs = useMemo(() => extractTaskArgs(toolCall.arguments), [toolCall.arguments]);
  const taskStats = useMemo(() => extractTaskStats(toolCall.result), [toolCall.result]);

  const description = taskArgs.description || "Subagent task";
  const subagentType = taskArgs.subagent_type || "agent";
  const model = taskArgs.model || "";

  const subagentColor = getSubagentTypeColor(subagentType);
  const modelColor = model ? getModelColor(model) : null;

  // Build stats summary
  const statParts: string[] = [];
  if (taskStats.totalDurationMs != null) {
    statParts.push(formatDuration(taskStats.totalDurationMs));
  }
  if (taskStats.totalTokens != null) {
    statParts.push(`${taskStats.totalTokens.toLocaleString()} tokens`);
  }
  if (taskStats.totalToolUseCount != null) {
    statParts.push(`${taskStats.totalToolUseCount} tool${taskStats.totalToolUseCount !== 1 ? "s" : ""}`);
  }

  const hasBody = Boolean(taskStats.textOutput) || hasError;

  return (
    <div
      data-testid="task-tool-call-card"
      className={`rounded-lg overflow-hidden ${className}`}
      style={{
        backgroundColor: hasError ? "hsla(0 70% 55% / 0.15)" : "var(--bg-elevated, hsl(220 10% 14%))",
        border: `1px solid ${hasError ? "hsla(0 70% 55% / 0.25)" : "var(--border-subtle, hsla(220 10% 100% / 0.06))"}`,
      }}
    >
      {/* Header */}
      <button
        onClick={() => hasBody && setIsExpanded(!isExpanded)}
        className={`w-full flex items-center gap-2 px-3 py-2 text-left transition-opacity ${hasBody ? "hover:opacity-80 cursor-pointer" : "cursor-default"}`}
        aria-expanded={hasBody ? isExpanded : undefined}
        aria-label={`${subagentType} subagent: ${description}. ${hasBody ? `Click to ${isExpanded ? "collapse" : "expand"}.` : ""}`}
      >
        {/* Expand/Collapse chevron (only if has body) */}
        {hasBody ? (
          isExpanded ? (
            <ChevronDown size={14} className="flex-shrink-0" style={{ color: "var(--text-muted, hsl(220 10% 45%))" }} />
          ) : (
            <ChevronRight size={14} className="flex-shrink-0" style={{ color: "var(--text-muted, hsl(220 10% 45%))" }} />
          )
        ) : (
          <Bot size={14} className="flex-shrink-0" style={{ color: "var(--text-muted, hsl(220 10% 45%))" }} />
        )}

        {/* Bot icon (when expandable, show alongside chevron) */}
        {hasBody && (
          <Bot size={14} className="flex-shrink-0" style={{ color: "var(--text-muted, hsl(220 10% 45%))" }} />
        )}

        {/* Subagent type badge */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
          style={{
            backgroundColor: subagentColor.bg,
            color: subagentColor.text,
          }}
        >
          {subagentType}
        </span>

        {/* Description text */}
        <span
          className="text-xs truncate flex-1 min-w-0"
          style={{ color: hasError ? "hsl(0 70% 75%)" : "var(--text-secondary, hsl(220 10% 75%))" }}
        >
          {description}
        </span>

        {/* Model badge */}
        {model && modelColor && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
            style={{
              backgroundColor: modelColor.bg,
              color: modelColor.text,
            }}
          >
            {model}
          </span>
        )}

        {/* Error indicator */}
        {hasError && (
          <span
            className="text-[10px] font-medium px-1.5 py-0.5 rounded"
            style={{
              backgroundColor: "hsla(0 70% 50% / 0.2)",
              color: "hsl(0 70% 70%)",
            }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Stats summary (shown below header when collapsed) */}
      {statParts.length > 0 && (
        <div
          className="px-3 py-1.5"
          style={{
            borderTop: `1px solid ${hasError ? "hsla(0 70% 55% / 0.15)" : "var(--border-subtle, hsla(220 10% 100% / 0.04))"}`,
          }}
        >
          <span className="text-xs" style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}>
            {statParts.join(" \u00B7 ")}
          </span>
        </div>
      )}

      {/* Expanded body: subagent text output */}
      {isExpanded && hasBody && (
        <div
          className="px-3 pb-3 pt-2"
          style={{
            borderTop: statParts.length > 0
              ? `1px solid ${hasError ? "hsla(0 70% 55% / 0.15)" : "var(--border-subtle, hsla(220 10% 100% / 0.04))"}`
              : undefined,
          }}
        >
          {/* Error message */}
          {hasError && toolCall.error && (
            <pre
              className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-h-48"
              style={{
                backgroundColor: "hsla(0 70% 50% / 0.1)",
                color: "hsl(0 70% 75%)",
                fontFamily: "var(--font-mono)",
                wordBreak: "break-word",
                whiteSpace: "pre-wrap",
              }}
            >
              {toolCall.error}
            </pre>
          )}

          {/* Subagent text output */}
          {taskStats.textOutput && (
            <pre
              className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-h-64"
              style={{
                backgroundColor: "var(--bg-surface, hsl(220 10% 10%))",
                color: "var(--text-secondary, hsl(220 10% 80%))",
                fontFamily: "var(--font-mono)",
                wordBreak: "break-word",
                whiteSpace: "pre-wrap",
              }}
            >
              {taskStats.textOutput}
            </pre>
          )}
        </div>
      )}
    </div>
  );
});
