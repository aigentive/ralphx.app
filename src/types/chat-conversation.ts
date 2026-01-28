// Context-aware chat conversation types and Zod schemas
// Types for ChatConversation, AgentRun, and related structures

import { z } from "zod";

// ============================================================================
// Context Type
// ============================================================================

/**
 * Context type for chat conversations
 */
export const CONTEXT_TYPE_VALUES = [
  "ideation",
  "task",
  "project",
  "task_execution",
  "review",
] as const;

export const ContextTypeSchema = z.enum(CONTEXT_TYPE_VALUES);
export type ContextType = z.infer<typeof ContextTypeSchema>;

// ============================================================================
// Agent Run Status
// ============================================================================

/**
 * Status values for agent runs
 */
export const AGENT_RUN_STATUS_VALUES = [
  "running",
  "completed",
  "failed",
  "cancelled",
] as const;

export const AgentRunStatusSchema = z.enum(AGENT_RUN_STATUS_VALUES);
export type AgentRunStatus = z.infer<typeof AgentRunStatusSchema>;

// ============================================================================
// Chat Conversation
// ============================================================================

/**
 * Chat conversation schema matching Rust backend serialization
 */
export const ChatConversationSchema = z.object({
  id: z.string().min(1),
  contextType: ContextTypeSchema,
  contextId: z.string().min(1),
  claudeSessionId: z.string().nullable(),
  title: z.string().nullable(),
  messageCount: z.number().int().min(0),
  lastMessageAt: z.string().datetime().nullable(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type ChatConversation = z.infer<typeof ChatConversationSchema>;

// ============================================================================
// Agent Run
// ============================================================================

/**
 * Agent run schema matching Rust backend serialization
 */
export const AgentRunSchema = z.object({
  id: z.string().min(1),
  conversationId: z.string().min(1),
  status: AgentRunStatusSchema,
  startedAt: z.string().datetime(),
  completedAt: z.string().datetime().nullable(),
  errorMessage: z.string().nullable(),
});

export type AgentRun = z.infer<typeof AgentRunSchema>;

// ============================================================================
// Tool Call (for display in chat UI)
// ============================================================================

/**
 * Tool call schema (parsed from JSON in message.toolCalls)
 */
export const ToolCallSchema = z.object({
  name: z.string().min(1),
  arguments: z.unknown(),
  result: z.unknown().nullable(),
  error: z.string().nullable(),
});

export type ToolCall = z.infer<typeof ToolCallSchema>;

// ============================================================================
// Queued Message (frontend-only state)
// ============================================================================

/**
 * Queued message schema (not persisted, frontend state only)
 */
export const QueuedMessageSchema = z.object({
  id: z.string().min(1),
  content: z.string().min(1),
  createdAt: z.string().datetime(),
  isEditing: z.boolean(),
});

export type QueuedMessage = z.infer<typeof QueuedMessageSchema>;

// ============================================================================
// Input Schemas (for API calls)
// ============================================================================

/**
 * Input for creating a new conversation
 */
export const CreateConversationInputSchema = z.object({
  contextType: ContextTypeSchema,
  contextId: z.string().min(1, "Context ID is required"),
  title: z.string().optional(),
});

export type CreateConversationInput = z.infer<
  typeof CreateConversationInputSchema
>;

/**
 * Input for sending a context-aware message
 */
export const SendContextMessageInputSchema = z.object({
  contextType: ContextTypeSchema,
  contextId: z.string().min(1, "Context ID is required"),
  content: z.string().min(1, "Message content is required"),
  conversationId: z.string().optional(),
});

export type SendContextMessageInput = z.infer<
  typeof SendContextMessageInputSchema
>;

// ============================================================================
// List Schemas
// ============================================================================

export const ChatConversationListSchema = z.array(ChatConversationSchema);
export type ChatConversationList = z.infer<typeof ChatConversationListSchema>;

export const AgentRunListSchema = z.array(AgentRunSchema);
export type AgentRunList = z.infer<typeof AgentRunListSchema>;

export const ToolCallListSchema = z.array(ToolCallSchema);
export type ToolCallList = z.infer<typeof ToolCallListSchema>;

// ============================================================================
// SendContextMessage Response
// ============================================================================

/**
 * Tool call in the response from send_context_message
 */
export const ToolCallResponseSchema = z.object({
  id: z.string().nullable(),
  name: z.string().min(1),
  arguments: z.unknown(),
  result: z.unknown().nullable(),
});

export type ToolCallResponse = z.infer<typeof ToolCallResponseSchema>;

/**
 * Response from send_context_message Tauri command
 */
export const SendContextMessageResponseSchema = z.object({
  response_text: z.string(),
  tool_calls: z.array(ToolCallResponseSchema),
  claude_session_id: z.string().nullable(),
  conversation_id: z.string().nullable(),
});

export type SendContextMessageResponse = z.infer<
  typeof SendContextMessageResponseSchema
>;
