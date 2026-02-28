/**
 * StreamingTask - Represents a subagent Task tool call during streaming.
 *
 * Links child tool calls (via parentToolUseId) to a parent Task invocation.
 * Used to group subagent work in the streaming chat UI.
 *
 * StreamingContentBlock - Represents a discrete chunk of content during streaming.
 * Used to preserve the natural interleaving of text and tool calls — when the
 * assistant writes text → calls a tool → writes more text, each segment renders
 * as a separate content block in the correct order.
 */

import type { ToolCall } from "@/components/Chat/ToolCallIndicator";

export type StreamingTaskStatus = "running" | "completed" | "failed";

/**
 * StreamingContentBlock - Discriminated union representing chunks of streaming content.
 * Allows text and tool calls to be interleaved in the order they arrive from the agent.
 *
 * The `task` variant is a position marker only — it records WHERE in the stream a Task
 * tool call appeared. Actual task metadata is read from `streamingTasks` Map via
 * toolUseId lookup. This preserves all existing StreamingTask behavior (status updates,
 * child tool calls) while rendering the card at its chronological position.
 */
export type StreamingContentBlock =
  | { type: "text"; text: string; seq?: number }
  | { type: "tool_use"; toolCall: ToolCall; seq?: number }
  | { type: "task"; toolUseId: string };

export interface StreamingTask {
  /** The Task tool_use.id — links child tool calls via parentToolUseId */
  toolUseId: string;
  /** Tool name that triggered this: "Task" or "Agent" */
  toolName: string;
  /** From Task input.description */
  description: string;
  /** Subagent type: "Explore", "Plan", "Bash", etc. */
  subagentType: string;
  /** Model used: "sonnet", "opus", "haiku" */
  model: string;
  /** Current status */
  status: StreamingTaskStatus;
  /** Timestamp when the task started (Date.now()) */
  startedAt: number;
  /** Timestamp when the task completed */
  completedAt?: number;
  /** Total duration in milliseconds (from task result) */
  totalDurationMs?: number;
  /** Total tokens used (from task result) */
  totalTokens?: number;
  /** Total tool use count (from task result) */
  totalToolUseCount?: number;
  /** Agent ID (from task result) */
  agentId?: string;
  /** Tool calls made by this subagent (matched by parentToolUseId) */
  childToolCalls: ToolCall[];
  /** Monotonically increasing sequence number for cross-event-type ordering */
  seq?: number;
}
