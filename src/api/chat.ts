// Tauri invoke wrappers for chat messages with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { ChatContext } from "../types/chat";

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
// Orchestrator API Functions
// ============================================================================

/**
 * Response from a tool call
 */
export interface ToolCallResultResponse {
  toolName: string;
  success: boolean;
  result: unknown | null;
  error: string | null;
}

/**
 * Response from the orchestrator
 */
export interface OrchestratorMessageResponse {
  responseText: string;
  proposalsCreated: Array<{
    id: string;
    sessionId: string;
    title: string;
    description: string | null;
    category: string;
    suggestedPriority: string;
    priorityScore: number;
    priorityReason: string | null;
  }>;
  toolCalls: ToolCallResultResponse[];
}

const OrchestratorResponseSchema = z.object({
  response_text: z.string(),
  proposals_created: z.array(z.object({
    id: z.string(),
    session_id: z.string(),
    title: z.string(),
    description: z.string().nullable(),
    category: z.string(),
    suggested_priority: z.string(),
    priority_score: z.number(),
    priority_reason: z.string().nullable(),
    priority_factors: z.unknown().nullable(),
    estimated_complexity: z.string(),
    user_priority: z.string().nullable(),
    user_modified: z.boolean(),
    status: z.string(),
    selected: z.boolean(),
    created_task_id: z.string().nullable(),
    sort_order: z.number(),
    created_at: z.string(),
    updated_at: z.string(),
  })),
  tool_calls: z.array(z.object({
    tool_name: z.string(),
    success: z.boolean(),
    result: z.unknown().nullable(),
    error: z.string().nullable(),
  })),
});

/**
 * Send a message to the orchestrator and get a response
 * This invokes the Claude CLI with the orchestrator-ideation agent
 * @param sessionId The ideation session ID
 * @param content The user message content
 * @returns The orchestrator response including any created proposals
 */
export async function sendOrchestratorMessage(
  sessionId: string,
  content: string
): Promise<OrchestratorMessageResponse> {
  const raw = await typedInvoke(
    "send_orchestrator_message",
    { input: { session_id: sessionId, content } },
    OrchestratorResponseSchema
  );

  return {
    responseText: raw.response_text,
    proposalsCreated: raw.proposals_created.map((p) => ({
      id: p.id,
      sessionId: p.session_id,
      title: p.title,
      description: p.description,
      category: p.category,
      suggestedPriority: p.suggested_priority,
      priorityScore: p.priority_score,
      priorityReason: p.priority_reason,
    })),
    toolCalls: raw.tool_calls.map((tc) => ({
      toolName: tc.tool_name,
      success: tc.success,
      result: tc.result,
      error: tc.error,
    })),
  };
}

/**
 * Check if the orchestrator agent is available
 * @returns True if the claude CLI is available
 */
export async function isOrchestratorAvailable(): Promise<boolean> {
  return invoke("is_orchestrator_available");
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
 * Send a context-aware message (uses conversation and --resume)
 * @param contextType The context type (ideation, task, project)
 * @param contextId The context ID (session_id, task_id, project_id)
 * @param content The message content
 * @returns The created message
 */
export async function sendContextMessage(
  contextType: ContextType,
  contextId: string,
  content: string
): Promise<ChatMessageResponse> {
  const raw = await typedInvoke(
    "send_context_message",
    {
      input: {
        context_type: contextType,
        context_id: contextId,
        content,
      },
    },
    ChatMessageResponseSchema
  );
  return transformMessage(raw);
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
      context_type: contextType,
      context_id: contextId,
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
    { conversation_id: conversationId },
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
      context_type: contextType,
      context_id: contextId,
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
    { conversation_id: conversationId },
    AgentRunResponseSchema.nullable()
  );
  return raw ? transformAgentRun(raw) : null;
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
  sendOrchestratorMessage,
  isOrchestratorAvailable,
  // Context-aware chat functions
  sendContextMessage,
  listConversations,
  getConversation,
  createConversation,
  getAgentRunStatus,
} as const;
