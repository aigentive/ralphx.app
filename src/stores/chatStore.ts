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

const MIN_WIDTH = 280;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 320;

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
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useChatStore = create<ChatState & ChatActions>()(
  immer((set) => ({
    // Initial state
    messages: {},
    context: null,
    isOpen: false,
    width: DEFAULT_WIDTH,
    isLoading: false,

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
