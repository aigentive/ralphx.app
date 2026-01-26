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
import type { ViewType } from "@/types/chat";

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
  /** Active question from agent requiring user response */
  activeQuestion: AskUserQuestionPayload | null;
  /** Current execution status (pause state, running/queued counts) */
  executionStatus: ExecutionStatusResponse;
  /** Whether to show archived tasks on the board */
  showArchived: boolean;
  /** Current search query for the task board */
  boardSearchQuery: string | null;
  /** Whether a search request is in flight */
  isSearching: boolean;
  /** ID of task to show in full-screen view, or null if none */
  taskFullViewId: string | null;
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
  /** Set active question from agent */
  setActiveQuestion: (question: AskUserQuestionPayload) => void;
  /** Clear active question after answer submitted */
  clearActiveQuestion: () => void;
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
  /** Set the board search query */
  setBoardSearchQuery: (query: string | null) => void;
  /** Set whether a search is in progress */
  setIsSearching: (searching: boolean) => void;
  /** Open task in full-screen view */
  openTaskFullView: (taskId: string) => void;
  /** Close task full-screen view */
  closeTaskFullView: () => void;
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
    activeQuestion: null,
    executionStatus: {
      isPaused: false,
      runningCount: 0,
      maxConcurrent: 2,
      queuedCount: 0,
      canStartTask: true,
    },
    showArchived: false,
    boardSearchQuery: null,
    isSearching: false,
    taskFullViewId: null,

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
        state.reviewsPanelOpen = !state.reviewsPanelOpen;
      }),

    setReviewsPanelOpen: (open) =>
      set((state) => {
        state.reviewsPanelOpen = open;
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

    setBoardSearchQuery: (query) =>
      set((state) => {
        state.boardSearchQuery = query;
      }),

    setIsSearching: (searching) =>
      set((state) => {
        state.isSearching = searching;
      }),

    openTaskFullView: (taskId) =>
      set((state) => {
        state.taskFullViewId = taskId;
      }),

    closeTaskFullView: () =>
      set((state) => {
        state.taskFullViewId = null;
      }),
  }))
);
