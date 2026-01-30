// Tauri invoke wrappers for unified chat API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type {
  ChatConversation,
  AgentRun,
  ContextType,
} from "../types/chat-conversation";
import type { ToolCall } from "../components/Chat/ToolCallIndicator";
import type { ContentBlockItem } from "../components/Chat/MessageItem";

// ============================================================================
// Typed Invoke Helper
// ============================================================================

async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// ============================================================================
// Response Types
// ============================================================================

/**
 * Chat message response from backend - with pre-parsed toolCalls and contentBlocks
 */
export interface ChatMessageResponse {
  id: string;
  sessionId: string | null;
  projectId: string | null;
  taskId: string | null;
  role: string;
  content: string;
  metadata: string | null;
  parentMessageId: string | null;
  conversationId: string | null;
  /** Pre-parsed tool calls array (parsed from JSON at API layer) */
  toolCalls: ToolCall[] | null;
  /** Pre-parsed content blocks array (parsed from JSON at API layer) */
  contentBlocks: ContentBlockItem[] | null;
  createdAt: string;
}

// ============================================================================
// Parsing Utilities
// ============================================================================

/**
 * Parse content blocks from raw JSON data
 * @param raw The raw data from backend (could be string, array, or null)
 * @returns Parsed content blocks array
 */
export function parseContentBlocks(raw: unknown): ContentBlockItem[] {
  if (!raw) return [];

  // If it's already an array, use it directly
  const data = typeof raw === "string" ? safeJsonParse(raw) : raw;
  if (!Array.isArray(data)) return [];

  return data as ContentBlockItem[];
}

/**
 * Parse tool calls from raw JSON data
 * @param raw The raw data from backend (could be string, array, or null)
 * @returns Parsed tool calls array
 */
export function parseToolCalls(raw: unknown): ToolCall[] {
  if (!raw) return [];

  // If it's already an array, use it directly
  const data = typeof raw === "string" ? safeJsonParse(raw) : raw;
  if (!Array.isArray(data)) return [];

  return data.map((tc, idx) => ({
    id: tc.id ?? `tool-${idx}`,
    name: tc.name ?? "unknown",
    arguments: tc.arguments ?? {},
    result: tc.result,
    error: tc.error,
  }));
}

/**
 * Safely parse JSON, returning null on failure
 */
function safeJsonParse(str: string): unknown {
  try {
    return JSON.parse(str);
  } catch {
    return null;
  }
}

/**
 * Queued message response from backend
 */
export interface QueuedMessageResponse {
  id: string;
  content: string;
  createdAt: string;
  isEditing: boolean;
}

// ============================================================================
// Response Schemas (snake_case from Rust backend)
// ============================================================================

// Response schemas for backend (snake_case - Rust default serialization)
const ChatConversationResponseSchema = z.object({
  id: z.string(),
  context_type: z.string(),
  context_id: z.string(),
  claude_session_id: z.string().nullable(),
  title: z.string().nullable(),
  message_count: z.number(),
  last_message_at: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

const AgentRunResponseSchema = z.object({
  id: z.string(),
  conversation_id: z.string(),
  status: z.string(),
  started_at: z.string(),
  completed_at: z.string().nullable(),
  error_message: z.string().nullable(),
});

type RawConversation = z.infer<typeof ChatConversationResponseSchema>;
type RawAgentRun = z.infer<typeof AgentRunResponseSchema>;

function transformConversation(raw: RawConversation): ChatConversation {
  return {
    id: raw.id,
    contextType: raw.context_type as ContextType,
    contextId: raw.context_id,
    claudeSessionId: raw.claude_session_id,
    title: raw.title,
    messageCount: raw.message_count,
    lastMessageAt: raw.last_message_at,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function transformAgentRun(raw: RawAgentRun): AgentRun {
  return {
    id: raw.id,
    conversationId: raw.conversation_id,
    status: raw.status as AgentRun["status"],
    startedAt: raw.started_at,
    completedAt: raw.completed_at,
    errorMessage: raw.error_message,
  };
}

// Schema for AgentMessageResponse from unified_chat_commands (snake_case)
const AgentMessageSchema = z.object({
  id: z.string(),
  role: z.string(),
  content: z.string(),
  tool_calls: z.any().nullable(),
  content_blocks: z.any().nullable(),
  created_at: z.string(),
});

type RawAgentMessage = z.infer<typeof AgentMessageSchema>;

function transformAgentMessage(raw: RawAgentMessage): ChatMessageResponse {
  return {
    id: raw.id,
    sessionId: null,
    projectId: null,
    taskId: null,
    role: raw.role,
    content: raw.content,
    metadata: null,
    parentMessageId: null,
    conversationId: null,
    // Parse at API layer to avoid redundant parsing in components
    toolCalls: parseToolCalls(raw.tool_calls),
    contentBlocks: parseContentBlocks(raw.content_blocks),
    createdAt: raw.created_at,
  };
}

/**
 * List all conversations for a given context
 * @param contextType The context type
 * @param contextId The context ID
 * @returns Array of conversations
 */
export async function listConversations(
  contextType: ContextType,
  contextId: string
): Promise<ChatConversation[]> {
  const raw = await typedInvoke(
    "list_agent_conversations",
    { contextType, contextId },
    z.array(ChatConversationResponseSchema)
  );
  return raw.map(transformConversation);
}

/**
 * Get a conversation with its messages
 * @param conversationId The conversation ID
 * @returns The conversation with messages
 */
export async function getConversation(
  conversationId: string
): Promise<{ conversation: ChatConversation; messages: ChatMessageResponse[] }> {
  const raw = await typedInvoke(
    "get_agent_conversation",
    { conversationId },
    z.object({
      conversation: ChatConversationResponseSchema,
      messages: z.array(AgentMessageSchema),
    })
  );

  return {
    conversation: transformConversation(raw.conversation),
    messages: raw.messages.map(transformAgentMessage),
  };
}

/**
 * Create a new conversation
 * @param contextType The context type
 * @param contextId The context ID
 * @returns The created conversation
 */
export async function createConversation(
  contextType: ContextType,
  contextId: string
): Promise<ChatConversation> {
  const raw = await typedInvoke(
    "create_agent_conversation",
    {
      input: {
        contextType,
        contextId,
      },
    },
    ChatConversationResponseSchema
  );
  return transformConversation(raw);
}

/**
 * Get the current agent run status for a conversation
 * @param conversationId The conversation ID
 * @returns The agent run if one is active, null otherwise
 */
export async function getAgentRunStatus(
  conversationId: string
): Promise<AgentRun | null> {
  const raw = await typedInvoke(
    "get_agent_run_status_unified",
    { conversationId },
    AgentRunResponseSchema.nullable()
  );
  return raw ? transformAgentRun(raw) : null;
}

// ============================================================================
// Namespace Export for Alternative Usage Pattern
// ============================================================================

const QueuedMessageResponseSchema = z.object({
  id: z.string(),
  content: z.string(),
  created_at: z.string(),
  is_editing: z.boolean(),
});

type RawQueuedMessage = z.infer<typeof QueuedMessageResponseSchema>;

function transformQueuedMessage(raw: RawQueuedMessage): QueuedMessageResponse {
  return {
    id: raw.id,
    content: raw.content,
    createdAt: raw.created_at,
    isEditing: raw.is_editing,
  };
}

// ============================================================================
// Namespace Export for Alternative Usage Pattern
// ============================================================================

/**
 * Chat API as a namespace object (alternative to individual imports)
 */
export const chatApi = {
  // Conversation management
  listConversations,
  getConversation,
  createConversation,
  getAgentRunStatus,
  // Message sending & queue
  sendAgentMessage,
  queueAgentMessage,
  getQueuedAgentMessages,
  deleteQueuedAgentMessage,
  // Agent lifecycle
  isChatServiceAvailable,
  stopAgent,
  isAgentRunning,
} as const;

// ============================================================================
// Unified Agent API Functions (Phase 5-6 Consolidation)
// ============================================================================

/**
 * Response from unified send_agent_message command
 */
export interface SendAgentMessageResult {
  conversationId: string;
  agentRunId: string;
  isNewConversation: boolean;
}

const SendAgentMessageResponseSchema = z.object({
  conversation_id: z.string(),
  agent_run_id: z.string(),
  is_new_conversation: z.boolean(),
});

type RawSendAgentMessageResponse = z.infer<typeof SendAgentMessageResponseSchema>;

function transformSendAgentMessageResponse(raw: RawSendAgentMessageResponse): SendAgentMessageResult {
  return {
    conversationId: raw.conversation_id,
    agentRunId: raw.agent_run_id,
    isNewConversation: raw.is_new_conversation,
  };
}

/**
 * Send a message using the unified agent API
 * Returns immediately with conversation_id and agent_run_id.
 * Processing happens in background with events emitted via Tauri.
 *
 * @param contextType The context type (ideation, task, project, task_execution)
 * @param contextId The context ID
 * @param content The message content
 */
export async function sendAgentMessage(
  contextType: ContextType,
  contextId: string,
  content: string
): Promise<SendAgentMessageResult> {
  const raw = await typedInvoke(
    "send_agent_message",
    {
      input: {
        contextType,
        contextId,
        content,
      },
    },
    SendAgentMessageResponseSchema
  );
  return transformSendAgentMessageResponse(raw);
}

/**
 * Queue a message to be sent when the current agent run completes
 *
 * @param contextType The context type
 * @param contextId The context ID
 * @param content The message content
 * @param clientId Optional client-provided ID (allows frontend/backend to use same ID)
 */
export async function queueAgentMessage(
  contextType: ContextType,
  contextId: string,
  content: string,
  clientId?: string
): Promise<QueuedMessageResponse> {
  const raw = await typedInvoke(
    "queue_agent_message",
    {
      input: {
        contextType,
        contextId,
        content,
        ...(clientId !== undefined && { clientId }),
      },
    },
    QueuedMessageResponseSchema
  );
  return transformQueuedMessage(raw);
}

/**
 * Get all queued messages for a context
 *
 * @param contextType The context type
 * @param contextId The context ID
 */
export async function getQueuedAgentMessages(
  contextType: ContextType,
  contextId: string
): Promise<QueuedMessageResponse[]> {
  const raw = await typedInvoke(
    "get_queued_agent_messages",
    { contextType, contextId },
    z.array(QueuedMessageResponseSchema)
  );
  return raw.map(transformQueuedMessage);
}

/**
 * Delete a queued message before it's sent
 *
 * @param contextType The context type
 * @param contextId The context ID
 * @param messageId The message ID to delete
 */
export async function deleteQueuedAgentMessage(
  contextType: ContextType,
  contextId: string,
  messageId: string
): Promise<boolean> {
  return typedInvoke(
    "delete_queued_agent_message",
    { contextType, contextId, messageId },
    z.boolean()
  );
}

/**
 * Check if the chat service is available (Claude CLI installed)
 */
export async function isChatServiceAvailable(): Promise<boolean> {
  return typedInvoke(
    "is_chat_service_available",
    {},
    z.boolean()
  );
}

/**
 * Stop a running agent for a context
 * Sends SIGTERM to the running agent process.
 *
 * @param contextType The context type (ideation, task, project, task_execution)
 * @param contextId The context ID
 * @returns True if an agent was stopped, false if no agent was running
 */
export async function stopAgent(
  contextType: ContextType,
  contextId: string
): Promise<boolean> {
  return typedInvoke(
    "stop_agent",
    { contextType, contextId },
    z.boolean()
  );
}

/**
 * Check if an agent is currently running for a context
 *
 * @param contextType The context type
 * @param contextId The context ID
 */
export async function isAgentRunning(
  contextType: ContextType,
  contextId: string
): Promise<boolean> {
  return typedInvoke(
    "is_agent_running",
    { contextType, contextId },
    z.boolean()
  );
}
