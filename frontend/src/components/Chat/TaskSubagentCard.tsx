/**
 * TaskSubagentCard - Renders a dedicated card for each running/completed Task subagent
 * during streaming.
 *
 * Features:
 * - Header: subagent type badge, description, model badge, running timer / completed duration
 * - Body (running): numbered list of child tool calls (StreamingToolIndicator-style)
 *   with Edit/Write calls rendered as inline DiffToolCallViews
 * - Body (completed): collapsed summary with duration, token count, tool use count
 * - Styling matches StreamingToolIndicator aesthetic (bg-elevated, border-subtle, orange accent)
 */

import React, { useState, useRef, useEffect, useCallback } from "react";
import { ChevronDown, ChevronRight, Loader2, Bot } from "lucide-react";
import type { StreamingTask } from "@/types/streaming-task";
import { ToolCallIndicator } from "./ToolCallIndicator";
import { formatDuration, getSubagentTypeColor, getModelColor } from "./tool-call-utils";

// ============================================================================
// Constants
// ============================================================================

/** Maximum height for the tool list content area */
const MAX_CONTENT_HEIGHT = 200;

/** Distance from bottom (px) within which we consider the user "near bottom" */
const NEAR_BOTTOM_THRESHOLD = 30;

// ============================================================================
// Types
// ============================================================================

interface TaskSubagentCardProps {
  task: StreamingTask;
}

// ============================================================================
// Running Timer Hook
// ============================================================================

function useElapsedTimer(startedAt: number, isRunning: boolean): string {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!isRunning) return;
    setNow(Date.now());
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, [isRunning]);

  if (!isRunning) return "";
  return formatDuration(now - startedAt);
}

// ============================================================================
// Sub-components
// ============================================================================

/** Completed state summary line */
function CompletedSummary({ task }: { task: StreamingTask }) {
  const parts: string[] = [];

  if (task.totalDurationMs != null) {
    parts.push(formatDuration(task.totalDurationMs));
  }
  if (task.totalTokens != null) {
    parts.push(`${task.totalTokens.toLocaleString()} tokens`);
  }
  if (task.totalToolUseCount != null) {
    parts.push(`${task.totalToolUseCount} tool${task.totalToolUseCount !== 1 ? "s" : ""}`);
  }

  if (parts.length === 0) return null;

  return (
    <span className="text-xs" style={{ color: "var(--text-muted)" }}>
      {parts.join(" \u00B7 ")}
    </span>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export const TaskSubagentCard = React.memo(function TaskSubagentCard({
  task,
}: TaskSubagentCardProps) {
  const isRunning = task.status === "running";
  const isCompleted = task.status === "completed";
  const [isExpanded, setIsExpanded] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const isNearBottomRef = useRef(true);

  const elapsed = useElapsedTimer(task.startedAt, isRunning);

  // Track scroll position to determine if user is near bottom
  const handleScroll = useCallback(() => {
    const el = contentRef.current;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    isNearBottomRef.current = distanceFromBottom <= NEAR_BOTTOM_THRESHOLD;
  }, []);

  // Auto-collapse when completed
  useEffect(() => {
    if (isCompleted) {
      setIsExpanded(false);
    }
  }, [isCompleted]);

  // Check if there are any displayable child tool calls (excludes result markers)
  const hasChildCalls = task.childToolCalls.some(
    (tc) => !tc.name.startsWith("result:toolu")
  );

  // Auto-scroll content when new tool calls arrive (only if user is near bottom)
  useEffect(() => {
    if (contentRef.current && isRunning && isNearBottomRef.current) {
      contentRef.current.scrollTop = contentRef.current.scrollHeight;
    }
  }, [task.childToolCalls.length, isRunning]);

  const isAgentCall = task.toolName.toLowerCase() === "agent";
  const subagentColor = getSubagentTypeColor(task.subagentType);
  const modelColor = getModelColor(task.model);

  return (
    <div
      data-testid="task-subagent-card"
      className="rounded-lg overflow-hidden mb-2"
      style={{
        backgroundColor: "var(--bg-elevated)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:opacity-80 transition-opacity border-b"
        style={{ borderColor: "var(--border-subtle)" }}
        aria-expanded={isExpanded}
        aria-label={`${task.subagentType} subagent: ${task.description}. Click to ${isExpanded ? "collapse" : "expand"}.`}
      >
        {/* Expand/Collapse chevron */}
        {isExpanded ? (
          <ChevronDown size={14} className="flex-shrink-0" style={{ color: "var(--text-muted)" }} />
        ) : (
          <ChevronRight size={14} className="flex-shrink-0" style={{ color: "var(--text-muted)" }} />
        )}

        {/* Running indicator or bot icon */}
        {isRunning ? (
          <Loader2
            size={14}
            className="animate-spin flex-shrink-0"
            style={{ color: "var(--accent-primary)" }}
          />
        ) : (
          <Bot
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--text-muted)" }}
          />
        )}

        {/* Agent vs Task label */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
          style={{
            backgroundColor: isAgentCall ? "hsla(14, 100%, 60%, 0.12)" : "hsla(220, 10%, 50%, 0.12)",
            color: isAgentCall ? "hsl(14, 100%, 65%)" : "hsl(220, 10%, 60%)",
          }}
        >
          {isAgentCall ? "Agent" : "Task"}
        </span>

        {/* Subagent type badge */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 font-medium"
          style={{
            backgroundColor: subagentColor.bg,
            color: subagentColor.text,
          }}
        >
          {task.subagentType}
        </span>

        {/* Description text */}
        <span
          className="text-xs truncate flex-1 min-w-0"
          style={{ color: "var(--text-secondary)" }}
        >
          {task.description}
        </span>

        {/* Model badge */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
          style={{
            backgroundColor: modelColor.bg,
            color: modelColor.text,
          }}
        >
          {task.model}
        </span>

        {/* Timer / Duration */}
        <span
          className="text-[10px] tabular-nums flex-shrink-0"
          style={{ color: "var(--text-muted)" }}
        >
          {isRunning ? elapsed : (task.totalDurationMs != null ? formatDuration(task.totalDurationMs) : "")}
        </span>
      </button>

      {/* Body */}
      {isExpanded && (
        <div className="px-3 py-2">
          {/* Child tool calls rendered as compact ToolCallIndicators */}
          {(isRunning || (isCompleted && hasChildCalls)) && (
            <>
              {/* Scrollable tool call list */}
              {hasChildCalls && (
                <div
                  ref={contentRef}
                  onScroll={handleScroll}
                  className="space-y-1 overflow-y-auto"
                  style={{ maxHeight: `${MAX_CONTENT_HEIGHT}px`, overscrollBehavior: "contain" }}
                >
                  {/* All child calls rendered with compact ToolCallIndicator */}
                  {task.childToolCalls
                    .filter((tc) => !tc.name.startsWith("result:toolu"))
                    .map((tc) => (
                      <ToolCallIndicator key={tc.id} toolCall={tc} compact />
                    ))
                  }

                  {/* Active indicator */}
                  {isRunning && (
                    <div className="flex items-center gap-2 text-xs pt-1">
                      <span className="w-4" />
                      <div className="flex items-center gap-1">
                        <div
                          className="w-1.5 h-1.5 rounded-full animate-pulse"
                          style={{ backgroundColor: "var(--accent-primary)" }}
                        />
                        <div
                          className="w-1.5 h-1.5 rounded-full animate-pulse"
                          style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.15s" }}
                        />
                        <div
                          className="w-1.5 h-1.5 rounded-full animate-pulse"
                          style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.3s" }}
                        />
                      </div>
                    </div>
                  )}
                </div>
              )}
            </>
          )}

          {/* Completed state: summary */}
          {isCompleted && !hasChildCalls && (
            <CompletedSummary task={task} />
          )}
        </div>
      )}

      {/* Collapsed completed summary (shown in header area when collapsed) */}
      {!isExpanded && isCompleted && (
        <div className="px-3 py-1.5 border-t" style={{ borderColor: "var(--border-subtle)" }}>
          <CompletedSummary task={task} />
        </div>
      )}
    </div>
  );
});
