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

import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, ChevronRight, Loader2, Bot } from "lucide-react";
import type { StreamingTask } from "@/types/streaming-task";
import { ToolCallIndicator } from "./ToolCallIndicator";

// ============================================================================
// Constants
// ============================================================================

/** Maximum height for the tool list content area */
const MAX_CONTENT_HEIGHT = 200;

// ============================================================================
// Types
// ============================================================================

interface TaskSubagentCardProps {
  task: StreamingTask;
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
  const [isExpanded, setIsExpanded] = useState(true);
  const contentRef = useRef<HTMLDivElement>(null);

  const elapsed = useElapsedTimer(task.startedAt, isRunning);

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

  // Auto-scroll content when new tool calls arrive
  useEffect(() => {
    if (contentRef.current && isRunning) {
      contentRef.current.scrollTop = contentRef.current.scrollHeight;
    }
  }, [task.childToolCalls.length, isRunning]);

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
                  className="space-y-1 overflow-y-auto"
                  style={{ maxHeight: `${MAX_CONTENT_HEIGHT}px` }}
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
