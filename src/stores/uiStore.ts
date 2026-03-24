/**
 * UI store using Zustand with immer middleware
 *
 * Manages ephemeral UI state: sidebar visibility, modal state,
 * notifications, loading states, confirmation dialogs, and
 * active user questions from agents.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { enableMapSet } from "immer";
import { invoke } from "@tauri-apps/api/core";
import { featureFlagsSchema } from "@/types/feature-flags";
import type { FeatureFlags } from "@/types/feature-flags";
import { isViewEnabled } from "@/hooks/useFeatureFlags";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";
import type { ExecutionStatusResponse } from "@/lib/tauri";
import type { RecoveryPromptEvent } from "@/types/events";
import type { ViewType } from "@/types/chat";
import type { InternalStatus } from "@/types/status";
import {
  loadCollapsedColumns,
  saveCollapsedColumns,
} from "@/components/tasks/TaskBoard/Column.utils";

// ============================================================================
// Show Merge Tasks Persistence
// ============================================================================

const SHOW_MERGE_TASKS_KEY = "ralphx-show-merge-tasks";

function loadShowMergeTasks(): boolean {
  try {
    const saved = localStorage.getItem(SHOW_MERGE_TASKS_KEY);
    if (saved !== null) {
      return JSON.parse(saved) as boolean;
    }
  } catch {
    /* ignore parse errors */
  }
  return true; // default: visible
}

function saveShowMergeTasks(show: boolean): void {
  try {
    localStorage.setItem(SHOW_MERGE_TASKS_KEY, JSON.stringify(show));
  } catch {
    /* ignore write errors */
  }
}
import { useIdeationStore } from "@/stores/ideationStore";
import { useProjectStore } from "@/stores/projectStore";

enableMapSet();

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
  insights: false,
  settings: false,
  task_detail: false,
  team: false, // team view has its own split layout
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
// Per-Project Route Persistence
// ============================================================================

const VIEW_BY_PROJECT_KEY = "ralphx-views-by-project";
const SESSION_BY_PROJECT_KEY = "ralphx-sessions-by-project";

function loadViewByProject(): Record<string, ViewType> {
  try {
    const stored = localStorage.getItem(VIEW_BY_PROJECT_KEY);
    return stored ? (JSON.parse(stored) as Record<string, ViewType>) : {};
  } catch {
    return {};
  }
}

function saveViewByProject(map: Record<string, ViewType>): void {
  try {
    localStorage.setItem(VIEW_BY_PROJECT_KEY, JSON.stringify(map));
  } catch {
    /* ignore write errors */
  }
}

function loadSessionByProject(): Record<string, string | null> {
  try {
    const stored = localStorage.getItem(SESSION_BY_PROJECT_KEY);
    return stored ? (JSON.parse(stored) as Record<string, string | null>) : {};
  } catch {
    return {};
  }
}

function saveSessionByProject(map: Record<string, string | null>): void {
  try {
    localStorage.setItem(SESSION_BY_PROJECT_KEY, JSON.stringify(map));
  } catch {
    /* ignore write errors */
  }
}

// ============================================================================
// Feature Flags (cached for synchronous guard use in Zustand actions)
// ============================================================================

const ALL_ENABLED_FLAGS: FeatureFlags = {
  activityPage: true,
  extensibilityPage: true,
  battleMode: true,
};

// ============================================================================
// Types
// ============================================================================

/** Modal types available in the application */
export type ModalType =
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
  /** Whether Battle Mode is active in Graph view */
  battleModeActive: boolean;
  /** Snapshot of graph panel visibility before entering battle mode */
  battleModePanelRestoreState: { userOpen: boolean; compactOpen: boolean } | null;
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
  /** View to return to when leaving team split view */
  previousView: ViewType | null;
  /** Filter for activity view navigation (set by StatusActivityBadge) */
  activityFilter: ActivityFilter;
  /** Set of collapsed column IDs (persisted to localStorage) */
  collapsedColumns: Set<string>;
  /** Per-project last view (persisted to localStorage) */
  viewByProject: Record<string, ViewType>;
  /** Per-project last ideation session ID (persisted to localStorage) */
  sessionByProject: Record<string, string | null>;
  /** Cached UI feature flags (fetched once at startup, defaults to all-enabled) */
  featureFlags: FeatureFlags;
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
  /** Enter battle mode and capture graph panel visibility state */
  enterBattleMode: () => void;
  /** Exit battle mode and restore graph panel visibility state */
  exitBattleMode: () => void;
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
  /** Set collapse state for a specific column */
  setColumnCollapsed: (columnId: string, collapsed: boolean) => void;
  /** Toggle collapse state for a specific column */
  toggleColumnCollapsed: (columnId: string) => void;
  /** Expand a specific column (shorthand for setColumnCollapsed(id, false)) */
  expandColumn: (columnId: string) => void;
  /** Replace the entire collapsed columns set */
  setCollapsedColumns: (columns: Set<string>) => void;
  /** Set the view to return to when leaving team split view */
  setPreviousView: (view: ViewType | null) => void;
  /** Atomically save old project state, restore new project state, clear ephemeral state */
  switchToProject: (oldProjectId: string | null, newProjectId: string) => void;
  /** Remove stale per-project route entries for a deleted project */
  cleanupProjectRoute: (projectId: string) => void;
  /** Update cached feature flags (called once on startup after Tauri command resolves) */
  setFeatureFlags: (flags: FeatureFlags) => void;
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
      maxConcurrent: 10,
      globalMaxConcurrent: 20,
      queuedCount: 0,
      canStartTask: true,
    },
    showArchived: false,
    showMergeTasks: loadShowMergeTasks(),
    boardSearchQuery: null,
    isSearching: false,
    selectedTaskId: null,
    graphSelection: null,
    graphRightPanelUserOpen: true,
    graphRightPanelCompactOpen: false,
    battleModeActive: false,
    battleModePanelRestoreState: null,
    taskHistoryState: null,
    taskCreationContext: null,
    chatVisibleByView: loadChatVisibility(),
    showWelcomeOverlay: false,
    welcomeOverlayReturnView: null,
    previousView: null,
    activityFilter: { taskId: null, sessionId: null },
    collapsedColumns: loadCollapsedColumns(),
    viewByProject: loadViewByProject(),
    sessionByProject: loadSessionByProject(),
    featureFlags: ALL_ENABLED_FLAGS,

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
        const safeView = isViewEnabled(view, state.featureFlags) ? view : "kanban";
        const projectId = useProjectStore.getState().activeProjectId;
        state.currentView = safeView;
        if (projectId) {
          state.viewByProject[projectId] = safeView;
          saveViewByProject(state.viewByProject);
        }
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

    setActiveQuestion: (sessionId, question) =>
      set((state) => {
        state.activeQuestions[sessionId] = question;
      }),

    clearActiveQuestion: (sessionId) =>
      set((state) => {
        delete state.activeQuestions[sessionId];
      }),

    dismissQuestion: (sessionId) =>
      set((state) => {
        delete state.activeQuestions[sessionId];
        delete state.answeredQuestions[sessionId];
      }),

    setAnsweredQuestion: (sessionId, summary) =>
      set((state) => {
        state.answeredQuestions[sessionId] = summary;
      }),

    clearAnsweredQuestion: (sessionId) =>
      set((state) => {
        delete state.answeredQuestions[sessionId];
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
        saveShowMergeTasks(show);
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

    enterBattleMode: () =>
      set((state) => {
        if (state.battleModeActive) return;
        state.battleModePanelRestoreState = {
          userOpen: state.graphRightPanelUserOpen,
          compactOpen: state.graphRightPanelCompactOpen,
        };
        state.battleModeActive = true;
        state.graphRightPanelUserOpen = false;
        state.graphRightPanelCompactOpen = false;
      }),

    exitBattleMode: () =>
      set((state) => {
        if (!state.battleModeActive) return;
        state.battleModeActive = false;
        if (state.battleModePanelRestoreState) {
          state.graphRightPanelUserOpen = state.battleModePanelRestoreState.userOpen;
          state.graphRightPanelCompactOpen = state.battleModePanelRestoreState.compactOpen;
        }
        state.battleModePanelRestoreState = null;
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

    setColumnCollapsed: (columnId, collapsed) =>
      set((state) => {
        if (collapsed) {
          state.collapsedColumns.add(columnId);
        } else {
          state.collapsedColumns.delete(columnId);
        }
        saveCollapsedColumns(state.collapsedColumns);
      }),

    toggleColumnCollapsed: (columnId) =>
      set((state) => {
        if (state.collapsedColumns.has(columnId)) {
          state.collapsedColumns.delete(columnId);
        } else {
          state.collapsedColumns.add(columnId);
        }
        saveCollapsedColumns(state.collapsedColumns);
      }),

    expandColumn: (columnId) =>
      set((state) => {
        state.collapsedColumns.delete(columnId);
        saveCollapsedColumns(state.collapsedColumns);
      }),

    setCollapsedColumns: (columns) =>
      set((state) => {
        state.collapsedColumns = columns;
        saveCollapsedColumns(state.collapsedColumns);
      }),

    setPreviousView: (view) =>
      set((state) => {
        state.previousView = view;
      }),

    switchToProject: (oldProjectId, newProjectId) =>
      set((state) => {
        // SAVE phase — skip if oldProjectId is null (first load)
        if (oldProjectId) {
          state.viewByProject[oldProjectId] = state.currentView;
          state.sessionByProject[oldProjectId] = useIdeationStore.getState().activeSessionId;
        }

        // RESTORE phase — resolve view, fallback ephemeral views to kanban
        let restoredView: ViewType = state.viewByProject[newProjectId] ?? "kanban";
        if (restoredView === "task_detail" || restoredView === "team") {
          restoredView = "kanban";
        }
        // Feature flag guard: redirect disabled views to kanban
        if (!isViewEnabled(restoredView, state.featureFlags)) {
          restoredView = "kanban";
        }

        // Persist updated maps
        saveViewByProject(state.viewByProject);
        saveSessionByProject(state.sessionByProject);

        // CLEAN + RESTORE (atomic)
        state.currentView = restoredView;
        state.selectedTaskId = null;
        state.graphSelection = null;
        state.taskHistoryState = null;
        state.boardSearchQuery = null;
        state.battleModeActive = false;
        state.battleModePanelRestoreState = null;
        state.activityFilter = { taskId: null, sessionId: null };
        state.graphRightPanelUserOpen = false;
        state.graphRightPanelCompactOpen = false;
      }),

    cleanupProjectRoute: (projectId) =>
      set((state) => {
        delete state.viewByProject[projectId];
        delete state.sessionByProject[projectId];
        saveViewByProject(state.viewByProject);
        saveSessionByProject(state.sessionByProject);
      }),

    setFeatureFlags: (flags) =>
      set((state) => {
        state.featureFlags = flags;
      }),
  }))
);

// Expose uiStore to window in web mode for Playwright testing
if (typeof window !== "undefined" && !window.__TAURI_INTERNALS__) {
  window.__uiStore = useUiStore;
}

// One-time feature flag initialization on module load.
// Zustand stores cannot use React hooks, so flags are fetched via invoke directly.
// Defaults to ALL_ENABLED until the async fetch resolves (prevents startup flash).
// Errors are silently ignored — all-enabled defaults remain active.
void invoke<unknown>("get_ui_feature_flags")
  .then((raw) => {
    const result = featureFlagsSchema.safeParse(raw);
    if (result.success) {
      useUiStore.getState().setFeatureFlags(result.data);
    }
  })
  .catch(() => {
    // Keep all-enabled defaults on error
  });
