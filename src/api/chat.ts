// Tauri invoke wrappers for chat messages with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { ChatContext } from "../types/chat";
import { SendContextMessageResponseSchema } from "../types/chat-conversation";

// ============================================================================
// Response Schemas (matching Rust backend serialization with snake_case)
// ============================================================================

const ChatMessageResponseSchema = z.object({
  id: z.string(),
  session_id: z.string().nullable(),
  project_id: z.string().nullable(),
  task_id: z.string().nullable(),
  role: z.string(),
  content: z.string(),
  metadata: z.string().nullable(),
  parent_message_id: z.string().nullable(),
  conversation_id: z.string().nullable(),
  tool_calls: z.string().nullable(),
  content_blocks: z.string().nullish(), // Optional for backwards compatibility
  created_at: z.string(),
});

// ============================================================================
// Transformed Types (camelCase for frontend usage)
// ============================================================================

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
  toolCalls: string | null;
  contentBlocks: string | null;
  createdAt: string;
}

// ============================================================================
// Input Types
// ============================================================================

/**
 * Base input for sending a chat message
 */
export interface SendMessageInput {
  role?: "user" | "orchestrator" | "system";
  content: string;
  metadata?: string;
  parentMessageId?: string;
}

/**
 * Message context - determines where the message is sent
 */
export type MessageContext =
  | { type: "session"; sessionId: string }
  | { type: "project"; projectId: string }
  | { type: "task"; taskId: string };

// ============================================================================
// Transform Functions (snake_case -> camelCase)
// ============================================================================

type RawMessage = z.infer<typeof ChatMessageResponseSchema>;

function transformMessage(raw: RawMessage): ChatMessageResponse {
  return {
    id: raw.id,
    sessionId: raw.session_id,
    projectId: raw.project_id,
    taskId: raw.task_id,
    role: raw.role,
    content: raw.content,
    metadata: raw.metadata,
    parentMessageId: raw.parent_message_id,
    conversationId: raw.conversation_id,
    toolCalls: raw.tool_calls,
    contentBlocks: raw.content_blocks ?? null,
    createdAt: raw.created_at,
  };
}

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
// Chat API Functions
// ============================================================================

/**
 * Send a chat message in the given context
 * @param context The message context (session, project, or task)
 * @param input Message content and options
 * @returns The created message
 */
export async function sendChatMessage(
  context: MessageContext,
  input: SendMessageInput
): Promise<ChatMessageResponse> {
  const baseInput = {
    role: input.role ?? "user",
    content: input.content,
    metadata: input.metadata,
    parent_message_id: input.parentMessageId,
  };

  let invokeInput: Record<string, unknown>;

  switch (context.type) {
    case "session":
      invokeInput = { ...baseInput, session_id: context.sessionId };
      break;
    case "project":
      invokeInput = { ...baseInput, project_id: context.projectId };
      break;
    case "task":
      invokeInput = { ...baseInput, task_id: context.taskId };
      break;
  }

  const raw = await typedInvoke(
    "send_chat_message",
    { input: invokeInput },
    ChatMessageResponseSchema
  );
  return transformMessage(raw);
}

/**
 * Send a message using ChatContext from the chat types
 * @param chatContext The chat context from the UI
 * @param content Message content
 * @param options Additional message options
 * @returns The created message
 */
export async function sendMessageWithContext(
  chatContext: ChatContext,
  content: string,
  options?: Omit<SendMessageInput, "content">
): Promise<ChatMessageResponse> {
  let messageContext: MessageContext;

  switch (chatContext.view) {
    case "ideation":
      if (!chatContext.ideationSessionId) {
        throw new Error("Ideation context requires sessionId");
      }
      messageContext = { type: "session", sessionId: chatContext.ideationSessionId };
      break;
    case "task_detail":
      if (!chatContext.selectedTaskId) {
        throw new Error("Task detail context requires selectedTaskId");
      }
      messageContext = { type: "task", taskId: chatContext.selectedTaskId };
      break;
    case "kanban":
      if (chatContext.selectedTaskId) {
        messageContext = { type: "task", taskId: chatContext.selectedTaskId };
      } else {
        messageContext = { type: "project", projectId: chatContext.projectId };
      }
      break;
    default:
      messageContext = { type: "project", projectId: chatContext.projectId };
  }

  return sendChatMessage(messageContext, { content, ...options });
}

/**
 * Get all messages for an ideation session
 * @param sessionId The session ID
 * @returns Array of messages
 */
export async function getSessionMessages(
  sessionId: string
): Promise<ChatMessageResponse[]> {
  const raw = await typedInvoke(
    "get_session_messages",
    { sessionId },
    z.array(ChatMessageResponseSchema)
  );
  return raw.map(transformMessage);
}

/**
 * Get recent messages for a session with a limit
 * @param sessionId The session ID
 * @param limit Maximum number of messages to return
 * @returns Array of messages
 */
export async function getRecentSessionMessages(
  sessionId: string,
  limit: number
): Promise<ChatMessageResponse[]> {
  const raw = await typedInvoke(
    "get_recent_session_messages",
    { sessionId, limit },
    z.array(ChatMessageResponseSchema)
  );
  return raw.map(transformMessage);
}

/**
 * Get all messages for a project (not in any session)
 * @param projectId The project ID
 * @returns Array of messages
 */
export async function getProjectMessages(
  projectId: string
): Promise<ChatMessageResponse[]> {
  const raw = await typedInvoke(
    "get_project_messages",
    { projectId },
    z.array(ChatMessageResponseSchema)
  );
  return raw.map(transformMessage);
}

/**
 * Get all messages for a task
 * @param taskId The task ID
 * @returns Array of messages
 */
export async function getTaskMessages(
  taskId: string
): Promise<ChatMessageResponse[]> {
  const raw = await typedInvoke(
    "get_task_messages",
    { taskId },
    z.array(ChatMessageResponseSchema)
  );
  return raw.map(transformMessage);
}

/**
 * Delete a chat message
 * @param messageId The message ID
 */
export async function deleteChatMessage(messageId: string): Promise<void> {
  await invoke("delete_chat_message", { id: messageId });
}

/**
 * Delete all messages in a session
 * @param sessionId The session ID
 */
export async function deleteSessionMessages(sessionId: string): Promise<void> {
  await invoke("delete_session_messages", { sessionId });
}

/**
 * Count messages in a session
 * @param sessionId The session ID
 * @returns Number of messages
 */
export async function countSessionMessages(sessionId: string): Promise<number> {
  return typedInvoke(
    "count_session_messages",
    { sessionId },
    z.number()
  );
}

// ============================================================================
// Context-Aware Chat API Functions
// ============================================================================

import type {
  ChatConversation,
  AgentRun,
  ContextType,
} from "../types/chat-conversation";

// Response schemas for backend snake_case
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

/**
 * Response from sendContextMessage with orchestrator result
 */
export interface SendContextMessageResult {
  responseText: string;
  toolCalls: Array<{
    id: string | null;
    name: string;
    arguments: unknown;
    result: unknown | null;
  }>;
  claudeSessionId: string | null;
  conversationId: string | null;
}

/**
 * Send a context-aware message (uses conversation and --resume)
 * This spawns Claude CLI with the appropriate agent and streams the response.
 * @param contextType The context type (ideation, task, project, task_execution)
 * @param contextId The context ID (session_id, task_id, project_id)
 * @param content The message content
 * @returns The orchestrator result with response text and tool calls
 */
export async function sendContextMessage(
  contextType: ContextType,
  contextId: string,
  content: string
): Promise<SendContextMessageResult> {
  const raw = await typedInvoke(
    "send_context_message",
    {
      input: {
        context_type: contextType,
        context_id: contextId,
        content,
      },
    },
    SendContextMessageResponseSchema
  );
  return {
    responseText: raw.response_text,
    toolCalls: raw.tool_calls.map((tc) => ({
      id: tc.id,
      name: tc.name,
      arguments: tc.arguments,
      result: tc.result,
    })),
    claudeSessionId: raw.claude_session_id,
    conversationId: raw.conversation_id,
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
    "list_conversations",
    {
      contextType,
      contextId,
    },
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
    "get_conversation",
    { conversationId },
    z.object({
      conversation: ChatConversationResponseSchema,
      messages: z.array(ChatMessageResponseSchema),
    })
  );

  return {
    conversation: transformConversation(raw.conversation),
    messages: raw.messages.map(transformMessage),
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
    "create_conversation",
    {
      input: {
        context_type: contextType,
        context_id: contextId,
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
    "get_agent_run_status",
    { conversationId },
    AgentRunResponseSchema.nullable()
  );
  return raw ? transformAgentRun(raw) : null;
}

// ============================================================================
// Namespace Export for Alternative Usage Pattern
// ============================================================================

// ============================================================================
// Task Execution Chat API Functions
// ============================================================================

/**
 * Queued message for task execution
 */
export interface QueuedMessageResponse {
  id: string;
  content: string;
  createdAt: string;
  isEditing: boolean;
}

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

/**
 * Get the active execution conversation for a task
 * @param taskId The task ID
 * @returns The active execution conversation if one exists
 */
export async function getExecutionConversation(
  taskId: string
): Promise<ChatConversation | null> {
  const raw = await typedInvoke(
    "get_execution_conversation",
    { task_id: taskId },
    ChatConversationResponseSchema.nullable()
  );
  return raw ? transformConversation(raw) : null;
}

/**
 * List all execution attempts for a task
 * @param taskId The task ID
 * @returns Array of execution conversations, ordered by created_at DESC
 */
export async function listTaskExecutions(
  taskId: string
): Promise<ChatConversation[]> {
  const raw = await typedInvoke(
    "list_task_executions",
    { task_id: taskId },
    z.array(ChatConversationResponseSchema)
  );
  return raw.map(transformConversation);
}

/**
 * Queue a message to be sent to the worker when it finishes its current response
 * @param taskId The task ID
 * @param content The message content
 * @returns The queued message
 */
export async function queueExecutionMessage(
  taskId: string,
  content: string
): Promise<QueuedMessageResponse> {
  const raw = await typedInvoke(
    "queue_execution_message",
    { task_id: taskId, content },
    QueuedMessageResponseSchema
  );
  return transformQueuedMessage(raw);
}

/**
 * Get all queued messages for a task
 * @param taskId The task ID
 * @returns Array of queued messages in FIFO order
 */
export async function getQueuedExecutionMessages(
  taskId: string
): Promise<QueuedMessageResponse[]> {
  const raw = await typedInvoke(
    "get_queued_execution_messages",
    { task_id: taskId },
    z.array(QueuedMessageResponseSchema)
  );
  return raw.map(transformQueuedMessage);
}

/**
 * Delete a queued message before it's sent
 * @param taskId The task ID
 * @param messageId The message ID to delete
 * @returns True if the message was found and deleted
 */
export async function deleteQueuedExecutionMessage(
  taskId: string,
  messageId: string
): Promise<boolean> {
  return typedInvoke(
    "delete_queued_execution_message",
    { task_id: taskId, message_id: messageId },
    z.boolean()
  );
}

// ============================================================================
// Namespace Export for Alternative Usage Pattern
// ============================================================================

/**
 * Chat API as a namespace object (alternative to individual imports)
 */
export const chatApi = {
  sendMessage: sendChatMessage,
  sendMessageWithContext,
  getSessionMessages,
  getRecentSessionMessages,
  getProjectMessages,
  getTaskMessages,
  deleteMessage: deleteChatMessage,
  deleteSessionMessages,
  countSessionMessages,
  // Context-aware chat functions
  sendContextMessage,
  listConversations,
  getConversation,
  createConversation,
  getAgentRunStatus,
  // Task execution chat functions
  getExecutionConversation,
  listTaskExecutions,
  queueExecutionMessage,
  getQueuedExecutionMessages,
  deleteQueuedExecutionMessage,
} as const;
