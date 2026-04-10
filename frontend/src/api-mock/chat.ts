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
  ChildSessionStatusResponse,
  ConversationStatsResponse,
  QueuedMessageResponse,
  SendAgentMessageResult,
} from "@/api/chat";
import { generateTestUuid } from "@/test/mock-data";
import { buildFallbackConversationStats } from "@/lib/chat/conversation-stats";
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
const mockChildSessionStatuses: Map<string, ChildSessionStatusResponse> = new Map();
const mockChildSessionStatusOverrides: Map<string, MockChildSessionStatusOverride> = new Map();

type MockChildSessionStatusOverride = {
  response?: ChildSessionStatusResponse;
  error?: string;
  delayMs?: number;
};

export interface MockChatController {
  reset(): void;
  seedScenario(name: MockChatScenarioName): void;
  seedConversation(
    conversation: ChatConversation,
    messages: ChatMessageResponse[]
  ): void;
  replaceMessages(
    conversationId: string,
    messages: ChatMessageResponse[]
  ): void;
  listScenarios(): MockChatScenarioName[];
  getChildSessionStatus(sessionId: string): Promise<ChildSessionStatusResponse>;
  setChildSessionStatusOverride(
    sessionId: string,
    override: MockChildSessionStatusOverride
  ): void;
  clearChildSessionStatusOverrides(): void;
  listConversations(
    contextType: ContextType,
    contextId: string
  ): Promise<ChatConversation[]>;
  getConversation(
    conversationId: string
  ): Promise<{ conversation: ChatConversation; messages: ChatMessageResponse[] }>;
  getConversationStats(
    conversationId: string
  ): Promise<ConversationStatsResponse | null>;
}

export function resetMockChatState(): void {
  mockConversations.clear();
  mockMessages.clear();
  mockQueuedMessages.clear();
  mockChildSessionStatuses.clear();
  mockChildSessionStatusOverrides.clear();
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

  for (const [sessionId, status] of Object.entries(scenario.childSessionStatuses ?? {})) {
    mockChildSessionStatuses.set(sessionId, { ...status });
  }
}

function cloneConversation(conversation: ChatConversation): ChatConversation {
  return { ...conversation };
}

function refreshConversationMessageStats(conversationId: string): void {
  const conversation = mockConversations.get(conversationId);
  if (!conversation) {
    return;
  }

  const messages = mockMessages.get(conversationId) ?? [];
  const lastMessageAt =
    messages.length > 0
      ? messages[messages.length - 1]?.createdAt ?? conversation.lastMessageAt
      : null;

  mockConversations.set(conversationId, {
    ...conversation,
    messageCount: messages.length,
    lastMessageAt,
    updatedAt: lastMessageAt ?? conversation.updatedAt,
  });
}

export function seedMockConversation(
  conversation: ChatConversation,
  messages: ChatMessageResponse[]
): void {
  mockConversations.set(conversation.id, cloneConversation(conversation));
  mockMessages.set(
    conversation.id,
    messages.map((message) => cloneMockChatMessage(message))
  );
  refreshConversationMessageStats(conversation.id);
}

export function replaceMockConversationMessages(
  conversationId: string,
  messages: ChatMessageResponse[]
): void {
  mockMessages.set(
    conversationId,
    messages.map((message) => cloneMockChatMessage(message))
  );
  refreshConversationMessageStats(conversationId);
}

function exposeMockChatController(): void {
  if (typeof window === "undefined") {
    return;
  }

  window.__mockChatApi = {
    reset: resetMockChatState,
    seedScenario: seedMockChatScenario,
    seedConversation: seedMockConversation,
    replaceMessages: replaceMockConversationMessages,
    listScenarios: listMockChatScenarios,
    getChildSessionStatus: mockGetChildSessionStatus,
    setChildSessionStatusOverride: mockSetChildSessionStatusOverride,
    clearChildSessionStatusOverrides: mockClearChildSessionStatusOverrides,
    listConversations: mockListConversations,
    getConversation: mockGetConversation,
    getConversationStats: mockGetConversationStats,
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

export async function mockGetConversationStats(
  conversationId: string
): Promise<ConversationStatsResponse | null> {
  const conversation = mockConversations.get(conversationId);
  if (!conversation) {
    return null;
  }

  return buildFallbackConversationStats(
    conversation,
    mockMessages.get(conversationId) ?? []
  );
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

function cloneChildSessionStatus(
  status: ChildSessionStatusResponse
): ChildSessionStatusResponse {
  return {
    ...status,
    agent_state: { ...status.agent_state },
    recent_messages: status.recent_messages.map((message) => ({ ...message })),
  };
}

function mockSetChildSessionStatusOverride(
  sessionId: string,
  override: MockChildSessionStatusOverride
): void {
  mockChildSessionStatusOverrides.set(sessionId, override);
}

function mockClearChildSessionStatusOverrides(): void {
  mockChildSessionStatusOverrides.clear();
}

export async function mockGetChildSessionStatus(
  sessionId: string
): Promise<ChildSessionStatusResponse> {
  const override = mockChildSessionStatusOverrides.get(sessionId);
  const delayMs = override?.delayMs ?? 0;

  if (delayMs > 0) {
    await new Promise((resolve) => globalThis.setTimeout(resolve, delayMs));
  }

  if (override?.error) {
    throw new Error(override.error);
  }

  const response = override?.response ?? mockChildSessionStatuses.get(sessionId);
  if (!response) {
    throw new Error(`No mock child session status seeded for ${sessionId}`);
  }

  return cloneChildSessionStatus(response);
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
  reset: resetMockChatState,
  seedScenario: seedMockChatScenario,
  seedConversation: seedMockConversation,
  replaceMessages: replaceMockConversationMessages,
  listConversations: mockListConversations,
  getConversation: mockGetConversation,
  createConversation: mockCreateConversation,
  getChildSessionStatus: mockGetChildSessionStatus,
  getAgentRunStatus: mockGetAgentRunStatus,
  sendAgentMessage: mockSendAgentMessage,
  getQueuedAgentMessages: mockGetQueuedAgentMessages,
  deleteQueuedAgentMessage: mockDeleteQueuedAgentMessage,
  isChatServiceAvailable: mockIsChatServiceAvailable,
  stopAgent: mockStopAgent,
  isAgentRunning: mockIsAgentRunning,
} as const;
