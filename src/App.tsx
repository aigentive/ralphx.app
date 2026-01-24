/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { useMemo, useState, useEffect, useCallback } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { ReviewsPanel } from "@/components/reviews/ReviewsPanel";
import { ExecutionControlBar } from "@/components/execution/ExecutionControlBar";
import { AskUserQuestionModal } from "@/components/modals/AskUserQuestionModal";
import { TaskDetailView } from "@/components/tasks/TaskDetailView";
import { ChatPanel } from "@/components/Chat/ChatPanel";
import { IdeationView } from "@/components/Ideation";
import { ExtensibilityView } from "@/components/ExtensibilityView";
import { useUiStore } from "@/stores/uiStore";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore, selectActiveSession } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import type { Task } from "@/types/task";
import type { ChatContext } from "@/types/chat";
import type { ChatMessage as ChatMessageType, ApplyProposalsInput } from "@/types/ideation";
import { usePendingReviews } from "@/hooks/useReviews";
import { useTasks } from "@/hooks/useTasks";
import {
  useIdeationSession,
  useCreateIdeationSession,
  useArchiveIdeationSession,
} from "@/hooks/useIdeation";
import { useProposalMutations } from "@/hooks/useProposals";
import { useApplyProposals } from "@/hooks/useApplyProposals";
import { useOrchestratorMessage } from "@/hooks/useOrchestrator";
import { api } from "@/lib/tauri";
import type { AskUserQuestionResponse } from "@/types/ask-user-question";

// Local storage key for persisting chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-chat-panel-width";

const queryClient = getQueryClient();

// Temporary hardcoded IDs until project selection is implemented
const DEFAULT_PROJECT_ID = "demo-project";
const DEFAULT_WORKFLOW_ID = "ralphx-default";

function ReviewIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M10 18a8 8 0 100-16 8 8 0 000 16z"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path
        d="M7 10l2 2 4-4"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ChatIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M3 5a2 2 0 012-2h10a2 2 0 012 2v8a2 2 0 01-2 2H8l-4 3v-3H5a2 2 0 01-2-2V5z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M7 7h6M7 10h4"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

function KanbanIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <rect x="3" y="3" width="4" height="14" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <rect x="8" y="3" width="4" height="10" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <rect x="13" y="3" width="4" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

function IdeationIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M10 2a6 6 0 016 6c0 2.22-1.21 4.16-3 5.19V15a2 2 0 01-2 2H9a2 2 0 01-2-2v-1.81C5.21 12.16 4 10.22 4 8a6 6 0 016-6z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M8 18h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M10 12.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5z"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path
        d="M16.2 10a6.2 6.2 0 01-.1 1.2l2.1 1.6a.5.5 0 01.1.6l-2 3.5a.5.5 0 01-.6.2l-2.5-1a6.5 6.5 0 01-2.1 1.2l-.4 2.6a.5.5 0 01-.5.4h-4a.5.5 0 01-.5-.4l-.4-2.6a6.5 6.5 0 01-2.1-1.2l-2.5 1a.5.5 0 01-.6-.2l-2-3.5a.5.5 0 01.1-.6l2.1-1.6a6.2 6.2 0 010-2.4L.6 5.7a.5.5 0 01-.1-.6l2-3.5a.5.5 0 01.6-.2l2.5 1a6.5 6.5 0 012.1-1.2l.4-2.6a.5.5 0 01.5-.4h4a.5.5 0 01.5.4l.4 2.6a6.5 6.5 0 012.1 1.2l2.5-1a.5.5 0 01.6.2l2 3.5a.5.5 0 01-.1.6l-2.1 1.6c.1.4.1.8.1 1.2z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// Transform API messages to component-compatible format
function transformMessages(messages: Array<{ role: string; id: string; content: string; createdAt: string; sessionId: string | null; projectId: string | null; taskId: string | null; metadata: string | null; parentMessageId: string | null }>): ChatMessageType[] {
  return messages.map((msg) => ({
    ...msg,
    role: (["user", "orchestrator", "system"].includes(msg.role) ? msg.role : "system") as "user" | "orchestrator" | "system",
  }));
}

function AppContent() {
  const reviewsPanelOpen = useUiStore((s) => s.reviewsPanelOpen);
  const toggleReviewsPanel = useUiStore((s) => s.toggleReviewsPanel);
  const setReviewsPanelOpen = useUiStore((s) => s.setReviewsPanelOpen);
  const executionStatus = useUiStore((s) => s.executionStatus);
  const setExecutionStatus = useUiStore((s) => s.setExecutionStatus);
  const activeQuestion = useUiStore((s) => s.activeQuestion);
  const clearActiveQuestion = useUiStore((s) => s.clearActiveQuestion);
  const activeModal = useUiStore((s) => s.activeModal);
  const modalContext = useUiStore((s) => s.modalContext);
  const closeModal = useUiStore((s) => s.closeModal);
  const currentView = useUiStore((s) => s.currentView);
  const setCurrentView = useUiStore((s) => s.setCurrentView);

  // Chat panel state
  const chatIsOpen = useChatStore((s) => s.isOpen);
  const chatWidth = useChatStore((s) => s.width);
  const toggleChatPanel = useChatStore((s) => s.togglePanel);
  const setChatWidth = useChatStore((s) => s.setWidth);

  // Ideation state
  const activeSession = useIdeationStore(selectActiveSession);
  const setActiveSession = useIdeationStore((s) => s.setActiveSession);
  const activeSessionId = activeSession?.id ?? "";
  // Get raw proposals from store and memoize the filtered/sorted version
  const allProposals = useProposalStore((s) => s.proposals);
  const proposals = useMemo(() => {
    if (!activeSessionId) return [];
    return Object.values(allProposals)
      .filter((p) => p.sessionId === activeSessionId)
      .sort((a, b) => a.sortOrder - b.sortOrder);
  }, [allProposals, activeSessionId]);

  // Extract task from modal context for task-detail modal
  const selectedTask = activeModal === "task-detail" && modalContext?.task
    ? (modalContext.task as Task)
    : null;

  const [isExecutionLoading, setIsExecutionLoading] = useState(false);
  const [isQuestionLoading, setIsQuestionLoading] = useState(false);

  const { count: pendingReviewCount } = usePendingReviews(DEFAULT_PROJECT_ID);
  const { data: tasks = [] } = useTasks(DEFAULT_PROJECT_ID);

  // Ideation hooks
  const { data: sessionData, isLoading: isSessionLoading } = useIdeationSession(activeSession?.id ?? "");
  const createSession = useCreateIdeationSession();
  const archiveSession = useArchiveIdeationSession();
  const { toggleSelection, deleteProposal, reorder } = useProposalMutations();
  const { apply: applyProposalsMutation } = useApplyProposals();
  const orchestratorMessage = useOrchestratorMessage(activeSession?.id ?? "");

  // Load persisted chat width from localStorage on mount
  useEffect(() => {
    const savedWidth = localStorage.getItem(CHAT_WIDTH_STORAGE_KEY);
    if (savedWidth) {
      const width = parseInt(savedWidth, 10);
      if (!isNaN(width)) {
        setChatWidth(width);
      }
    }
  }, [setChatWidth]);

  // Persist chat width to localStorage when it changes
  useEffect(() => {
    localStorage.setItem(CHAT_WIDTH_STORAGE_KEY, chatWidth.toString());
  }, [chatWidth]);

  // Keyboard shortcuts for view switching (Cmd+1 for Kanban, Cmd+2 for Ideation, Cmd+3 for Extensibility)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key === "1") {
          e.preventDefault();
          setCurrentView("kanban");
        } else if (e.key === "2") {
          e.preventDefault();
          setCurrentView("ideation");
        } else if (e.key === "3") {
          e.preventDefault();
          setCurrentView("extensibility");
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setCurrentView]);

  // Build chat context based on current view
  const chatContext: ChatContext = useMemo(() => {
    if (selectedTask) {
      return {
        view: "task_detail",
        projectId: DEFAULT_PROJECT_ID,
        selectedTaskId: selectedTask.id,
      };
    }
    if (currentView === "ideation" && activeSession) {
      return {
        view: "ideation",
        projectId: DEFAULT_PROJECT_ID,
        ideationSessionId: activeSession.id,
        selectedProposalIds: proposals.filter((p) => p.selected).map((p) => p.id),
      };
    }
    return {
      view: currentView,
      projectId: DEFAULT_PROJECT_ID,
    };
  }, [selectedTask, currentView, activeSession, proposals]);

  const handlePauseToggle = async () => {
    setIsExecutionLoading(true);
    try {
      const response = executionStatus.isPaused
        ? await api.execution.resume()
        : await api.execution.pause();
      setExecutionStatus(response.status);
    } catch (error) {
      console.error("Failed to toggle pause:", error);
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleStop = async () => {
    setIsExecutionLoading(true);
    try {
      const response = await api.execution.stop();
      setExecutionStatus(response.status);
    } catch (error) {
      console.error("Failed to stop execution:", error);
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleQuestionSubmit = async (response: AskUserQuestionResponse) => {
    setIsQuestionLoading(true);
    try {
      console.log("Submit answer:", response);
      // TODO: Call Tauri command to submit answer and trigger BlockersResolved event
      clearActiveQuestion();
    } catch (error) {
      console.error("Failed to submit answer:", error);
    } finally {
      setIsQuestionLoading(false);
    }
  };

  const handleQuestionClose = () => {
    // Close without submitting - question remains unanswered
    clearActiveQuestion();
  };

  // Ideation handlers
  const handleNewSession = useCallback(async () => {
    try {
      const session = await createSession.mutateAsync({
        projectId: DEFAULT_PROJECT_ID,
      });
      setActiveSession(session.id);
    } catch (error) {
      console.error("Failed to create session:", error);
    }
  }, [createSession, setActiveSession]);

  const handleArchiveSession = useCallback(async (sessionId: string) => {
    try {
      await archiveSession.mutateAsync(sessionId);
      setActiveSession(null);
    } catch (error) {
      console.error("Failed to archive session:", error);
    }
  }, [archiveSession, setActiveSession]);

  const handleSendIdeationMessage = useCallback(async (content: string) => {
    if (!activeSession) return;
    try {
      await orchestratorMessage.mutateAsync(content);
    } catch (error) {
      console.error("Failed to send orchestrator message:", error);
    }
  }, [activeSession, orchestratorMessage]);

  const handleSelectProposal = useCallback((proposalId: string) => {
    toggleSelection.mutate(proposalId);
  }, [toggleSelection]);

  const handleEditProposal = useCallback((proposalId: string) => {
    // Open edit modal for proposal
    console.log("Edit proposal:", proposalId);
  }, []);

  const handleRemoveProposal = useCallback((proposalId: string) => {
    deleteProposal.mutate(proposalId);
  }, [deleteProposal]);

  const handleReorderProposals = useCallback((proposalIds: string[]) => {
    if (activeSession) {
      reorder.mutate({ sessionId: activeSession.id, proposalIds });
    }
  }, [activeSession, reorder]);

  const handleApplyProposals = useCallback(async (options: ApplyProposalsInput) => {
    try {
      await applyProposalsMutation.mutateAsync(options);
    } catch (error) {
      console.error("Failed to apply proposals:", error);
    }
  }, [applyProposalsMutation]);

  // Build task titles lookup
  const taskTitles = useMemo(() => {
    const titles: Record<string, string> = {};
    for (const task of tasks) {
      titles[task.id] = task.title;
    }
    return titles;
  }, [tasks]);

  return (
    <main
      className="min-h-screen flex flex-col"
      style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}
    >
      {/* Header */}
      <header
        className="flex items-center justify-between p-4 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <div className="flex items-center gap-4">
          <h1
            className="text-xl font-bold"
            style={{ color: "var(--accent-primary)" }}
          >
            RalphX
          </h1>
          {/* View Navigation */}
          <nav className="flex items-center gap-1" role="navigation" aria-label="Main views">
            <button
              onClick={() => setCurrentView("kanban")}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
              style={{
                backgroundColor: currentView === "kanban"
                  ? "var(--bg-elevated)"
                  : "transparent",
                color: currentView === "kanban"
                  ? "var(--accent-primary)"
                  : "var(--text-secondary)",
              }}
              data-testid="nav-kanban"
              aria-current={currentView === "kanban" ? "page" : undefined}
              title="Kanban (⌘1)"
            >
              <KanbanIcon />
              <span className="text-sm font-medium">Kanban</span>
            </button>
            <button
              onClick={() => setCurrentView("ideation")}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
              style={{
                backgroundColor: currentView === "ideation"
                  ? "var(--bg-elevated)"
                  : "transparent",
                color: currentView === "ideation"
                  ? "var(--accent-primary)"
                  : "var(--text-secondary)",
              }}
              data-testid="nav-ideation"
              aria-current={currentView === "ideation" ? "page" : undefined}
              title="Ideation (⌘2)"
            >
              <IdeationIcon />
              <span className="text-sm font-medium">Ideation</span>
            </button>
            <button
              onClick={() => setCurrentView("extensibility")}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
              style={{
                backgroundColor: currentView === "extensibility"
                  ? "var(--bg-elevated)"
                  : "transparent",
                color: currentView === "extensibility"
                  ? "var(--accent-primary)"
                  : "var(--text-secondary)",
              }}
              data-testid="nav-extensibility"
              aria-current={currentView === "extensibility" ? "page" : undefined}
              title="Extensibility (⌘3)"
            >
              <GearIcon />
              <span className="text-sm font-medium">Extensibility</span>
            </button>
          </nav>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-sm" style={{ color: "var(--text-muted)" }}>
            Demo Project
          </span>
          {/* Chat Panel Toggle */}
          <button
            onClick={toggleChatPanel}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
            style={{
              backgroundColor: chatIsOpen
                ? "var(--bg-elevated)"
                : "transparent",
              color: chatIsOpen
                ? "var(--accent-primary)"
                : "var(--text-secondary)",
            }}
            data-testid="chat-toggle"
            title="Toggle Chat (⌘K)"
          >
            <ChatIcon />
            <span className="text-sm font-medium">Chat</span>
            <kbd
              className="ml-1 px-1 py-0.5 text-xs rounded"
              style={{
                backgroundColor: "var(--bg-elevated)",
                color: "var(--text-muted)",
              }}
            >
              ⌘K
            </kbd>
          </button>
          {/* Reviews Panel Toggle */}
          <button
            onClick={toggleReviewsPanel}
            className="relative flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
            style={{
              backgroundColor: reviewsPanelOpen
                ? "var(--bg-elevated)"
                : "transparent",
              color: reviewsPanelOpen
                ? "var(--accent-primary)"
                : "var(--text-secondary)",
            }}
            data-testid="reviews-toggle"
          >
            <ReviewIcon />
            <span className="text-sm font-medium">Reviews</span>
            {/* Badge with pending count */}
            {pendingReviewCount > 0 && (
              <span
                className="absolute -top-1 -right-1 flex items-center justify-center w-5 h-5 text-xs font-bold rounded-full"
                style={{
                  backgroundColor: "var(--status-review)",
                  color: "white",
                }}
                data-testid="reviews-badge"
              >
                {pendingReviewCount > 9 ? "9+" : pendingReviewCount}
              </span>
            )}
          </button>
        </div>
      </header>

      {/* Main content area with view-specific content and optional panels */}
      <div className="flex-1 flex overflow-hidden">
        {/* Main view area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 overflow-hidden">
            {currentView === "kanban" && (
              <TaskBoard
                projectId={DEFAULT_PROJECT_ID}
                workflowId={DEFAULT_WORKFLOW_ID}
              />
            )}
            {currentView === "ideation" && (
              <IdeationView
                session={activeSession}
                messages={transformMessages(sessionData?.messages ?? [])}
                proposals={proposals}
                onSendMessage={handleSendIdeationMessage}
                onNewSession={handleNewSession}
                onArchiveSession={handleArchiveSession}
                onSelectProposal={handleSelectProposal}
                onEditProposal={handleEditProposal}
                onRemoveProposal={handleRemoveProposal}
                onReorderProposals={handleReorderProposals}
                onApply={handleApplyProposals}
                isLoading={isSessionLoading || createSession.isPending || archiveSession.isPending || applyProposalsMutation.isPending || orchestratorMessage.isPending}
              />
            )}
            {currentView === "extensibility" && <ExtensibilityView />}
          </div>
          {/* ExecutionControlBar at bottom (only show in kanban view) */}
          {currentView === "kanban" && (
            <div className="p-4 border-t" style={{ borderColor: "var(--border-subtle)" }}>
              <ExecutionControlBar
                runningCount={executionStatus.runningCount}
                maxConcurrent={executionStatus.maxConcurrent}
                queuedCount={executionStatus.queuedCount}
                isPaused={executionStatus.isPaused}
                isLoading={isExecutionLoading}
                onPauseToggle={handlePauseToggle}
                onStop={handleStop}
              />
            </div>
          )}
        </div>

        {/* ReviewsPanel slide-out */}
        {reviewsPanelOpen && (
          <div
            className="w-96 border-l flex-shrink-0"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <ReviewsPanel
              projectId={DEFAULT_PROJECT_ID}
              taskTitles={taskTitles}
              onClose={() => setReviewsPanelOpen(false)}
              onApprove={(reviewId) => {
                console.log("Approve review:", reviewId);
                // TODO: Call approveReview mutation
              }}
              onRequestChanges={(reviewId) => {
                console.log("Request changes for review:", reviewId);
                // TODO: Open request changes modal
              }}
              onViewDiff={(reviewId) => {
                console.log("View diff for review:", reviewId);
                // TODO: Open diff viewer
              }}
            />
          </div>
        )}

        {/* ChatPanel - resizable side panel with Cmd+K toggle */}
        <ChatPanel context={chatContext} />
      </div>

      {/* AskUserQuestionModal - renders when activeQuestion is set */}
      <AskUserQuestionModal
        question={activeQuestion}
        onSubmit={handleQuestionSubmit}
        onClose={handleQuestionClose}
        isLoading={isQuestionLoading}
      />

      {/* TaskDetailView Modal - renders when task-detail modal is active */}
      {selectedTask && (
        <div
          data-testid="task-detail-modal"
          className="fixed inset-0 z-50 flex items-center justify-center"
        >
          <div
            className="absolute inset-0"
            style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
            onClick={closeModal}
          />
          <div
            className="relative w-full max-w-2xl max-h-[80vh] overflow-auto m-4"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              onClick={closeModal}
              className="absolute top-2 right-2 p-1 rounded hover:bg-black/10 z-10"
              style={{ color: "var(--text-secondary)" }}
              data-testid="task-detail-close"
            >
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M15 5L5 15M5 5l10 10" />
              </svg>
            </button>
            <TaskDetailView task={selectedTask} />
          </div>
        </div>
      )}
    </main>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <AppContent />
      </EventProvider>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

export default App;
