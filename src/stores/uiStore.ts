/**
 * UI store using Zustand with immer middleware
 *
 * Manages ephemeral UI state: sidebar visibility, modal state,
 * notifications, loading states, confirmation dialogs, and
 * active user questions from agents.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";
import type { ExecutionStatusResponse } from "@/lib/tauri";
import type { RecoveryPromptEvent } from "@/types/events";
import type { ViewType } from "@/types/chat";
import type { InternalStatus } from "@/types/status";

export type GraphSelection =
  | { kind: "task"; id: string }
  | { kind: "planGroup"; id: string }
  | { kind: "tierGroup"; id: string }
  | { kind: "customGroup"; id: string };

// ============================================================================
// Chat Visibility Persistence
// ============================================================================

const CHAT_VISIBILITY_KEY = "ralphx-chat-visibility-by-view";

const DEFAULT_CHAT_VISIBILITY: Record<ViewType, boolean> = {
  kanban: true, // visible by default (integrated layout)
  graph: false, // hidden by default (focus on visualization)
  ideation: true, // always visible (built-in chat)
  extensibility: false,
  activity: false,
  settings: false,
  task_detail: false,
};

function loadChatVisibility(): Record<ViewType, boolean> {
  try {
    const saved = localStorage.getItem(CHAT_VISIBILITY_KEY);
    if (saved) {
      return { ...DEFAULT_CHAT_VISIBILITY, ...JSON.parse(saved) };
    }
  } catch {
    /* ignore parse errors */
  }
  return { ...DEFAULT_CHAT_VISIBILITY };
}

function applyTaskSelection(
  state: { selectedTaskId: string | null; taskHistoryState: UiState["taskHistoryState"]; chatVisibleByView: Record<ViewType, boolean> },
  taskId: string | null
): void {
  state.selectedTaskId = taskId;
  // Auto-open chat when a task is selected in kanban view
  if (taskId !== null) {
    state.chatVisibleByView.kanban = true;
    saveChatVisibility(state.chatVisibleByView);
  }
  // Clear history state when task is deselected
  if (taskId === null) {
    state.taskHistoryState = null;
  }
}

function saveChatVisibility(visibility: Record<ViewType, boolean>): void {
  localStorage.setItem(CHAT_VISIBILITY_KEY, JSON.stringify(visibility));
}

// ============================================================================
// Types
// ============================================================================

/** Modal types available in the application */
export type ModalType =
  | "task-detail"
  | "task-create"
  | "project-settings"
  | "settings"
  | "ask-user-question"
  | null;

/** Notification severity levels */
export type NotificationType = "success" | "error" | "warning" | "info";

/** A notification to display to the user */
export interface Notification {
  id: string;
  type: NotificationType;
  message: string;
  title?: string;
  duration?: number;
}

/** Filter for activity view navigation from StatusActivityBadge */
export interface ActivityFilter {
  taskId: string | null;
  sessionId: string | null;
}

/** Confirmation dialog configuration */
export interface ConfirmationConfig {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel?: () => void;
}

// ============================================================================
// State Interface
// ============================================================================

interface UiState {
  /** Whether the sidebar is open */
  sidebarOpen: boolean;
  /** Whether the reviews panel is open */
  reviewsPanelOpen: boolean;
  /** Current main view (kanban, ideation, etc.) */
  currentView: ViewType;
  /** Currently active modal type, or null if none */
  activeModal: ModalType;
  /** Context data for the active modal */
  modalContext: Record<string, unknown> | undefined;
  /** Active notifications */
  notifications: Notification[];
  /** Loading states for different parts of the UI */
  loading: Record<string, boolean>;
  /** Active confirmation dialog */
  confirmation: ConfirmationConfig | null;
  /** Active questions from agents, keyed by sessionId */
  activeQuestions: Record<string, AskUserQuestionPayload>;
  /** Answered question summaries, keyed by sessionId */
  answeredQuestions: Record<string, string>;
  /** Active recovery prompt from backend */
  recoveryPrompt: RecoveryPromptEvent | null;
  /** Surface that currently owns the recovery prompt dialog */
  recoveryPromptSurface: "chat" | "task_detail" | null;
  /** Current execution status (pause state, running/queued counts) */
  executionStatus: ExecutionStatusResponse;
  /** Whether to show archived tasks on the board */
  showArchived: boolean;
  /** Whether to show merge tasks on the board */
  showMergeTasks: boolean;
  /** Current search query for the task board */
  boardSearchQuery: string | null;
  /** Whether a search request is in flight */
  isSearching: boolean;
  /** ID of selected task for split-screen overlay (kanban view only) */
  selectedTaskId: string | null;
  /** Active selection in the task graph (single selection across types) */
  graphSelection: GraphSelection | null;
  /** User toggle for graph right panel visibility */
  graphRightPanelUserOpen: boolean;
  /** Compact-mode toggle for graph right panel visibility */
  graphRightPanelCompactOpen: boolean;
  /** History state for time-travel feature - shared between TaskDetailOverlay and IntegratedChatPanel */
  taskHistoryState: {
    status: InternalStatus;
    timestamp: string;
    /** Conversation ID from the state transition metadata (for states that spawn conversations) */
    conversationId?: string | undefined;
    /** Agent run ID from the state transition metadata */
    agentRunId?: string | undefined;
  } | null;
  /** Task creation overlay context, or null if closed */
  taskCreationContext: { projectId: string; defaultTitle?: string } | null;
  /** Chat visibility per view (persisted to localStorage) */
  chatVisibleByView: Record<ViewType, boolean>;
  /** Whether the welcome screen is manually shown (vs. empty state) */
  showWelcomeOverlay: boolean;
  /** View to return to when closing manually-opened welcome screen */
  welcomeOverlayReturnView: ViewType | null;
  /** Filter for activity view navigation (set by StatusActivityBadge) */
  activityFilter: ActivityFilter;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface UiActions {
  /** Toggle sidebar visibility */
  toggleSidebar: () => void;
  /** Set sidebar visibility directly */
  setSidebarOpen: (open: boolean) => void;
  /** Toggle reviews panel visibility */
  toggleReviewsPanel: () => void;
  /** Set reviews panel visibility directly */
  setReviewsPanelOpen: (open: boolean) => void;
  /** Set the current main view */
  setCurrentView: (view: ViewType) => void;
  /** Open a modal with optional context */
  openModal: (type: ModalType, context?: Record<string, unknown>) => void;
  /** Close the current modal */
  closeModal: () => void;
  /** Add a notification */
  addNotification: (notification: Notification) => void;
  /** Remove a notification by ID */
  removeNotification: (id: string) => void;
  /** Clear all notifications */
  clearNotifications: () => void;
  /** Set loading state for a key */
  setLoading: (key: string, loading: boolean) => void;
  /** Show a confirmation dialog */
  showConfirmation: (config: ConfirmationConfig) => void;
  /** Hide the confirmation dialog */
  hideConfirmation: () => void;
  /** Set active question for a session */
  setActiveQuestion: (sessionId: string, question: AskUserQuestionPayload) => void;
  /** Clear active question for a session */
  clearActiveQuestion: (sessionId: string) => void;
  /** Dismiss question for a session (clears both question and answered state) */
  dismissQuestion: (sessionId: string) => void;
  /** Set answered summary for a session */
  setAnsweredQuestion: (sessionId: string, summary: string) => void;
  /** Clear answered summary for a session */
  clearAnsweredQuestion: (sessionId: string) => void;
  /** Set active recovery prompt */
  setRecoveryPrompt: (prompt: RecoveryPromptEvent) => void;
  /** Clear active recovery prompt */
  clearRecoveryPrompt: () => void;
  /** Set surface that owns the recovery prompt dialog */
  setRecoveryPromptSurface: (surface: "chat" | "task_detail" | null) => void;
  /** Update full execution status from backend */
  setExecutionStatus: (status: ExecutionStatusResponse) => void;
  /** Set just the paused state */
  setExecutionPaused: (isPaused: boolean) => void;
  /** Set running count */
  setExecutionRunningCount: (count: number) => void;
  /** Set queued count */
  setExecutionQueuedCount: (count: number) => void;
  /** Set whether to show archived tasks */
  setShowArchived: (show: boolean) => void;
  /** Set whether to show merge tasks */
  setShowMergeTasks: (show: boolean) => void;
  /** Set the board search query */
  setBoardSearchQuery: (query: string | null) => void;
  /** Set whether a search is in progress */
  setIsSearching: (searching: boolean) => void;
  /** Set selected task ID for split-screen overlay */
  setSelectedTaskId: (taskId: string | null) => void;
  /** Set active graph selection */
  setGraphSelection: (selection: GraphSelection | null) => void;
  /** Clear active graph selection */
  clearGraphSelection: () => void;
  /** Toggle graph right panel visibility */
  toggleGraphRightPanel: () => void;
  /** Set graph right panel visibility */
  setGraphRightPanelUserOpen: (open: boolean) => void;
  /** Toggle compact-mode graph right panel visibility */
  toggleGraphRightPanelCompactOpen: () => void;
  /** Set compact-mode graph right panel visibility */
  setGraphRightPanelCompactOpen: (open: boolean) => void;
  /** Set task history state for time-travel feature */
  setTaskHistoryState: (state: {
    status: InternalStatus;
    timestamp: string;
    conversationId?: string | undefined;
    agentRunId?: string | undefined;
  } | null) => void;
  /** Open task creation overlay */
  openTaskCreation: (projectId: string, defaultTitle?: string) => void;
  /** Close task creation overlay */
  closeTaskCreation: () => void;
  /** Set chat visibility for a specific view */
  setChatVisible: (view: ViewType, visible: boolean) => void;
  /** Toggle chat visibility for a specific view */
  toggleChatVisible: (view: ViewType) => void;
  /** Open welcome screen overlay, saving current view */
  openWelcomeOverlay: () => void;
  /** Close welcome screen overlay, restoring previous view */
  closeWelcomeOverlay: () => void;
  /** Set activity filter for context-aware navigation */
  setActivityFilter: (filter: Partial<ActivityFilter>) => void;
  /** Clear activity filter */
  clearActivityFilter: () => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useUiStore = create<UiState & UiActions>()(
  immer((set) => ({
    // Initial state
    sidebarOpen: true,
    reviewsPanelOpen: false,
    currentView: "kanban" as ViewType,
    activeModal: null,
    modalContext: undefined,
    notifications: [],
    loading: {},
    confirmation: null,
    activeQuestions: {},
    answeredQuestions: {},
    recoveryPrompt: null,
    recoveryPromptSurface: null,
    executionStatus: {
      isPaused: false,
      runningCount: 0,
      maxConcurrent: 2,
      globalMaxConcurrent: 20,
      queuedCount: 0,
      canStartTask: true,
    },
    showArchived: false,
    showMergeTasks: false,
    boardSearchQuery: null,
    isSearching: false,
    selectedTaskId: null,
    graphSelection: null,
    graphRightPanelUserOpen: true,
    graphRightPanelCompactOpen: false,
    taskHistoryState: null,
    taskCreationContext: null,
    chatVisibleByView: loadChatVisibility(),
    showWelcomeOverlay: false,
    welcomeOverlayReturnView: null,
    activityFilter: { taskId: null, sessionId: null },

    // Actions
    toggleSidebar: () =>
      set((state) => {
        state.sidebarOpen = !state.sidebarOpen;
      }),

    setSidebarOpen: (open) =>
      set((state) => {
        state.sidebarOpen = open;
      }),

    toggleReviewsPanel: () =>
      set((state) => {
        const willOpen = !state.reviewsPanelOpen;
        state.reviewsPanelOpen = willOpen;
        // Mutual exclusion: close chat when opening reviews
        if (willOpen) {
          Object.keys(state.chatVisibleByView).forEach((view) => {
            state.chatVisibleByView[view as ViewType] = false;
          });
          saveChatVisibility(state.chatVisibleByView);
        }
      }),

    setReviewsPanelOpen: (open) =>
      set((state) => {
        state.reviewsPanelOpen = open;
        // Mutual exclusion: close chat when opening reviews
        if (open) {
          Object.keys(state.chatVisibleByView).forEach((view) => {
            state.chatVisibleByView[view as ViewType] = false;
          });
          saveChatVisibility(state.chatVisibleByView);
        }
      }),

    setCurrentView: (view) =>
      set((state) => {
        state.currentView = view;
      }),

    openModal: (type, context) =>
      set((state) => {
        state.activeModal = type;
        state.modalContext = context;
      }),

    closeModal: () =>
      set((state) => {
        state.activeModal = null;
        state.modalContext = undefined;
      }),

    addNotification: (notification) =>
      set((state) => {
        state.notifications.push(notification);
      }),

    removeNotification: (id) =>
      set((state) => {
        state.notifications = state.notifications.filter((n) => n.id !== id);
      }),

    clearNotifications: () =>
      set((state) => {
        state.notifications = [];
      }),

    setLoading: (key, loading) =>
      set((state) => {
        state.loading[key] = loading;
      }),

    showConfirmation: (config) =>
      set((state) => {
        state.confirmation = config;
      }),

    hideConfirmation: () =>
      set((state) => {
        state.confirmation = null;
      }),

    setActiveQuestion: (question) =>
      set((state) => {
        state.activeQuestion = question;
      }),

    clearActiveQuestion: () =>
      set((state) => {
        state.activeQuestion = null;
      }),

    setRecoveryPrompt: (prompt) =>
      set((state) => {
        state.recoveryPrompt = prompt;
        state.recoveryPromptSurface = null;
      }),

    clearRecoveryPrompt: () =>
      set((state) => {
        state.recoveryPrompt = null;
        state.recoveryPromptSurface = null;
      }),

    setRecoveryPromptSurface: (surface) =>
      set((state) => {
        state.recoveryPromptSurface = surface;
      }),

    setExecutionStatus: (status) =>
      set((state) => {
        state.executionStatus = status;
      }),

    setExecutionPaused: (isPaused) =>
      set((state) => {
        state.executionStatus.isPaused = isPaused;
      }),

    setExecutionRunningCount: (count) =>
      set((state) => {
        state.executionStatus.runningCount = count;
      }),

    setExecutionQueuedCount: (count) =>
      set((state) => {
        state.executionStatus.queuedCount = count;
      }),

    setShowArchived: (show) =>
      set((state) => {
        state.showArchived = show;
      }),

    setShowMergeTasks: (show) =>
      set((state) => {
        state.showMergeTasks = show;
      }),

    setBoardSearchQuery: (query) =>
      set((state) => {
        state.boardSearchQuery = query;
      }),

    setIsSearching: (searching) =>
      set((state) => {
        state.isSearching = searching;
      }),

    setSelectedTaskId: (taskId) =>
      set((state) => {
        applyTaskSelection(state, taskId);
        if (taskId !== null) {
          state.graphSelection = { kind: "task", id: taskId };
        } else if (state.graphSelection?.kind === "task") {
          state.graphSelection = null;
        }
      }),

    setGraphSelection: (selection) =>
      set((state) => {
        state.graphSelection = selection;
      }),

    clearGraphSelection: () =>
      set((state) => {
        state.graphSelection = null;
      }),

    toggleGraphRightPanel: () =>
      set((state) => {
        state.graphRightPanelUserOpen = !state.graphRightPanelUserOpen;
      }),

    setGraphRightPanelUserOpen: (open) =>
      set((state) => {
        state.graphRightPanelUserOpen = open;
      }),

    toggleGraphRightPanelCompactOpen: () =>
      set((state) => {
        state.graphRightPanelCompactOpen = !state.graphRightPanelCompactOpen;
      }),

    setGraphRightPanelCompactOpen: (open) =>
      set((state) => {
        state.graphRightPanelCompactOpen = open;
      }),

    setTaskHistoryState: (historyState) =>
      set((state) => {
        state.taskHistoryState = historyState;
      }),

    openTaskCreation: (projectId, defaultTitle) =>
      set((state) => {
        state.taskCreationContext = {
          projectId,
          ...(defaultTitle !== undefined && { defaultTitle }),
        };
      }),

    closeTaskCreation: () =>
      set((state) => {
        state.taskCreationContext = null;
      }),

    setChatVisible: (view, visible) =>
      set((state) => {
        state.chatVisibleByView[view] = visible;
        saveChatVisibility(state.chatVisibleByView);
      }),

    toggleChatVisible: (view) =>
      set((state) => {
        const willOpen = !state.chatVisibleByView[view];
        state.chatVisibleByView[view] = willOpen;
        saveChatVisibility(state.chatVisibleByView);
        // Mutual exclusion: close reviews when opening chat
        if (willOpen) {
          state.reviewsPanelOpen = false;
        }
      }),

    openWelcomeOverlay: () =>
      set((state) => {
        state.welcomeOverlayReturnView = state.currentView;
        state.showWelcomeOverlay = true;
      }),

    closeWelcomeOverlay: () =>
      set((state) => {
        state.showWelcomeOverlay = false;
      }),

    setActivityFilter: (filter) =>
      set((state) => {
        if (filter.taskId !== undefined) {
          state.activityFilter.taskId = filter.taskId;
        }
        if (filter.sessionId !== undefined) {
          state.activityFilter.sessionId = filter.sessionId;
        }
      }),

    clearActivityFilter: () =>
      set((state) => {
        state.activityFilter = { taskId: null, sessionId: null };
      }),
  }))
);

// Expose uiStore to window in web mode for Playwright testing
if (typeof window !== "undefined" && !window.__TAURI_INTERNALS__) {
  window.__uiStore = useUiStore;
}
