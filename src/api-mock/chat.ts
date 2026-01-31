/**
 * Mock Chat API
 *
 * Mirrors the interface of src/api/chat.ts with mock implementations.
 */

import type { ChatConversation, ContextType } from "@/types/chat-conversation";
import type {
  ChatMessageResponse,
  QueuedMessageResponse,
  SendAgentMessageResult,
} from "@/api/chat";
import { generateTestUuid } from "@/test/mock-data";

// ============================================================================
// Mock State
// ============================================================================

const mockConversations: Map<string, ChatConversation> = new Map();
const mockMessages: Map<string, ChatMessageResponse[]> = new Map();
const mockQueuedMessages: Map<string, QueuedMessageResponse[]> = new Map();

// ============================================================================
// Mock Chat API Functions
// ============================================================================

export async function mockListConversations(
  contextType: ContextType,
  contextId: string
): Promise<ChatConversation[]> {
  return Array.from(mockConversations.values()).filter(
    (c) => c.contextType === contextType && c.contextId === contextId
  );
}

export async function mockGetConversation(
  conversationId: string
): Promise<{ conversation: ChatConversation; messages: ChatMessageResponse[] }> {
  const conversation = mockConversations.get(conversationId);
  if (!conversation) {
    // Return a new empty conversation
    const newConversation: ChatConversation = {
      id: conversationId,
      contextType: "project",
      contextId: "mock-project",
      claudeSessionId: null,
      title: null,
      messageCount: 0,
      lastMessageAt: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    return { conversation: newConversation, messages: [] };
  }
  return {
    conversation,
    messages: mockMessages.get(conversationId) ?? [],
  };
}

export async function mockCreateConversation(
  contextType: ContextType,
  contextId: string
): Promise<ChatConversation> {
  const conversation: ChatConversation = {
    id: generateTestUuid(),
    contextType,
    contextId,
    claudeSessionId: null,
    title: null,
    messageCount: 0,
    lastMessageAt: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  mockConversations.set(conversation.id, conversation);
  return conversation;
}

export async function mockGetAgentRunStatus(
  _conversationId: string
): Promise<null> {
  // No agent runs in mock mode
  return null;
}

export async function mockSendAgentMessage(
  contextType: ContextType,
  contextId: string,
  _content: string
): Promise<SendAgentMessageResult> {
  // Find or create conversation
  let conversation = Array.from(mockConversations.values()).find(
    (c) => c.contextType === contextType && c.contextId === contextId
  );

  const isNew = !conversation;
  if (!conversation) {
    conversation = await mockCreateConversation(contextType, contextId);
  }

  return {
    conversationId: conversation.id,
    agentRunId: generateTestUuid(),
    isNewConversation: isNew,
  };
}

export async function mockQueueAgentMessage(
  contextType: ContextType,
  contextId: string,
  content: string,
  clientId?: string
): Promise<QueuedMessageResponse> {
  const key = `${contextType}:${contextId}`;
  const queued: QueuedMessageResponse = {
    id: clientId ?? generateTestUuid(),
    content,
    createdAt: new Date().toISOString(),
    isEditing: false,
  };

  const existing = mockQueuedMessages.get(key) ?? [];
  existing.push(queued);
  mockQueuedMessages.set(key, existing);

  return queued;
}

export async function mockGetQueuedAgentMessages(
  contextType: ContextType,
  contextId: string
): Promise<QueuedMessageResponse[]> {
  const key = `${contextType}:${contextId}`;
  return mockQueuedMessages.get(key) ?? [];
}

export async function mockDeleteQueuedAgentMessage(
  contextType: ContextType,
  contextId: string,
  messageId: string
): Promise<boolean> {
  const key = `${contextType}:${contextId}`;
  const existing = mockQueuedMessages.get(key) ?? [];
  const filtered = existing.filter((m) => m.id !== messageId);
  mockQueuedMessages.set(key, filtered);
  return existing.length !== filtered.length;
}

export async function mockIsChatServiceAvailable(): Promise<boolean> {
  // Chat is not available in mock mode
  return false;
}

export async function mockStopAgent(
  _contextType: ContextType,
  _contextId: string
): Promise<boolean> {
  // No agent to stop in mock mode
  return false;
}

export async function mockIsAgentRunning(
  _contextType: ContextType,
  _contextId: string
): Promise<boolean> {
  // No agents running in mock mode
  return false;
}

// ============================================================================
// Mock Chat API Object
// ============================================================================

export const mockChatApi = {
  listConversations: mockListConversations,
  getConversation: mockGetConversation,
  createConversation: mockCreateConversation,
  getAgentRunStatus: mockGetAgentRunStatus,
  sendAgentMessage: mockSendAgentMessage,
  queueAgentMessage: mockQueueAgentMessage,
  getQueuedAgentMessages: mockGetQueuedAgentMessages,
  deleteQueuedAgentMessage: mockDeleteQueuedAgentMessage,
  isChatServiceAvailable: mockIsChatServiceAvailable,
  stopAgent: mockStopAgent,
  isAgentRunning: mockIsAgentRunning,
} as const;
