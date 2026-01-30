/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { useMemo, useState, useEffect, useCallback } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { toast } from "sonner";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { ReviewsPanel } from "@/components/reviews/ReviewsPanel";
import { ExecutionControlBar } from "@/components/execution/ExecutionControlBar";
import { AskUserQuestionModal } from "@/components/modals/AskUserQuestionModal";
import { TaskDetailModal } from "@/components/tasks/TaskDetailModal";
import { TaskFullView } from "@/components/tasks/TaskFullView";
import { ChatPanel } from "@/components/Chat/ChatPanel";
import { KanbanSplitLayout, Navigation } from "@/components/layout";
import { PermissionDialog } from "@/components/PermissionDialog";
import { IdeationView, ProposalEditModal } from "@/components/Ideation";
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
import type { ChatContext } from "@/types/chat";
import type { ApplyProposalsInput } from "@/types/ideation";
import { toTaskProposal } from "@/api/ideation";
import type { CreateProject } from "@/types/project";
import { usePendingReviews } from "@/hooks/useReviews";
import { useReviewMutations } from "@/hooks/useReviewMutations";
import { useExecutionEvents } from "@/hooks/useExecutionEvents";
import { useTasks } from "@/hooks/useTasks";
import { useProjects } from "@/hooks/useProjects";
import {
  useIdeationSession,
  useIdeationSessions,
  useCreateIdeationSession,
  useArchiveIdeationSession,
  useDeleteIdeationSession,
} from "@/hooks/useIdeation";
import { useConfirmation } from "@/hooks/useConfirmation";
import { useProposalMutations } from "@/hooks/useProposals";
import { useApplyProposals } from "@/hooks/useApplyProposals";
import { useAppKeyboardShortcuts } from "@/hooks/useAppKeyboardShortcuts";
import { api, getGitBranches } from "@/lib/tauri";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  MessageSquare,
  CheckCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Toaster } from "@/components/ui/sonner";

// Local storage key for persisting chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-chat-panel-width";

const queryClient = getQueryClient();

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
  const taskFullViewId = useUiStore((s) => s.taskFullViewId);
  const closeTaskFullView = useUiStore((s) => s.closeTaskFullView);
  // Split layout chat state (for kanban view)
  const chatCollapsed = useUiStore((s) => s.chatCollapsed);
  const toggleChatCollapsed = useUiStore((s) => s.toggleChatCollapsed);


  // Chat panel state (for non-kanban views)
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
  const addSession = useIdeationStore((s) => s.addSession);
  const activeSessionId = activeSession?.id ?? "";
  // Get raw proposals from store and memoize the filtered/sorted version
  const allProposals = useProposalStore((s) => s.proposals);
  const setProposals = useProposalStore((s) => s.setProposals);
  const proposals = useMemo(() => {
    if (!activeSessionId) return [];
    return Object.values(allProposals)
      .filter((p) => p.sessionId === activeSessionId)
      .sort((a, b) => a.sortOrder - b.sortOrder);
  }, [allProposals, activeSessionId]);
  const [editingProposalId, setEditingProposalId] = useState<string | null>(null);
  const editingProposal = editingProposalId
    ? allProposals[editingProposalId] ?? null
    : null;

  // Extract task from modal context for task-detail modal
  const selectedTask = activeModal === "task-detail" && modalContext?.task
    ? (modalContext.task as Task)
    : null;

  const [isExecutionLoading, setIsExecutionLoading] = useState(false);
  const [isQuestionLoading, setIsQuestionLoading] = useState(false);

  // Check if we should show the empty state (no projects)
  const hasNoProjects = !isLoadingProjects && Object.keys(projects).length === 0;

  // Use active project ID (queries are disabled when null)
  const currentProjectId = activeProjectId ?? "";

  const { count: pendingReviewCount } = usePendingReviews(currentProjectId);
  const { data: tasks = [] } = useTasks(currentProjectId);

  // Real-time execution status updates via Tauri events
  useExecutionEvents();
  const { approve: approveReview, requestChanges: requestChangesReview, isApproving, isRequestingChanges } = useReviewMutations();

  // Ideation hooks
  const { data: sessionData } = useIdeationSession(activeSession?.id ?? "");
  const { data: allSessions = [] } = useIdeationSessions(currentProjectId);
  const createSession = useCreateIdeationSession();
  const archiveSession = useArchiveIdeationSession();
  const deleteSession = useDeleteIdeationSession();
  const { confirm, ConfirmationDialog } = useConfirmation();
  const { toggleSelection, deleteProposal, reorder, updateProposal } = useProposalMutations();
  const { apply: applyProposalsMutation } = useApplyProposals();

  // Sync proposals from sessionData to the store
  useEffect(() => {
    if (sessionData?.proposals) {
      // Convert API response to store type using proper mapping function
      setProposals(sessionData.proposals.map(toTaskProposal));
    }
  }, [sessionData?.proposals, setProposals]);


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

  // Keyboard shortcuts for view switching and chat toggle
  useAppKeyboardShortcuts({
    currentView,
    setCurrentView,
    toggleChatPanel,
    toggleChatCollapsed,
  });

  // Build chat context based on current view
  const chatContext: ChatContext = useMemo(() => {
    if (selectedTask) {
      return {
        view: "task_detail",
        projectId: currentProjectId,
        selectedTaskId: selectedTask.id,
      };
    }
    if (currentView === "ideation") {
      if (activeSession) {
        return {
          view: "ideation",
          projectId: currentProjectId,
          ideationSessionId: activeSession.id,
          selectedProposalIds: proposals.filter((p) => p.selected).map((p) => p.id),
        };
      }
      // No session yet - fall back to project context for chat
      return {
        view: "kanban",
        projectId: currentProjectId,
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
    } catch {
      toast.error(
        executionStatus.isPaused
          ? "Failed to resume execution"
          : "Failed to pause execution"
      );
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleStop = async () => {
    setIsExecutionLoading(true);
    try {
      const response = await api.execution.stop();
      setExecutionStatus(response.status);
    } catch {
      toast.error("Failed to stop execution");
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleQuestionSubmit = async () => {
    setIsQuestionLoading(true);
    try {
      clearActiveQuestion();
    } catch {
      toast.error("Failed to submit answer");
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
      // Add session to store immediate (don't wait for refetch)
      addSession(session);
      setActiveSession(session.id);
    } catch {
      toast.error("Failed to create new session");
    }
  }, [createSession, addSession, setActiveSession, currentProjectId]);

  const handleArchiveSession = useCallback(async (sessionId: string) => {
    try {
      await archiveSession.mutateAsync(sessionId);
      setActiveSession(null);
    } catch {
      toast.error("Failed to archive session");
    }
  }, [archiveSession, setActiveSession]);

  const handleDeleteSession = useCallback(async (sessionId: string) => {
    const sessionToDelete = allSessions.find(s => s.id === sessionId);

    const confirmed = await confirm({
      title: "Delete session?",
      description: `This will permanently delete "${sessionToDelete?.title || 'this session'}" and all its messages. This action cannot be undone.`,
      confirmText: "Delete",
      variant: "destructive",
    });

    if (!confirmed) return;

    try {
      await deleteSession.mutateAsync(sessionId);
      if (activeSession?.id === sessionId) {
        setActiveSession(null);
      }
      toast.success("Session deleted");
    } catch {
      toast.error("Failed to delete session");
    }
  }, [deleteSession, confirm, allSessions, activeSession, setActiveSession]);

  const handleSelectSession = useCallback((sessionId: string) => {
    // Find the session in allSessions and add to store if not already there
    const session = allSessions.find((s) => s.id === sessionId);
    if (session) {
      addSession(session);
      setActiveSession(sessionId);
    }
  }, [allSessions, addSession, setActiveSession]);

  const handleSelectProposal = useCallback((proposalId: string) => {
    toggleSelection.mutate(proposalId);
  }, [toggleSelection]);

  const handleEditProposal = useCallback((proposalId: string) => {
    setEditingProposalId(proposalId);
  }, []);

  const handleSaveProposal = useCallback(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    async (proposalId: string, data: any) => {
      try {
        await updateProposal.mutateAsync({ proposalId, changes: data });
        setEditingProposalId(null);
        toast.success("Proposal updated");
      } catch {
        toast.error("Failed to update proposal");
      }
    },
    [updateProposal]
  );

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
    } catch {
      toast.error("Failed to apply proposals");
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
    } catch {
      return null;
    }
  }, []);

  const handleFetchBranches = useCallback(async (workingDirectory: string): Promise<string[]> => {
    try {
      const branches = await getGitBranches(workingDirectory);
      return branches;
    } catch {
      return [];
    }
  }, []);

  return (
    <main
      className="h-screen flex flex-col overflow-hidden"
      style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}
    >
      {/* Header - macOS Tahoe Liquid Glass */}
      <TooltipProvider delayDuration={300}>
        <header
          className="fixed top-0 left-0 right-0 h-14 flex items-center justify-between pr-4 pl-24 border-b z-50 select-none"
          style={{
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(24px)",
            WebkitBackdropFilter: "blur(24px)",
            borderColor: "rgba(255,255,255,0.06)",
            boxShadow: "0 1px 0 rgba(255,255,255,0.03)",
          }}
          data-tauri-drag-region
          data-testid="app-header"
        >
          {/* Left Section: Branding + Navigation */}
          <div className="flex items-center gap-6">
            {/* App Branding */}
            <h1
              className="text-xl font-bold tracking-tight select-none"
              style={{ color: "#ff6b35" }}
            >
              RalphX
            </h1>

            {/* View Navigation */}
            <Navigation currentView={currentView} onViewChange={setCurrentView} />
          </div>

          {/* Spacer */}
          <div className="flex-1" />

          {/* Right Section: Project Selector + Panel Toggles */}
          <div
            className="flex items-center gap-2"
            style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
          >
            {/* Project selector */}
            <div className="mr-2">
              <ProjectSelector onNewProject={handleOpenProjectWizard} align="end" />
            </div>
            {/* Chat Panel Toggle - hidden on ideation (has built-in chat) */}
            {currentView !== "ideation" && (() => {
              // For kanban view, chat is always visible but can be collapsed
              // For other views, chat panel can be completely closed
              const isExpanded = currentView === "kanban" ? !chatCollapsed : chatIsOpen;
              const handleToggle = currentView === "kanban" ? toggleChatCollapsed : toggleChatPanel;

              return (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleToggle}
                      className={cn(
                        "gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                        isExpanded ? "px-3" : "px-2 xl:px-3"
                      )}
                      style={{
                        background: isExpanded
                          ? "rgba(255,107,53,0.1)"
                          : "transparent",
                        border: isExpanded ? "1px solid rgba(255,107,53,0.15)" : "1px solid transparent",
                        color: isExpanded ? "#ff6b35" : "rgba(255,255,255,0.5)",
                      }}
                      data-testid="chat-toggle"
                    >
                      <MessageSquare className="w-[18px] h-[18px] flex-shrink-0" />
                      <span className={cn(
                        "text-sm font-medium whitespace-nowrap",
                        isExpanded ? "inline" : "hidden xl:inline"
                      )}>
                        Chat
                      </span>
                      <kbd
                        className={cn(
                          "ml-1 px-1.5 py-0.5 text-xs rounded",
                          isExpanded ? "inline" : "hidden xl:inline"
                        )}
                        style={{
                          backgroundColor: "rgba(255,255,255,0.05)",
                          color: "rgba(255,255,255,0.4)",
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
              );
            })()}

            {/* Reviews Panel Toggle */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleReviewsPanel}
                  className={cn(
                    "relative gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                    reviewsPanelOpen ? "px-3" : "px-2 xl:px-3"
                  )}
                  style={{
                    background: reviewsPanelOpen
                      ? "rgba(255,107,53,0.1)"
                      : "transparent",
                    border: reviewsPanelOpen ? "1px solid rgba(255,107,53,0.15)" : "1px solid transparent",
                    color: reviewsPanelOpen ? "#ff6b35" : "rgba(255,255,255,0.5)",
                  }}
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
                <KanbanSplitLayout
                  projectId={currentProjectId}
                  footer={
                    <ExecutionControlBar
                      runningCount={executionStatus.runningCount}
                      maxConcurrent={executionStatus.maxConcurrent}
                      queuedCount={executionStatus.queuedCount}
                      isPaused={executionStatus.isPaused}
                      isLoading={isExecutionLoading}
                      onPauseToggle={handlePauseToggle}
                      onStop={handleStop}
                    />
                  }
                >
                  <TaskBoard projectId={currentProjectId} />
                </KanbanSplitLayout>
              )}
              {currentView === "ideation" && (
                <IdeationView
                  session={sessionData?.session ?? activeSession}
                  sessions={allSessions}
                  proposals={proposals}
                  onNewSession={handleNewSession}
                  onSelectSession={handleSelectSession}
                  onArchiveSession={handleArchiveSession}
                  onDeleteSession={handleDeleteSession}
                  onSelectProposal={handleSelectProposal}
                  onEditProposal={handleEditProposal}
                  onRemoveProposal={handleRemoveProposal}
                  onReorderProposals={handleReorderProposals}
                  onApply={handleApplyProposals}
                />
              )}
              {currentView === "extensibility" && <ExtensibilityView />}
              {currentView === "activity" && <ActivityView showHeader />}
              {currentView === "settings" && <SettingsView />}
            </div>
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
                  approveReview.mutate({ reviewId });
                }}
                onRequestChanges={(reviewId, notes) => {
                  // Use provided notes or a default message
                  // Note: ReviewDetailModal (PRD task 37) will provide proper notes input
                  const reviewNotes = notes || "Changes requested";
                  requestChangesReview.mutate({ reviewId, notes: reviewNotes });
                }}
                onViewDiff={(_reviewId) => {
                  // TODO: Open diff viewer (task in PRD - ColumnGroup component)
                }}
                isApproving={isApproving}
                isRequestingChanges={isRequestingChanges}
              />
            </div>
          )}

          {/* ChatPanel - resizable side panel with Cmd+K toggle (not on kanban or ideation) */}
          {currentView !== "kanban" && currentView !== "ideation" && <ChatPanel context={chatContext} />}
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

      {/* Proposal Edit Modal - Edit ideation proposals */}
      <ProposalEditModal
        proposal={editingProposal}
        onSave={handleSaveProposal}
        onCancel={() => setEditingProposalId(null)}
        isSaving={updateProposal.isPending}
      />

      {/* TaskFullView - Full-screen task view (rendered when taskFullViewId is set) */}
      {taskFullViewId && (
        <TaskFullView taskId={taskFullViewId} onClose={closeTaskFullView} />
      )}

      {/* Confirmation Dialog */}
      <ConfirmationDialog />

      {/* Toast notifications */}
      <Toaster />
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
