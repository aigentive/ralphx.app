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
import type { ToolCall } from "./tool-widgets/shared.constants";
import {
  extractDelegationMetadata,
  isDelegationStartToolCall,
} from "./delegation-tool-calls";
import { TaskToolCallDelegatedTranscript } from "./TaskToolCallDelegatedTranscript";
import {
  EMPTY_STATS,
  extractChildToolCalls,
  extractTaskArgs,
  extractTaskStats,
} from "./TaskToolCallCard.utils";
import {
  buildTaskCardTranscriptEntryFromToolCall,
  TaskCardTranscriptView,
} from "./TaskCardTranscript";
import {
  formatProviderModelEffortLabel,
} from "./provider-harness";
import {
  TaskCardKindBadge,
  TaskCardModelBadge,
  TaskCardProviderHarnessBadge,
  TaskCardStatusBadge,
  TaskCardSubagentTypeBadge,
  TaskCardSummary,
} from "./TaskCardShared";

// ============================================================================
// Types
// ============================================================================

interface TaskToolCallCardProps {
  toolCall: ToolCall;
  className?: string;
}

// ============================================================================
// Helpers
// ============================================================================


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

  const bodyText = delegation.textOutput ?? taskStats.textOutput;
  const delegatedConversationId = isDelegateCall ? delegation.delegatedConversationId ?? null : null;
  const statusBadge = isDelegateCall && delegation.status && delegation.status !== "completed"
    ? delegation.status
    : null;

  const transcriptEntry = useMemo(
    () => buildTaskCardTranscriptEntryFromToolCall({
      entryId: toolCall.id,
      bodyText,
      childToolCalls,
    }),
    [bodyText, childToolCalls, toolCall.id],
  );
  const hasTranscriptBody = transcriptEntry.blocks.length > 0;
  const hasBody = hasTranscriptBody || hasError || delegatedConversationId != null;
  const providerMetadata = {
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
  };

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
        <TaskCardKindBadge toolName={toolCall.name} />

        {/* Subagent type badge — hidden for redundant "agent" default */}
        {showSubagentTypeBadge && <TaskCardSubagentTypeBadge subagentType={subagentType} />}

        {/* Title text */}
        <span
          className="text-xs truncate flex-1 min-w-0"
          style={{ color: hasError ? "hsl(0 70% 75%)" : "var(--text-secondary, hsl(220 10% 75%))" }}
        >
          {cardTitle}
        </span>

        {isDelegateCall && (
          <TaskCardProviderHarnessBadge
            providerHarness={delegation.providerHarness}
            providerMetadata={providerMetadata}
          />
        )}

        {/* Model badge */}
        {!providerModelEffortLabel && (
          <TaskCardModelBadge label={model || null} colorKey={model || null} />
        )}

        {providerModelEffortLabel && (
          <TaskCardModelBadge
            label={providerModelEffortLabel}
            colorKey={delegation.effectiveModelId ?? delegation.logicalModel ?? providerModelEffortLabel}
            providerMetadata={providerMetadata}
          />
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
        <TaskCardStatusBadge label={statusBadge} />

        <TaskCardStatusBadge label={hasError ? "Failed" : null} tone="error" />
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
      {(
        taskStats.totalDurationMs != null ||
        taskStats.totalTokens != null ||
        taskStats.totalToolUseCount != null ||
        taskStats.estimatedUsd != null
      ) && (
        <div
          className="px-3 py-1.5"
          style={{
            borderTop: `1px solid ${hasError ? "hsla(0 70% 55% / 0.15)" : "var(--border-subtle, hsla(220 10% 100% / 0.04))"}`,
          }}
        >
          <TaskCardSummary
            metrics={{
              totalDurationMs: taskStats.totalDurationMs,
              totalTokens: taskStats.totalTokens,
              totalToolUseCount: taskStats.totalToolUseCount,
              estimatedUsd: taskStats.estimatedUsd,
            }}
          />
        </div>
      )}

      {/* Expanded body: child tool calls + subagent text output */}
      {isExpanded && hasBody && (
        <div
          className="px-3 pb-3 pt-2"
          style={{
            borderTop: (
              taskStats.totalDurationMs != null ||
              taskStats.totalTokens != null ||
              taskStats.totalToolUseCount != null ||
              taskStats.estimatedUsd != null
            )
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

          {delegatedConversationId ? (
            <TaskToolCallDelegatedTranscript
              conversationId={delegatedConversationId}
              fallbackText={bodyText}
            />
          ) : hasTranscriptBody ? (
            <div className="max-h-64 overflow-y-auto">
              <TaskCardTranscriptView entries={[transcriptEntry]} />
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
});
