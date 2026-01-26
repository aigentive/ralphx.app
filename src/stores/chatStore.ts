/**
 * Chat store using Zustand with immer middleware
 *
 * Manages chat panel state for the frontend. Messages are stored in a
 * Record keyed by context key (e.g., "session:abc", "task:def", "project:xyz")
 * for efficient lookup by context.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { ChatMessage } from "@/types/ideation";
import type { ChatContext } from "@/types/chat";

// ============================================================================
// Constants
// ============================================================================

const MIN_WIDTH = 320;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 384; // Matches ReviewsPanel (w-96)

// ============================================================================
// Types
// ============================================================================

/**
 * A queued message that will be sent when the agent finishes
 */
export interface QueuedMessage {
  /** Local ID for tracking */
  id: string;
  /** Message content */
  content: string;
  /** When the message was queued */
  createdAt: string;
  /** Whether this message is currently being edited */
  isEditing: boolean;
}

// ============================================================================
// State Interface
// ============================================================================

interface ChatState {
  /** Messages indexed by context key for efficient lookup */
  messages: Record<string, ChatMessage[]>;
  /** Current chat context (view, selected items) */
  context: ChatContext | null;
  /** Whether the chat panel is open */
  isOpen: boolean;
  /** Panel width in pixels */
  width: number;
  /** Loading state for async operations */
  isLoading: boolean;
  /** Active conversation ID for the current context */
  activeConversationId: string | null;
  /** Messages queued to send when agent finishes (for ideation/task/project chat) */
  queuedMessages: QueuedMessage[];
  /** Messages queued to send when worker finishes (for task_execution context) */
  executionQueuedMessages: Record<string, QueuedMessage[]>;
  /** Whether an agent is currently running */
  isAgentRunning: boolean;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface ChatActions {
  /** Set the current chat context */
  setContext: (context: ChatContext | null) => void;
  /** Toggle the chat panel open/close */
  togglePanel: () => void;
  /** Set the panel open state directly */
  setOpen: (isOpen: boolean) => void;
  /** Set the panel width (clamped to min/max) */
  setWidth: (width: number) => void;
  /** Add a message to a context */
  addMessage: (contextKey: string, message: ChatMessage) => void;
  /** Set all messages for a context */
  setMessages: (contextKey: string, messages: ChatMessage[]) => void;
  /** Clear messages for a context */
  clearMessages: (contextKey: string) => void;
  /** Set loading state */
  setLoading: (isLoading: boolean) => void;
  /** Set the active conversation ID */
  setActiveConversation: (conversationId: string | null) => void;
  /** Set whether an agent is currently running */
  setAgentRunning: (isRunning: boolean) => void;
  /** Queue a message to be sent when the agent finishes */
  queueMessage: (content: string) => void;
  /** Edit a queued message */
  editQueuedMessage: (id: string, content: string) => void;
  /** Delete a queued message */
  deleteQueuedMessage: (id: string) => void;
  /** Process the queue (send first message and remove from queue) */
  processQueue: () => Promise<void>;
  /** Start editing a queued message */
  startEditingQueuedMessage: (id: string) => void;
  /** Stop editing a queued message */
  stopEditingQueuedMessage: (id: string) => void;
  /** Queue a message to be sent to the worker when it finishes */
  queueExecutionMessage: (taskId: string, content: string) => void;
  /** Delete a queued execution message */
  deleteExecutionQueuedMessage: (taskId: string, messageId: string) => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useChatStore = create<ChatState & ChatActions>()(
  immer((set, get) => ({
    // Initial state
    messages: {},
    context: null,
    isOpen: false,
    width: DEFAULT_WIDTH,
    isLoading: false,
    activeConversationId: null,
    queuedMessages: [],
    executionQueuedMessages: {},
    isAgentRunning: false,

    // Actions
    setContext: (context) =>
      set((state) => {
        state.context = context;
      }),

    togglePanel: () =>
      set((state) => {
        state.isOpen = !state.isOpen;
      }),

    setOpen: (isOpen) =>
      set((state) => {
        state.isOpen = isOpen;
      }),

    setWidth: (width) =>
      set((state) => {
        state.width = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, width));
      }),

    addMessage: (contextKey, message) =>
      set((state) => {
        if (!state.messages[contextKey]) {
          state.messages[contextKey] = [];
        }
        state.messages[contextKey].push(message);
      }),

    setMessages: (contextKey, messages) =>
      set((state) => {
        state.messages[contextKey] = messages;
      }),

    clearMessages: (contextKey) =>
      set((state) => {
        delete state.messages[contextKey];
      }),

    setLoading: (isLoading) =>
      set((state) => {
        state.isLoading = isLoading;
      }),

    setActiveConversation: (conversationId) =>
      set((state) => {
        state.activeConversationId = conversationId;
      }),

    setAgentRunning: (isRunning) =>
      set((state) => {
        state.isAgentRunning = isRunning;
      }),

    queueMessage: (content) =>
      set((state) => {
        const queuedMessage: QueuedMessage = {
          id: `queued-${Date.now()}-${Math.random()}`,
          content,
          createdAt: new Date().toISOString(),
          isEditing: false,
        };
        state.queuedMessages.push(queuedMessage);
      }),

    editQueuedMessage: (id, content) =>
      set((state) => {
        const message = state.queuedMessages.find((m) => m.id === id);
        if (message) {
          message.content = content;
          message.isEditing = false;
        }
      }),

    deleteQueuedMessage: (id) =>
      set((state) => {
        state.queuedMessages = state.queuedMessages.filter((m) => m.id !== id);
      }),

    startEditingQueuedMessage: (id) =>
      set((state) => {
        const message = state.queuedMessages.find((m) => m.id === id);
        if (message) {
          message.isEditing = true;
        }
      }),

    stopEditingQueuedMessage: (id) =>
      set((state) => {
        const message = state.queuedMessages.find((m) => m.id === id);
        if (message) {
          message.isEditing = false;
        }
      }),

    processQueue: async () => {
      const state = get();
      if (state.queuedMessages.length === 0) {
        return;
      }

      // Remove the first message from the queue
      set((draft) => {
        draft.queuedMessages.shift();
      });

      // The actual sending logic will be handled by the useChat hook
      // This function just manages the queue state
      // The useChat hook will:
      // 1. Subscribe to chat:run_completed events
      // 2. Get the first queued message BEFORE calling processQueue
      // 3. Call processQueue to remove it from the queue
      // 4. Send the message via the API
      // 5. Handle the response
    },

    queueExecutionMessage: (taskId, content) =>
      set((state) => {
        const queuedMessage: QueuedMessage = {
          id: `queued-exec-${Date.now()}-${Math.random()}`,
          content,
          createdAt: new Date().toISOString(),
          isEditing: false,
        };

        if (!state.executionQueuedMessages[taskId]) {
          state.executionQueuedMessages[taskId] = [];
        }
        state.executionQueuedMessages[taskId].push(queuedMessage);
      }),

    deleteExecutionQueuedMessage: (taskId, messageId) =>
      set((state) => {
        if (state.executionQueuedMessages[taskId]) {
          state.executionQueuedMessages[taskId] =
            state.executionQueuedMessages[taskId].filter(
              (m) => m.id !== messageId
            );

          // Clean up empty arrays
          if (state.executionQueuedMessages[taskId].length === 0) {
            delete state.executionQueuedMessages[taskId];
          }
        }
      }),
  }))
);

// ============================================================================
// Context Key Helper
// ============================================================================

/**
 * Generate a context key from a ChatContext
 * Used to key messages by their source context
 */
export function getContextKey(context: ChatContext): string {
  if (context.view === "ideation" && context.ideationSessionId) {
    return `session:${context.ideationSessionId}`;
  }
  if (context.view === "task_detail" && context.selectedTaskId) {
    return `task:${context.selectedTaskId}`;
  }
  // Default to project-level context
  return `project:${context.projectId}`;
}

// ============================================================================
// Selectors (defined outside store for memoization)
// ============================================================================

/**
 * Select messages for a specific context key
 * @param contextKey - The context key to get messages for
 * @returns Selector function returning messages array
 */
export const selectMessagesForContext =
  (contextKey: string) =>
  (state: ChatState): ChatMessage[] =>
    state.messages[contextKey] ?? [];

/**
 * Select message count for a specific context key
 * @param contextKey - The context key to count messages for
 * @returns Selector function returning message count
 */
export const selectMessageCount =
  (contextKey: string) =>
  (state: ChatState): number =>
    state.messages[contextKey]?.length ?? 0;

/**
 * Select queued messages
 * @returns Selector function returning queued messages array
 */
export const selectQueuedMessages = (
  state: ChatState & ChatActions
): QueuedMessage[] => state.queuedMessages;

/**
 * Select whether an agent is currently running
 * @returns Selector function returning agent running state
 */
export const selectIsAgentRunning = (
  state: ChatState & ChatActions
): boolean => state.isAgentRunning;

/**
 * Select active conversation ID
 * @returns Selector function returning active conversation ID
 */
export const selectActiveConversationId = (
  state: ChatState & ChatActions
): string | null => state.activeConversationId;

// Stable empty array to avoid creating new references
const EMPTY_QUEUED_MESSAGES: QueuedMessage[] = [];

/**
 * Select queued execution messages for a specific task
 * @param taskId - The task ID to get queued messages for
 * @returns Selector function returning queued execution messages array
 */
export const selectExecutionQueuedMessages =
  (taskId: string) =>
  (state: ChatState): QueuedMessage[] =>
    state.executionQueuedMessages[taskId] ?? EMPTY_QUEUED_MESSAGES;
