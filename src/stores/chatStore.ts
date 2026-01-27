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
 *
 * The ID is shared between frontend and backend for reliable sync.
 * Frontend generates the ID and sends it to the backend, ensuring
 * both sides can reference the same message by ID.
 */
export interface QueuedMessage {
  /** Message ID (shared between frontend and backend) */
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
  /** Messages queued to send when agent finishes, keyed by context key */
  queuedMessages: Record<string, QueuedMessage[]>;
  /** Messages queued to send when worker finishes (for task_execution context) */
  executionQueuedMessages: Record<string, QueuedMessage[]>;
  /** Whether an agent is currently running, keyed by context key */
  isAgentRunning: Record<string, boolean>;
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
  /** Set whether an agent is currently running for a context */
  setAgentRunning: (contextKey: string, isRunning: boolean) => void;
  /** Queue a message to be sent when the agent finishes */
  queueMessage: (contextKey: string, content: string, clientId?: string) => void;
  /** Edit a queued message */
  editQueuedMessage: (contextKey: string, id: string, content: string) => void;
  /** Delete a queued message */
  deleteQueuedMessage: (contextKey: string, id: string) => void;
  /** Process the queue (send first message and remove from queue) */
  processQueue: (contextKey: string) => Promise<void>;
  /** Start editing a queued message */
  startEditingQueuedMessage: (contextKey: string, id: string) => void;
  /** Stop editing a queued message */
  stopEditingQueuedMessage: (contextKey: string, id: string) => void;
  /** Queue a message to be sent to the worker when it finishes */
  queueExecutionMessage: (taskId: string, content: string, clientId?: string) => void;
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
    queuedMessages: {},
    executionQueuedMessages: {},
    isAgentRunning: {},

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

    setAgentRunning: (contextKey, isRunning) =>
      set((state) => {
        if (isRunning) {
          state.isAgentRunning[contextKey] = true;
        } else {
          delete state.isAgentRunning[contextKey];
        }
      }),

    queueMessage: (contextKey, content, clientId) =>
      set((state) => {
        const queuedMessage: QueuedMessage = {
          id: clientId ?? `queued-${Date.now()}-${Math.random()}`,
          content,
          createdAt: new Date().toISOString(),
          isEditing: false,
        };
        if (!state.queuedMessages[contextKey]) {
          state.queuedMessages[contextKey] = [];
        }
        state.queuedMessages[contextKey].push(queuedMessage);
      }),

    editQueuedMessage: (contextKey, id, content) =>
      set((state) => {
        const messages = state.queuedMessages[contextKey];
        if (messages) {
          const message = messages.find((m) => m.id === id);
          if (message) {
            message.content = content;
            message.isEditing = false;
          }
        }
      }),

    deleteQueuedMessage: (contextKey, id) =>
      set((state) => {
        if (state.queuedMessages[contextKey]) {
          state.queuedMessages[contextKey] = state.queuedMessages[
            contextKey
          ].filter((m) => m.id !== id);

          // Clean up empty arrays
          if (state.queuedMessages[contextKey].length === 0) {
            delete state.queuedMessages[contextKey];
          }
        }
      }),

    startEditingQueuedMessage: (contextKey, id) =>
      set((state) => {
        const messages = state.queuedMessages[contextKey];
        if (messages) {
          const message = messages.find((m) => m.id === id);
          if (message) {
            message.isEditing = true;
          }
        }
      }),

    stopEditingQueuedMessage: (contextKey, id) =>
      set((state) => {
        const messages = state.queuedMessages[contextKey];
        if (messages) {
          const message = messages.find((m) => m.id === id);
          if (message) {
            message.isEditing = false;
          }
        }
      }),

    processQueue: async (contextKey) => {
      const state = get();
      const messages = state.queuedMessages[contextKey];
      if (!messages || messages.length === 0) {
        return;
      }

      // Remove the first message from the queue
      set((draft) => {
        if (draft.queuedMessages[contextKey]) {
          draft.queuedMessages[contextKey].shift();
          if (draft.queuedMessages[contextKey].length === 0) {
            delete draft.queuedMessages[contextKey];
          }
        }
      });
    },

    queueExecutionMessage: (taskId, content, clientId) =>
      set((state) => {
        const queuedMessage: QueuedMessage = {
          id: clientId ?? `queued-exec-${Date.now()}-${Math.random()}`,
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
 * Select queued messages for a specific context
 * @param contextKey - The context key to get queued messages for
 * @returns Selector function returning queued messages array
 */
export const selectQueuedMessages =
  (contextKey: string) =>
  (state: ChatState): QueuedMessage[] =>
    state.queuedMessages[contextKey] ?? EMPTY_QUEUED_MESSAGES;

/**
 * Select whether an agent is currently running for a context
 * @param contextKey - The context key to check
 * @returns Selector function returning agent running state
 */
export const selectIsAgentRunning =
  (contextKey: string) =>
  (state: ChatState): boolean =>
    state.isAgentRunning[contextKey] ?? false;

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
