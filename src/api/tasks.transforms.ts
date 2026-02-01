// Transform functions for converting snake_case tasks API responses to camelCase frontend types

import { z } from "zod";
import { InjectTaskResponseSchemaRaw, StateTransitionResponseSchemaRaw } from "./tasks.schemas";
import { transformTask, type Task, type InternalStatus } from "@/types/task";

/**
 * Frontend InjectTaskResponse type (camelCase)
 */
export interface InjectTaskResponse {
  task: Task;
  target: "backlog" | "planned";
  priority: number;
  makeNextApplied: boolean;
}

/**
 * Transform InjectTaskResponseSchemaRaw to InjectTaskResponse
 */
export function transformInjectTaskResponse(
  raw: z.infer<typeof InjectTaskResponseSchemaRaw>
): InjectTaskResponse {
  return {
    task: transformTask(raw.task),
    target: raw.target,
    priority: raw.priority,
    makeNextApplied: raw.make_next_applied,
  };
}

/**
 * Frontend StateTransition type (camelCase)
 * Represents a single state transition in a task's history.
 */
export interface StateTransition {
  /** Status transitioned from (null for initial state) */
  fromStatus: InternalStatus | null;
  /** Status transitioned to */
  toStatus: InternalStatus;
  /** What triggered this transition (e.g., "user", "agent", "system") */
  trigger: string;
  /** When the transition occurred (RFC3339 format) */
  timestamp: string;
  /** Conversation ID for states that spawn conversations (executing, re_executing, reviewing) */
  conversationId?: string;
  /** Agent run ID for the specific execution within the conversation */
  agentRunId?: string;
}

/**
 * Transform StateTransitionResponseSchemaRaw to StateTransition
 */
export function transformStateTransition(
  raw: z.infer<typeof StateTransitionResponseSchemaRaw>
): StateTransition {
  return {
    fromStatus: raw.from_status as InternalStatus | null,
    toStatus: raw.to_status as InternalStatus,
    trigger: raw.trigger,
    timestamp: raw.timestamp,
    ...(raw.conversation_id !== undefined && { conversationId: raw.conversation_id }),
    ...(raw.agent_run_id !== undefined && { agentRunId: raw.agent_run_id }),
  };
}
