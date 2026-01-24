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
    { session_id: sessionId },
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
    { session_id: sessionId, limit },
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
    { project_id: projectId },
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
    { task_id: taskId },
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
  await invoke("delete_session_messages", { session_id: sessionId });
}

/**
 * Count messages in a session
 * @param sessionId The session ID
 * @returns Number of messages
 */
export async function countSessionMessages(sessionId: string): Promise<number> {
  return typedInvoke(
    "count_session_messages",
    { session_id: sessionId },
    z.number()
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
} as const;
