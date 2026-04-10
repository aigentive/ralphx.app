/**
 * Mock Chat API
 *
 * Mirrors the interface of src/api/chat.ts with mock implementations.
 */

import type {
  ChatConversation,
  ContextType,
} from "@/types/chat-conversation";
import { normalizeConversationProviderMetadata } from "@/types/chat-conversation";
import type {
  ChatMessageResponse,
  QueuedMessageResponse,
  SendAgentMessageResult,
} from "@/api/chat";
import { generateTestUuid } from "@/test/mock-data";
import {
  cloneMockChatMessage,
  getMockChatScenario,
  listMockChatScenarios,
  type MockChatScenarioName,
} from "./chat-scenarios";

// ============================================================================
// Mock State
// ============================================================================

const mockConversations: Map<string, ChatConversation> = new Map();
const mockMessages: Map<string, ChatMessageResponse[]> = new Map();
const mockQueuedMessages: Map<string, QueuedMessageResponse[]> = new Map();

export interface MockChatController {
  reset(): void;
  seedScenario(name: MockChatScenarioName): void;
  listScenarios(): MockChatScenarioName[];
  listConversations(
    contextType: ContextType,
    contextId: string
  ): Promise<ChatConversation[]>;
  getConversation(
    conversationId: string
  ): Promise<{ conversation: ChatConversation; messages: ChatMessageResponse[] }>;
}

export function resetMockChatState(): void {
  mockConversations.clear();
  mockMessages.clear();
  mockQueuedMessages.clear();
}

export function seedMockChatScenario(name: MockChatScenarioName): void {
  const scenario = getMockChatScenario(name);
  resetMockChatState();

  for (const conversation of scenario.conversations) {
    mockConversations.set(conversation.id, conversation);
  }

  for (const [conversationId, messages] of Object.entries(scenario.messages)) {
    mockMessages.set(
      conversationId,
      messages.map((message) => cloneMockChatMessage(message))
    );
  }

  for (const [key, queued] of Object.entries(scenario.queuedMessages ?? {})) {
    mockQueuedMessages.set(key, [...queued]);
  }
}

function exposeMockChatController(): void {
  if (typeof window === "undefined") {
    return;
  }

  window.__mockChatApi = {
    reset: resetMockChatState,
    seedScenario: seedMockChatScenario,
    listScenarios: listMockChatScenarios,
    listConversations: mockListConversations,
    getConversation: mockGetConversation,
  };
}

exposeMockChatController();

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
      ...normalizeConversationProviderMetadata({}),
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
    ...normalizeConversationProviderMetadata({}),
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
    wasQueued: false,
  };
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
  getQueuedAgentMessages: mockGetQueuedAgentMessages,
  deleteQueuedAgentMessage: mockDeleteQueuedAgentMessage,
  isChatServiceAvailable: mockIsChatServiceAvailable,
  stopAgent: mockStopAgent,
  isAgentRunning: mockIsAgentRunning,
} as const;
