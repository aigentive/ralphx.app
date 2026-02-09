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

import React, { useState, useMemo, useRef, useEffect } from "react";
import { ChevronDown, ChevronRight, Loader2, Bot } from "lucide-react";
import type { StreamingTask } from "@/types/streaming-task";
import type { ToolCall } from "./ToolCallIndicator";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { DiffToolCallView } from "./DiffToolCallView";

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
// Tool Summary Helpers (reused from StreamingToolIndicator patterns)
// ============================================================================

function getToolVerb(name: string): string {
  const n = name.toLowerCase();
  switch (n) {
    case "bash": return "Running";
    case "read": return "Reading";
    case "write": return "Writing";
    case "edit": return "Editing";
    case "glob": return "Finding";
    case "grep": return "Searching";
    default: return "Calling";
  }
}

function getToolPrimary(toolCall: ToolCall): string {
  const { name, arguments: args } = toolCall;
  const n = name.toLowerCase();
  const typedArgs = args as Record<string, unknown> | undefined;

  switch (n) {
    case "bash": {
      const desc = typedArgs?.description as string | undefined;
      const cmd = typedArgs?.command as string | undefined;
      return desc || (cmd ? `$ ${cmd.slice(0, 50)}${(cmd.length ?? 0) > 50 ? "..." : ""}` : "Running command");
    }
    case "read":
      return (typedArgs?.file_path as string) || "Reading file";
    case "write":
      return (typedArgs?.file_path as string) || "Writing file";
    case "edit":
      return (typedArgs?.file_path as string) || "Editing file";
    case "glob":
      return (typedArgs?.pattern as string) || "Searching files";
    case "grep":
      return typedArgs?.pattern ? `"${typedArgs.pattern}"` : "Searching content";
    default:
      return name.replace(/_/g, " ");
  }
}

// ============================================================================
// Sub-components
// ============================================================================

/** Single tool call line in the numbered list */
const ToolCallLine = React.memo(function ToolCallLine({
  toolCall,
  index,
}: {
  toolCall: ToolCall;
  index: number;
}) {
  const verb = getToolVerb(toolCall.name);
  const primary = getToolPrimary(toolCall);
  const hasError = Boolean(toolCall.error);

  return (
    <div
      className="flex items-start gap-2 text-xs"
      style={{
        color: hasError ? "var(--status-error)" : "var(--text-secondary)",
      }}
    >
      <span
        className="flex-shrink-0 w-4 text-right tabular-nums"
        style={{ color: "var(--text-muted)" }}
      >
        {index + 1}.
      </span>
      <div className="flex-1 min-w-0">
        <span className="font-medium" style={{ color: "var(--text-primary)" }}>
          {verb}
        </span>{" "}
        <span
          className="font-mono break-all"
          style={{
            color: hasError ? "var(--status-error)" : "var(--text-secondary)",
          }}
        >
          {primary}
        </span>
      </div>
    </div>
  );
});

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

  // Split child tool calls: Edit/Write get rendered as DiffToolCallViews, rest as list items
  const { diffCalls, otherCalls } = useMemo(() => {
    const diff: ToolCall[] = [];
    const other: ToolCall[] = [];
    for (const tc of task.childToolCalls) {
      if (tc.name.startsWith("result:toolu")) continue;
      if (isDiffToolCall(tc.name) && tc.arguments != null) {
        diff.push(tc);
      } else {
        other.push(tc);
      }
    }
    return { diffCalls: diff, otherCalls: other };
  }, [task.childToolCalls]);

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
          {/* Running state: numbered list of tool calls + inline diffs */}
          {(isRunning || (isCompleted && (otherCalls.length > 0 || diffCalls.length > 0))) && (
            <>
              {/* Scrollable tool call list */}
              {otherCalls.length > 0 && (
                <div
                  ref={contentRef}
                  className="space-y-1.5 overflow-y-auto"
                  style={{ maxHeight: `${MAX_CONTENT_HEIGHT}px` }}
                >
                  {otherCalls.map((tc, i) => (
                    <ToolCallLine key={tc.id || i} toolCall={tc} index={i} />
                  ))}

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

              {/* Inline DiffToolCallViews for Edit/Write child calls */}
              {diffCalls.length > 0 && (
                <div className={otherCalls.length > 0 ? "mt-2 space-y-2" : "space-y-2"}>
                  {diffCalls.map((tc) => (
                    <DiffToolCallView key={tc.id} toolCall={tc} isStreaming={isRunning} className="" />
                  ))}
                </div>
              )}
            </>
          )}

          {/* Completed state: summary */}
          {isCompleted && otherCalls.length === 0 && diffCalls.length === 0 && (
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
