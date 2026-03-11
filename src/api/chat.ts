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
  /** Sender name for team mode messages (teammate name or "lead") */
  sender: string | null;
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

  return data.map((block) => {
    const item: ContentBlockItem = {
      type: block.type,
      text: block.text,
      id: block.id,
      name: block.name,
      arguments: block.arguments,
      result: block.result,
    };
    // Transform diff_context (snake_case) to diffContext (camelCase) for tool_use blocks
    if (block.type === "tool_use" && block.diff_context) {
      item.diffContext = {
        oldContent: block.diff_context.old_content ?? undefined,
        filePath: block.diff_context.file_path,
      };
    }
    return item;
  });
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

  return data.map((tc, idx) => {
    const toolCall: ToolCall = {
      id: tc.id ?? `tool-${idx}`,
      name: tc.name ?? "unknown",
      arguments: tc.arguments ?? {},
      result: tc.result,
      error: tc.error,
    };
    if (tc.diff_context) {
      toolCall.diffContext = {
        oldContent: tc.diff_context.old_content ?? undefined,
        filePath: tc.diff_context.file_path,
      };
    }
    return toolCall;
  });
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

/**
 * A streaming task in the active state HTTP response from GET /api/conversations/:id/active-state.
 * Mirrors the Rust ActiveStreamingTask struct (snake_case — no rename_all on the Rust struct).
 */
export interface ActiveStreamingTaskResponse {
  tool_use_id: string;
  description?: string;
  subagent_type?: string;
  model?: string;
  status: string;
  teammate_name?: string;
  /** Total tokens used (from TaskCompleted stats) */
  total_tokens?: number;
  /** Total tool uses count (from TaskCompleted stats) */
  total_tool_uses?: number;
  /** Duration in milliseconds (from TaskCompleted stats) */
  duration_ms?: number;
}

/**
 * Response from GET /api/conversations/:id/active-state HTTP endpoint.
 * Used to hydrate streaming UI when navigating to an active agent execution.
 */
export interface ConversationActiveStateResponse {
  is_active: boolean;
  tool_calls: unknown[];
  streaming_tasks: ActiveStreamingTaskResponse[];
  partial_text: string;
}

/**
 * Fetch the active streaming state for a conversation.
 * Called when navigating to a conversation with an active agent execution
 * to hydrate the streaming UI with missed events.
 *
 * @param conversationId - The conversation ID
 * @returns The active state response
 */
export async function getConversationActiveState(
  conversationId: string
): Promise<ConversationActiveStateResponse> {
  const res = await fetch(
    `http://localhost:3847/api/conversations/${conversationId}/active-state`
  );
  if (!res.ok) {
    throw new Error(`Failed to get conversation active state: ${res.status}`);
  }
  return res.json() as Promise<ConversationActiveStateResponse>;
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
  sender: z.string().nullable().optional(),
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
    sender: raw.sender ?? null,
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
  // Attachments
  listMessageAttachments,
  // Active state
  getConversationActiveState,
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
 * @param attachmentIds Optional array of attachment IDs to link to this message
 */
export async function sendAgentMessage(
  contextType: ContextType,
  contextId: string,
  content: string,
  attachmentIds?: string[],
  target?: string
): Promise<SendAgentMessageResult> {
  const raw = await typedInvoke(
    "send_agent_message",
    {
      input: {
        contextType,
        contextId,
        content,
        ...(attachmentIds !== undefined && attachmentIds.length > 0 && { attachmentIds }),
        ...(target !== undefined && { target }),
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
 * @param attachmentIds Optional array of attachment IDs to link to this message
 */
export async function queueAgentMessage(
  contextType: ContextType,
  contextId: string,
  content: string,
  clientId?: string,
  attachmentIds?: string[],
  target?: string
): Promise<QueuedMessageResponse> {
  const raw = await typedInvoke(
    "queue_agent_message",
    {
      input: {
        contextType,
        contextId,
        content,
        ...(clientId !== undefined && { clientId }),
        ...(attachmentIds !== undefined && attachmentIds.length > 0 && { attachmentIds }),
        ...(target !== undefined && { target }),
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

// ============================================================================
// Chat Attachments API
// ============================================================================

/**
 * Chat attachment response from backend
 */
export interface ChatAttachmentResponse {
  id: string;
  conversationId: string;
  messageId: string | null;
  fileName: string;
  filePath: string;
  mimeType: string | null;
  fileSize: number;
  createdAt: string;
}

const ChatAttachmentResponseSchema = z.object({
  id: z.string(),
  conversationId: z.string(),
  messageId: z.string().nullable(),
  fileName: z.string(),
  filePath: z.string(),
  mimeType: z.string().nullable(),
  fileSize: z.number(),
  createdAt: z.string(),
});

/**
 * List all attachments for a specific message
 *
 * @param messageId The message ID
 * @returns Array of attachments
 */
export async function listMessageAttachments(
  messageId: string
): Promise<ChatAttachmentResponse[]> {
  return typedInvoke(
    "list_message_attachments",
    { messageId },
    z.array(ChatAttachmentResponseSchema)
  );
}
