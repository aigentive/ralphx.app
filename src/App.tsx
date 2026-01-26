/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { useMemo, useState, useEffect, useCallback, useRef } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { ReviewsPanel } from "@/components/reviews/ReviewsPanel";
import { ExecutionControlBar } from "@/components/execution/ExecutionControlBar";
import { AskUserQuestionModal } from "@/components/modals/AskUserQuestionModal";
import { TaskDetailModal } from "@/components/tasks/TaskDetailModal";
import { ChatPanel } from "@/components/Chat/ChatPanel";
import { PermissionDialog } from "@/components/PermissionDialog";
import { IdeationView } from "@/components/Ideation";
import { ExtensibilityView } from "@/components/ExtensibilityView";
import { ActivityView } from "@/components/activity";
import { SettingsView } from "@/components/settings";
import { ProjectSelector } from "@/components/projects/ProjectSelector";
import { ProjectCreationWizard } from "@/components/projects/ProjectCreationWizard";
import { useUiStore } from "@/stores/uiStore";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore, selectActiveSession } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import { useProjectStore } from "@/stores/projectStore";
import type { Task } from "@/types/task";
import type { ChatContext, ViewType } from "@/types/chat";
import type { ChatMessage as ChatMessageType, ApplyProposalsInput } from "@/types/ideation";
import type { CreateProject } from "@/types/project";
import { usePendingReviews } from "@/hooks/useReviews";
import { useTasks } from "@/hooks/useTasks";
import { useProjects } from "@/hooks/useProjects";
import {
  useIdeationSession,
  useCreateIdeationSession,
  useArchiveIdeationSession,
} from "@/hooks/useIdeation";
import { useProposalMutations } from "@/hooks/useProposals";
import { useApplyProposals } from "@/hooks/useApplyProposals";
import { useOrchestratorMessage } from "@/hooks/useOrchestrator";
import { api, getGitBranches } from "@/lib/tauri";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { AskUserQuestionResponse } from "@/types/ask-user-question";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  LayoutGrid,
  Lightbulb,
  Puzzle,
  Activity,
  SlidersHorizontal,
  MessageSquare,
  CheckCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";

// Local storage key for persisting chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-chat-panel-width";

const queryClient = getQueryClient();

// Temporary hardcoded IDs until project selection is implemented
const DEFAULT_PROJECT_ID = "demo-project";
const DEFAULT_WORKFLOW_ID = "ralphx-default";

// Navigation items configuration
const NAV_ITEMS: {
  view: ViewType;
  label: string;
  icon: React.ElementType;
  shortcut: string;
}[] = [
  { view: "kanban", label: "Kanban", icon: LayoutGrid, shortcut: "⌘1" },
  { view: "ideation", label: "Ideation", icon: Lightbulb, shortcut: "⌘2" },
  { view: "extensibility", label: "Extensibility", icon: Puzzle, shortcut: "⌘3" },
  { view: "activity", label: "Activity", icon: Activity, shortcut: "⌘4" },
  { view: "settings", label: "Settings", icon: SlidersHorizontal, shortcut: "⌘5" },
];

// Transform API messages to component-compatible format
function transformMessages(messages: Array<{ role: string; id: string; content: string; createdAt: string; sessionId: string | null; projectId: string | null; taskId: string | null; metadata: string | null; parentMessageId: string | null; conversationId?: string | null; toolCalls?: string | null }>): ChatMessageType[] {
  return messages.map((msg) => ({
    ...msg,
    role: (["user", "orchestrator", "system"].includes(msg.role) ? msg.role : "system") as "user" | "orchestrator" | "system",
    conversationId: msg.conversationId ?? null,
    toolCalls: msg.toolCalls ?? null,
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

  // Project state
  const projects = useProjectStore((s) => s.projects);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const setProjects = useProjectStore((s) => s.setProjects);
  const addProject = useProjectStore((s) => s.addProject);
  const selectProject = useProjectStore((s) => s.selectProject);

  // Fetch projects from backend
  const { data: fetchedProjects, isLoading: isLoadingProjects } = useProjects();

  // Project creation wizard state
  const [isProjectWizardOpen, setIsProjectWizardOpen] = useState(false);
  const [isCreatingProject, setIsCreatingProject] = useState(false);
  const [projectCreationError, setProjectCreationError] = useState<string | null>(null);

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

  // Check if we should show the empty state (no projects)
  const hasNoProjects = !isLoadingProjects && Object.keys(projects).length === 0;

  // Use active project ID or fallback for development
  const currentProjectId = activeProjectId ?? DEFAULT_PROJECT_ID;

  const { count: pendingReviewCount } = usePendingReviews(currentProjectId);
  const { data: tasks = [] } = useTasks(currentProjectId);

  // Ideation hooks
  const { data: sessionData, isLoading: isSessionLoading } = useIdeationSession(activeSession?.id ?? "");
  const createSession = useCreateIdeationSession();
  const archiveSession = useArchiveIdeationSession();
  const { toggleSelection, deleteProposal, reorder } = useProposalMutations();
  const { apply: applyProposalsMutation } = useApplyProposals();
  const orchestratorMessage = useOrchestratorMessage(activeSession?.id ?? "");

  // Seed builtin workflows on app startup
  useEffect(() => {
    api.workflows.seedBuiltin().catch((err) => {
      console.error("Failed to seed builtin workflows:", err);
    });
  }, []);

  // Sync fetched projects to store and auto-select first project
  useEffect(() => {
    if (fetchedProjects && fetchedProjects.length > 0) {
      setProjects(fetchedProjects);
      // Auto-select first project if none is selected
      if (!activeProjectId) {
        const firstProject = fetchedProjects[0];
        if (firstProject) {
          selectProject(firstProject.id);
        }
      }
    }
  }, [fetchedProjects, setProjects, activeProjectId, selectProject]);

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

  // Keyboard shortcuts for view switching (Cmd+1-5 for main views, Cmd+K for chat)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        switch (e.key) {
          case "1":
            e.preventDefault();
            setCurrentView("kanban");
            break;
          case "2":
            e.preventDefault();
            setCurrentView("ideation");
            break;
          case "3":
            e.preventDefault();
            setCurrentView("extensibility");
            break;
          case "4":
            e.preventDefault();
            setCurrentView("activity");
            break;
          case "5":
          case ".":
          case ",":
            // Cmd+5, Cmd+. or Cmd+, for settings (Cmd+, may not work in dev mode)
            e.preventDefault();
            setCurrentView("settings");
            break;
          case "k":
          case "K": {
            // Cmd+K to toggle chat panel (skip if in input/textarea)
            const activeElement = document.activeElement;
            if (
              activeElement instanceof HTMLInputElement ||
              activeElement instanceof HTMLTextAreaElement
            ) {
              return;
            }
            e.preventDefault();
            toggleChatPanel();
            break;
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setCurrentView, toggleChatPanel]);

  // Global shortcut for Cmd+, (registered at OS level to bypass DevTools interception)
  const setCurrentViewRef = useRef(setCurrentView);
  setCurrentViewRef.current = setCurrentView;

  useEffect(() => {
    const shortcut = "CommandOrControl+,";

    register(shortcut, () => {
      setCurrentViewRef.current("settings");
    }).catch((err) => {
      console.warn("Failed to register global shortcut:", err);
    });

    return () => {
      unregister(shortcut).catch(() => {
        // Ignore unregister errors on cleanup
      });
    };
  }, []);

  // Build chat context based on current view
  const chatContext: ChatContext = useMemo(() => {
    if (selectedTask) {
      return {
        view: "task_detail",
        projectId: currentProjectId,
        selectedTaskId: selectedTask.id,
      };
    }
    if (currentView === "ideation" && activeSession) {
      return {
        view: "ideation",
        projectId: currentProjectId,
        ideationSessionId: activeSession.id,
        selectedProposalIds: proposals.filter((p) => p.selected).map((p) => p.id),
      };
    }
    return {
      view: currentView,
      projectId: currentProjectId,
    };
  }, [selectedTask, currentView, activeSession, proposals, currentProjectId]);

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
        projectId: currentProjectId,
      });
      setActiveSession(session.id);
    } catch (error) {
      console.error("Failed to create session:", error);
    }
  }, [createSession, setActiveSession, currentProjectId]);

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

  // Project wizard handlers
  const handleOpenProjectWizard = useCallback(() => {
    setProjectCreationError(null);
    setIsProjectWizardOpen(true);
  }, []);

  const handleCloseProjectWizard = useCallback(() => {
    setIsProjectWizardOpen(false);
    setProjectCreationError(null);
  }, []);

  const handleCreateProject = useCallback(async (projectData: CreateProject) => {
    setIsCreatingProject(true);
    setProjectCreationError(null);
    try {
      // Call Tauri backend to create project
      const newProject = await api.projects.create(projectData);
      addProject(newProject);
      selectProject(newProject.id);
      setIsProjectWizardOpen(false);
    } catch (error) {
      console.error("Failed to create project:", error);
      setProjectCreationError(error instanceof Error ? error.message : "Failed to create project");
    } finally {
      setIsCreatingProject(false);
    }
  }, [addProject, selectProject]);

  const handleBrowseFolder = useCallback(async (): Promise<string | null> => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Select Project Folder",
      });
      // selected is string | string[] | null for directories
      if (typeof selected === "string") {
        return selected;
      }
      return null;
    } catch (error) {
      console.error("Failed to browse folder:", error);
      return null;
    }
  }, []);

  const handleFetchBranches = useCallback(async (workingDirectory: string): Promise<string[]> => {
    try {
      const branches = await getGitBranches(workingDirectory);
      return branches;
    } catch (error) {
      console.error("Failed to fetch branches:", error);
      return [];
    }
  }, []);

  return (
    <main
      className="h-screen flex flex-col overflow-hidden"
      style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}
    >
      {/* Header - Premium Design: Fixed 48px, shadow, Tauri drag region */}
      <TooltipProvider delayDuration={300}>
        <header
          className="fixed top-0 left-0 right-0 h-14 flex items-center justify-between pr-4 pl-24 border-b z-50 select-none"
          style={{
            backgroundColor: "var(--bg-surface)",
            borderColor: "var(--border-subtle)",
            boxShadow: "0 1px 3px rgba(0,0,0,0.1), 0 1px 2px rgba(0,0,0,0.06)",
          }}
          data-tauri-drag-region
          data-testid="app-header"
        >
          {/* Left Section: Branding + Navigation */}
          <div className="flex items-center gap-6">
            {/* App Branding */}
            <h1
              className="text-xl font-bold tracking-tight select-none"
              style={{ color: "var(--accent-primary)" }}
            >
              RalphX
            </h1>

            {/* View Navigation */}
            <nav
              className="flex items-center gap-1"
              role="navigation"
              aria-label="Main views"
              style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
            >
              {NAV_ITEMS.map(({ view, label, icon: Icon, shortcut }) => {
                const isActive = currentView === view;
                return (
                  <Tooltip key={view}>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setCurrentView(view)}
                        className={cn(
                          "gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                          // Compact on small screens, expanded on xl+
                          isActive ? "px-3" : "px-2 xl:px-3",
                          isActive
                            ? "bg-[var(--bg-elevated)] text-[var(--accent-primary)]"
                            : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
                        )}
                        data-testid={`nav-${view}`}
                        aria-current={isActive ? "page" : undefined}
                      >
                        <Icon className="w-[18px] h-[18px] flex-shrink-0" />
                        <span className={cn(
                          "text-sm font-medium whitespace-nowrap",
                          isActive ? "inline" : "hidden xl:inline"
                        )}>
                          {label}
                        </span>
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="text-xs">
                      {label} <kbd className="ml-1 opacity-70">{shortcut}</kbd>
                    </TooltipContent>
                  </Tooltip>
                );
              })}
            </nav>
          </div>

          {/* Spacer */}
          <div className="flex-1" />

          {/* Right Section: Project Selector + Panel Toggles */}
          <div
            className="flex items-center gap-2"
            style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
          >
            {/* Project selector - always aligned end */}
            <div className="mr-2">
              <ProjectSelector onNewProject={handleOpenProjectWizard} align="end" />
            </div>
            {/* Chat Panel Toggle */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleChatPanel}
                  className={cn(
                    "gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                    chatIsOpen ? "px-3" : "px-2 xl:px-3",
                    chatIsOpen
                      ? "bg-[var(--bg-elevated)] text-[var(--accent-primary)]"
                      : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
                  )}
                  data-testid="chat-toggle"
                >
                  <MessageSquare className="w-[18px] h-[18px] flex-shrink-0" />
                  <span className={cn(
                    "text-sm font-medium whitespace-nowrap",
                    chatIsOpen ? "inline" : "hidden xl:inline"
                  )}>
                    Chat
                  </span>
                  <kbd
                    className={cn(
                      "ml-1 px-1.5 py-0.5 text-xs rounded",
                      chatIsOpen ? "inline" : "hidden xl:inline"
                    )}
                    style={{
                      backgroundColor: "var(--bg-elevated)",
                      color: "var(--text-muted)",
                    }}
                  >
                    ⌘K
                  </kbd>
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                Toggle Chat <kbd className="ml-1 opacity-70">⌘K</kbd>
              </TooltipContent>
            </Tooltip>

            {/* Reviews Panel Toggle */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleReviewsPanel}
                  className={cn(
                    "relative gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                    reviewsPanelOpen ? "px-3" : "px-2 xl:px-3",
                    reviewsPanelOpen
                      ? "bg-[var(--bg-elevated)] text-[var(--accent-primary)]"
                      : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
                  )}
                  data-testid="reviews-toggle"
                >
                  <CheckCircle className="w-[18px] h-[18px] flex-shrink-0" />
                  <span className={cn(
                    "text-sm font-medium whitespace-nowrap",
                    reviewsPanelOpen ? "inline" : "hidden xl:inline"
                  )}>
                    Reviews
                  </span>
                  {/* Badge with pending count */}
                  {pendingReviewCount > 0 && (
                    <span
                      className="absolute -top-1 -right-1 flex items-center justify-center min-w-[18px] h-[18px] px-1 text-xs font-bold rounded-full animate-badge-pop"
                      style={{
                        backgroundColor: "var(--status-warning)",
                        color: "white",
                      }}
                      data-testid="reviews-badge"
                    >
                      {pendingReviewCount > 9 ? "9+" : pendingReviewCount}
                    </span>
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                Toggle Reviews Panel
              </TooltipContent>
            </Tooltip>
          </div>
        </header>
      </TooltipProvider>

      {/* Spacer for fixed header */}
      <div className="h-14 flex-shrink-0" />

      {/* Main content area - shows empty state wizard or normal content */}
      {hasNoProjects ? (
        /* Empty state: centered project creation wizard */
        <div
          className="flex-1 flex items-center justify-center"
          data-testid="empty-state"
        >
          <div className="text-center">
            <h2
              className="text-xl font-semibold mb-2"
              style={{ color: "var(--text-primary)" }}
            >
              Welcome to RalphX
            </h2>
            <p
              className="text-sm mb-6"
              style={{ color: "var(--text-muted)" }}
            >
              Get started by creating your first project
            </p>
            <button
              onClick={handleOpenProjectWizard}
              className="px-6 py-3 rounded-lg text-sm font-medium transition-colors"
              style={{
                backgroundColor: "var(--accent-primary)",
                color: "#fff",
              }}
              data-testid="create-first-project-button"
            >
              Create Project
            </button>
          </div>
        </div>
      ) : (
        /* Normal content with view-specific content and optional panels */
        <div className="flex-1 flex overflow-hidden">
          {/* Main view area */}
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-hidden h-full">
              {currentView === "kanban" && (
                <TaskBoard
                  projectId={currentProjectId}
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
              {currentView === "activity" && <ActivityView showHeader />}
              {currentView === "settings" && <SettingsView />}
            </div>
            {/* ExecutionControlBar at bottom (only show in kanban view) */}
            {currentView === "kanban" && (
              <div className="flex-shrink-0 p-4 border-t" style={{ borderColor: "var(--border-subtle)" }}>
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
                projectId={currentProjectId}
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
      )}

      {/* AskUserQuestionModal - renders when activeQuestion is set */}
      <AskUserQuestionModal
        question={activeQuestion}
        onSubmit={handleQuestionSubmit}
        onClose={handleQuestionClose}
        isLoading={isQuestionLoading}
      />

      {/* TaskDetailModal - renders when task-detail modal is active */}
      <TaskDetailModal
        task={selectedTask}
        isOpen={!!selectedTask}
        onClose={closeModal}
      />

      {/* Project Creation Wizard */}
      <ProjectCreationWizard
        isOpen={isProjectWizardOpen}
        onClose={handleCloseProjectWizard}
        onCreate={handleCreateProject}
        onBrowseFolder={handleBrowseFolder}
        onFetchBranches={handleFetchBranches}
        isCreating={isCreatingProject}
        error={projectCreationError}
        isFirstRun={hasNoProjects}
      />

      {/* Permission Dialog - Global UI-based permission approval */}
      <PermissionDialog />
    </main>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <AppContent />
      </EventProvider>
    </QueryClientProvider>
  );
}

export default App;
