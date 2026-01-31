/**
 * Type definitions for ActivityView components
 */

import type { AgentMessageEvent } from "@/types/events";

// ============================================================================
// Filter Types
// ============================================================================

export type MessageTypeFilter = "all" | "thinking" | "tool_call" | "tool_result" | "text" | "error";

/** View mode: real-time (Zustand) vs historical (database) */
export type ViewMode = "realtime" | "historical";

export interface ExpandedState {
  [key: string]: boolean;
}

export interface CopiedState {
  [key: string]: boolean;
}

// ============================================================================
// Message Types
// ============================================================================

/**
 * Unified message type that can represent both real-time and historical events
 */
export interface UnifiedActivityMessage {
  id: string;
  type: AgentMessageEvent["type"];
  content: string;
  timestamp: number;
  metadata?: Record<string, unknown> | undefined;
  taskId?: string | undefined;
  sessionId?: string | undefined;
  internalStatus?: string | null | undefined;
  role?: string | undefined;
}

// ============================================================================
// Constants
// ============================================================================

export const MESSAGE_TYPES: { key: MessageTypeFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "thinking", label: "Thinking" },
  { key: "tool_call", label: "Tool Calls" },
  { key: "tool_result", label: "Results" },
  { key: "text", label: "Text" },
  { key: "error", label: "Errors" },
];

// Status options for filtering (aligned with internal status values)
export const STATUS_OPTIONS: { value: string; label: string }[] = [
  { value: "Ready", label: "Ready" },
  { value: "Queued", label: "Queued" },
  { value: "WorkerActive", label: "Worker Active" },
  { value: "WorkerDone", label: "Worker Done" },
  { value: "Reviewing", label: "Reviewing" },
  { value: "Approved", label: "Approved" },
  { value: "FixingRejection", label: "Fixing Rejection" },
  { value: "Escalated", label: "Escalated" },
  { value: "Done", label: "Done" },
];

// Role options for filtering
export type RoleFilterValue = "agent" | "system" | "user";
export const ROLE_OPTIONS: { value: RoleFilterValue; label: string }[] = [
  { value: "agent", label: "Agent" },
  { value: "system", label: "System" },
  { value: "user", label: "User" },
];
