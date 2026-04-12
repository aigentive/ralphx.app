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
import { ToolCallIndicator } from "./ToolCallIndicator";
import type { ToolCall } from "./tool-widgets/shared.constants";
import { formatDuration, getSubagentTypeColor, getModelColor } from "./tool-call-utils";
import {
  extractDelegationMetadata,
  isDelegationStartToolCall,
} from "./delegation-tool-calls";
import {
  formatMessageAttributionTooltip,
  formatProviderHarnessLabel,
  formatProviderModelEffortLabel,
  getProviderHarnessBadgeStyle,
} from "./provider-harness";

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
  /** Agent-specific: named agent */
  name: string | undefined;
  /** Agent-specific: isolation mode (e.g. "worktree") */
  isolation: string | undefined;
  /** Agent-specific: whether to run in background */
  run_in_background: boolean | undefined;
}

interface TaskStats {
  /** True when result was parsed (even if all fields are undefined). False when no parseable result. */
  statsAvailable: boolean;
  agentId: string | undefined;
  totalDurationMs: number | undefined;
  totalTokens: number | undefined;
  totalToolUseCount: number | undefined;
  model: string | undefined;
  textOutput: string | undefined;
  estimatedUsd?: number;
}

// ============================================================================
// Helpers
// ============================================================================

const EMPTY_ARGS: TaskArgs = {
  description: undefined,
  subagent_type: undefined,
  model: undefined,
  prompt: undefined,
  name: undefined,
  isolation: undefined,
  run_in_background: undefined,
};

/** Extract Task/Agent arguments (description, subagent_type, model, and Agent-specific fields) */
function extractTaskArgs(args: unknown): TaskArgs {
  if (!args || typeof args !== "object") return EMPTY_ARGS;
  const a = args as Record<string, unknown>;
  return {
    description: typeof a.description === "string" ? a.description : undefined,
    subagent_type: typeof a.subagent_type === "string" ? a.subagent_type : undefined,
    model: typeof a.model === "string" ? a.model : undefined,
    prompt: typeof a.prompt === "string" ? a.prompt : undefined,
    name: typeof a.name === "string" ? a.name : undefined,
    isolation: typeof a.isolation === "string" ? a.isolation : undefined,
    run_in_background: typeof a.run_in_background === "boolean" ? a.run_in_background : undefined,
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
  statsAvailable: false,
  agentId: undefined,
  totalDurationMs: undefined,
  totalTokens: undefined,
  totalToolUseCount: undefined,
  model: undefined,
  textOutput: undefined,
};

/**
 * Extract child tool calls from the Task result content blocks.
 *
 * When result is an array of content blocks, tool_use blocks represent
 * tool calls made by the subagent. We pair each tool_use with its
 * subsequent tool_result (matched by tool_use_id) to build ToolCall objects.
 */
function extractChildToolCalls(result: unknown): ToolCall[] {
  if (!Array.isArray(result)) return [];

  const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];
  const toolResultMap = new Map<string, unknown>();

  for (const block of result) {
    if (!block || typeof block !== "object") continue;
    const b = block as Record<string, unknown>;

    if (b.type === "tool_use" && typeof b.name === "string" && typeof b.id === "string") {
      toolUseBlocks.push({ id: b.id, name: b.name, input: b.input });
    } else if (b.type === "tool_result" && typeof b.tool_use_id === "string") {
      toolResultMap.set(b.tool_use_id, b.content);
    }
  }

  return toolUseBlocks.map((tu) => {
    const tc: ToolCall = { id: tu.id, name: tu.name, arguments: tu.input };
    const resultContent = toolResultMap.get(tu.id);
    if (resultContent != null) {
      tc.result = resultContent;
    }
    return tc;
  });
}

/** Text-parsing fallback: extract stats and textOutput from raw result content. Used for old DB rows without a structured stats field. */
function extractTaskStatsFromResult(result: unknown): TaskStats {
  if (result == null) return EMPTY_STATS;

  // Result can be a string, array of content blocks, or JSON object
  let text: string;
  if (typeof result === "string") {
    text = result;
  } else if (Array.isArray(result)) {
    // Array of content blocks — join text blocks
    const textBlocks = result.filter(
      (b: unknown) => b && typeof b === "object" && (b as Record<string, unknown>).type === "text",
    );
    if (textBlocks.length === 0) {
      // tool_use-only array or empty — result exists but no text stats available
      return { ...EMPTY_STATS, statsAvailable: false };
    }
    text = textBlocks.map((b: unknown) => (b as Record<string, unknown>).text as string).join("\n");
  } else if (typeof result === "object") {
    // Single content block object — extract .text field if present
    const obj = result as Record<string, unknown>;
    if (typeof obj.text === "string") {
      text = obj.text;
    } else {
      return { ...EMPTY_STATS, statsAvailable: false };
    }
  } else {
    return EMPTY_STATS;
  }

  let agentId: string | undefined;
  let totalDurationMs: number | undefined;
  let totalTokens: number | undefined;
  let totalToolUseCount: number | undefined;
  let textOutput: string | undefined;

  // Extract agentId (case-insensitive — defensive, CLI currently emits lowercase)
  const agentIdMatch = text.match(/agentId:\s*([a-fA-F0-9]+)/i);
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

  // Extract text output (everything before agentId/usage block).
  // Use (?:^|\n) to handle agentId at start of text (no preceding newline)
  const agentIdPos = text.search(/(?:^|\n)agentId:/);
  if (agentIdPos >= 0) {
    // Slice up to the match position (0 if agentId is first line → empty output)
    textOutput = text.slice(0, agentIdPos).trim() || undefined;
  } else if (!usageMatch) {
    // No agentId or usage block — the whole result is text output
    textOutput = text.trim() || undefined;
  }

  return { statsAvailable: true, agentId, totalDurationMs, totalTokens, totalToolUseCount, model: undefined, textOutput };
}

/**
 * Extract stats for a Task/Agent tool call.
 *
 * Structured path (new DB rows): checks toolCall.stats first — directly uses camelCase fields
 * persisted by the backend at TaskCompleted time. agentId and textOutput still come from result.
 *
 * Text-parsing fallback (old DB rows): if toolCall.stats is absent/undefined, falls back to
 * parsing the result text for embedded <usage> tags and agentId lines.
 */
function extractTaskStats(toolCall: ToolCall): TaskStats {
  // Structured path: use stats field if present (new DB rows — camelCase from backend)
  if (toolCall.stats !== undefined) {
    // Still extract agentId + textOutput from result text (not in stats field)
    const fromResult = extractTaskStatsFromResult(toolCall.result);
    return {
      statsAvailable: true,
      agentId: fromResult.agentId,
      totalDurationMs: toolCall.stats.durationMs,
      totalTokens: toolCall.stats.totalTokens,
      totalToolUseCount: toolCall.stats.totalToolUses,
      model: toolCall.stats.model,
      textOutput: fromResult.textOutput,
    };
  }

  // Text-parsing fallback for old DB rows without structured stats
  if (import.meta.env.DEV) {
    console.debug("[extractTaskStats] text-parsing fallback for tool call:", toolCall.id);
  }
  return extractTaskStatsFromResult(toolCall.result);
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
  const delegation = useMemo(
    () => extractDelegationMetadata(toolCall.arguments, toolCall.result),
    [toolCall.arguments, toolCall.result],
  );
  const isDelegateCall = isDelegationStartToolCall(toolCall.name);
  const taskStats = useMemo(
    () => isDelegateCall
      ? {
          ...EMPTY_STATS,
          statsAvailable: true,
          totalDurationMs: delegation.durationMs,
          totalTokens: delegation.totalTokens,
          model: delegation.effectiveModelId ?? delegation.logicalModel,
          textOutput: delegation.textOutput,
          estimatedUsd: delegation.estimatedUsd,
        }
      : extractTaskStats(toolCall),
    [delegation, isDelegateCall, toolCall],
  );
  const childToolCalls = useMemo(() => extractChildToolCalls(toolCall.result), [toolCall.result]);

  const isAgentCall = !isDelegateCall && toolCall.name.toLowerCase() === "agent";
  const subagentType = isDelegateCall ? "delegated" : taskArgs.subagent_type || "agent";
  const model = isDelegateCall
    ? delegation.effectiveModelId ?? delegation.logicalModel ?? ""
    : taskArgs.model || "";
  const providerHarnessLabel = isDelegateCall
    ? formatProviderHarnessLabel(delegation.providerHarness)
    : null;
  const providerHarnessStyle = getProviderHarnessBadgeStyle(delegation.providerHarness);
  const providerModelEffortLabel = isDelegateCall
      ? formatProviderModelEffortLabel({
        providerHarness: delegation.providerHarness,
        providerSessionId: delegation.providerSessionId,
        upstreamProvider: delegation.upstreamProvider,
        providerProfile: delegation.providerProfile,
        logicalModel: delegation.logicalModel,
        effectiveModelId: delegation.effectiveModelId,
        logicalEffort: delegation.logicalEffort,
        effectiveEffort: delegation.effectiveEffort,
        inputTokens: delegation.inputTokens,
        outputTokens: delegation.outputTokens,
        cacheCreationTokens: delegation.cacheCreationTokens,
        cacheReadTokens: delegation.cacheReadTokens,
        estimatedUsd: delegation.estimatedUsd,
      })
    : null;
  const providerTooltip = isDelegateCall
      ? formatMessageAttributionTooltip({
        providerHarness: delegation.providerHarness,
        providerSessionId: delegation.providerSessionId,
        upstreamProvider: delegation.upstreamProvider,
        providerProfile: delegation.providerProfile,
        logicalModel: delegation.logicalModel,
        effectiveModelId: delegation.effectiveModelId,
        logicalEffort: delegation.logicalEffort,
        effectiveEffort: delegation.effectiveEffort,
        inputTokens: delegation.inputTokens,
        outputTokens: delegation.outputTokens,
        cacheCreationTokens: delegation.cacheCreationTokens,
        cacheReadTokens: delegation.cacheReadTokens,
        estimatedUsd: delegation.estimatedUsd,
      })
    : null;

  // Card title: Agent with name → show name (team mode); otherwise description or fallback
  const cardTitle = isDelegateCall
    ? delegation.agentName || delegation.title || "Delegated specialist"
    : isAgentCall && taskArgs.name
      ? taskArgs.name
      : taskArgs.description || (isAgentCall ? "Agent task" : "Subagent task");

  // Hide subagent_type badge when it's the redundant default "agent" value
  const showSubagentTypeBadge = !isDelegateCall && Boolean(subagentType && subagentType !== "agent");

  // Subtitle: shown below title for named agents (description or prompt preview)
  const subtitle = isDelegateCall
    ? delegation.prompt
      ? `${delegation.prompt.slice(0, 100)}${delegation.prompt.length > 100 ? "..." : ""}`
      : null
    : isAgentCall && taskArgs.name && taskArgs.description
      ? taskArgs.description
      : isAgentCall && taskArgs.name && !taskArgs.description && taskArgs.prompt
        ? taskArgs.prompt.slice(0, 100) + "..."
        : null;

  const subagentColor = getSubagentTypeColor(subagentType);
  const modelColor = model ? getModelColor(model) : null;
  const bodyText = delegation.textOutput ?? taskStats.textOutput;
  const statusBadge = isDelegateCall && delegation.status && delegation.status !== "completed"
    ? delegation.status
    : null;

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
  if (taskStats.estimatedUsd != null) {
    statParts.push(`$${taskStats.estimatedUsd.toFixed(2)}`);
  }

  const hasBody = Boolean(bodyText) || hasError || childToolCalls.length > 0;

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
        aria-label={`${isDelegateCall ? "delegated" : subagentType} task: ${cardTitle}. ${hasBody ? `Click to ${isExpanded ? "collapse" : "expand"}.` : ""}`}
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

        {/* Agent vs Task label */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
          style={{
              backgroundColor: isDelegateCall
                ? "hsla(150, 55%, 45%, 0.12)"
                : isAgentCall
                  ? "hsla(14, 100%, 60%, 0.12)"
                  : "hsla(220, 10%, 50%, 0.12)",
              color: isDelegateCall
                ? "hsl(150, 55%, 63%)"
                : isAgentCall
                  ? "hsl(14, 100%, 65%)"
                  : "hsl(220, 10%, 60%)",
            }}
          >
          {isDelegateCall ? "Delegate" : isAgentCall ? "Agent" : "Task"}
          </span>

        {/* Subagent type badge — hidden for redundant "agent" default */}
        {showSubagentTypeBadge && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
            style={{
              backgroundColor: subagentColor.bg,
              color: subagentColor.text,
            }}
          >
            {subagentType}
          </span>
        )}

        {/* Title text */}
        <span
          className="text-xs truncate flex-1 min-w-0"
          style={{ color: hasError ? "hsl(0 70% 75%)" : "var(--text-secondary, hsl(220 10% 75%))" }}
        >
          {cardTitle}
        </span>

        {providerHarnessLabel && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
            style={providerHarnessStyle}
            title={providerTooltip ?? undefined}
          >
            {providerHarnessLabel}
          </span>
        )}

        {/* Model badge */}
        {model && modelColor && !providerModelEffortLabel && (
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

        {providerModelEffortLabel && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
            style={{
              backgroundColor: modelColor?.bg ?? "hsla(220, 10%, 50%, 0.15)",
              color: modelColor?.text ?? "hsl(220, 10%, 65%)",
            }}
            title={providerTooltip ?? undefined}
          >
            {providerModelEffortLabel}
          </span>
        )}

        {/* Agent-specific: isolation badge */}
        {isAgentCall && taskArgs.isolation && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
            style={{
              backgroundColor: "hsla(200, 70%, 50%, 0.12)",
              color: "hsl(200, 70%, 65%)",
            }}
          >
            {taskArgs.isolation}
          </span>
        )}

        {/* Agent-specific: background indicator */}
        {isAgentCall && taskArgs.run_in_background && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
            style={{
              backgroundColor: "hsla(280, 50%, 50%, 0.12)",
              color: "hsl(280, 50%, 70%)",
            }}
          >
            bg
          </span>
        )}

        {/* Error indicator */}
        {statusBadge && (
          <span
            className="text-[10px] font-medium px-1.5 py-0.5 rounded"
            style={{
              backgroundColor: "hsla(38 90% 50% / 0.15)",
              color: "hsl(38 90% 60%)",
            }}
          >
            {statusBadge}
          </span>
        )}

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

      {/* Subtitle: description or prompt preview for named agents */}
      {subtitle && (
        <div
          className="px-3 pb-1.5"
          style={{ paddingLeft: "2.25rem" /* align under title, past chevron+bot icons */ }}
        >
          <span
            className="text-[11px] truncate block"
            style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
          >
            {subtitle}
          </span>
        </div>
      )}

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

      {/* Expanded body: child tool calls + subagent text output */}
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

          {/* Child tool calls — rendered as compact ToolCallIndicators */}
          {childToolCalls.length > 0 && (
            <div className="space-y-1 max-h-64 overflow-y-auto mb-2">
              {childToolCalls.map((tc) => (
                <ToolCallIndicator key={tc.id} toolCall={tc} compact />
              ))}
            </div>
          )}

          {/* Subagent text output */}
          {bodyText && (
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
              {bodyText}
            </pre>
          )}
        </div>
      )}
    </div>
  );
});
