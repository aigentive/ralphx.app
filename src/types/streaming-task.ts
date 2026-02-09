/**
 * StreamingTask - Represents a subagent Task tool call during streaming.
 *
 * Links child tool calls (via parentToolUseId) to a parent Task invocation.
 * Used to group subagent work in the streaming chat UI.
 */

import type { ToolCall } from "@/components/Chat/ToolCallIndicator";

export type StreamingTaskStatus = "running" | "completed" | "failed";

export interface StreamingTask {
  /** The Task tool_use.id — links child tool calls via parentToolUseId */
  toolUseId: string;
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
}
