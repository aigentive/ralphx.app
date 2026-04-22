import { useCallback, useEffect, useMemo, useRef, useState, type ElementType } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  ClipboardList,
  FileText,
  GitPullRequestArrow,
  PanelRightOpen,
  PanelRightClose,
} from "lucide-react";
import { toast } from "sonner";

import { chatApi } from "@/api/chat";
import { executionApi } from "@/api/execution";
import { ideationApi } from "@/api/ideation";
import { projectsApi } from "@/api/projects";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { chatKeys, useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { projectKeys, useProjects } from "@/hooks/useProjects";
import { getModelLabel } from "@/lib/model-utils";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import {
  selectArtifactState,
  useAgentSessionStore,
  type AgentArtifactTab,
  type AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import { AgentsArtifactPane } from "./AgentsArtifactPane";
import { AgentsSidebar } from "./AgentsSidebar";
import {
  getAgentConversationStoreKey,
  sortAgentConversations,
  toProjectAgentConversation,
  type AgentConversation,
} from "./agentConversations";
import {
  deriveAgentTitleFromMessages,
  isDefaultAgentTitle,
} from "./agentTitle";
import {
  DEFAULT_AGENT_RUNTIME,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { NewAgentDialog } from "./NewAgentDialog";
import {
  agentConversationKeys,
  useProjectAgentConversations,
} from "./useProjectAgentConversations";
import { resolveAttachedIdeationSessionId } from "./attachedIdeationSession";

const HEADER_ARTIFACT_TABS: Array<{
  id: AgentArtifactTab;
  label: string;
  icon: ElementType;
}> = [
  { id: "plan", label: "Plan", icon: FileText },
  { id: "verification", label: "Verification", icon: CheckCircle2 },
  { id: "proposal", label: "Proposals", icon: GitPullRequestArrow },
  { id: "tasks", label: "Tasks", icon: ClipboardList },
];

interface AgentsViewProps {
  projectId: string;
  isNewAgentDialogOpen: boolean;
  onNewAgentDialogOpenChange: (open: boolean) => void;
  onCreateProject: () => void;
}

export function AgentsView({
  projectId,
  isNewAgentDialogOpen,
  onNewAgentDialogOpenChange,
  onCreateProject,
}: AgentsViewProps) {
  const queryClient = useQueryClient();
  const [isQuickCreating, setIsQuickCreating] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const autoTitleStateRef = useRef<
    Map<string, { messages: string[]; lastTitle: string | null }>
  >(new Map());
  const childArchiveSyncRef = useRef<Set<string>>(new Set());
  const { data: projects = [], isLoading: isLoadingProjects } = useProjects();
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

  const focusedProjectId = useAgentSessionStore((s) => s.focusedProjectId);
  const selectedProjectId = useAgentSessionStore((s) => s.selectedProjectId);
  const selectedConversationId = useAgentSessionStore((s) => s.selectedConversationId);
  const runtimeByConversationId = useAgentSessionStore((s) => s.runtimeByConversationId);
  const lastRuntimeByProjectId = useAgentSessionStore((s) => s.lastRuntimeByProjectId);
  const setFocusedProject = useAgentSessionStore((s) => s.setFocusedProject);
  const selectConversation = useAgentSessionStore((s) => s.selectConversation);
  const clearSelection = useAgentSessionStore((s) => s.clearSelection);
  const setRuntimeForConversation = useAgentSessionStore((s) => s.setRuntimeForConversation);
  const setArtifactOpen = useAgentSessionStore((s) => s.setArtifactOpen);
  const setArtifactTab = useAgentSessionStore((s) => s.setArtifactTab);
  const setTaskArtifactMode = useAgentSessionStore((s) => s.setTaskArtifactMode);

  const defaultProjectId = focusedProjectId || selectedProjectId || projectId || projects[0]?.id || null;
  const activeProjectId = selectedProjectId || defaultProjectId;
  const focusedConversations = useProjectAgentConversations(activeProjectId, showArchived);
  const artifactState = useAgentSessionStore(selectArtifactState(selectedConversationId));

  const activeConversation = useMemo(() => {
    if (!selectedConversationId) {
      return null;
    }
    return focusedConversations.data?.find((conversation) => conversation.id === selectedConversationId) ?? null;
  }, [focusedConversations.data, selectedConversationId]);
  const selectedConversationQuery = useConversation(selectedConversationId, {
    enabled: !!selectedConversationId,
  });
  const attachedIdeationSessionId = useMemo(
    () => resolveAttachedIdeationSessionId(activeConversation, selectedConversationQuery.data?.messages ?? []),
    [activeConversation, selectedConversationQuery.data?.messages],
  );
  const attachedIdeationSessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedIdeationSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedIdeationSessionId!),
    enabled: !!attachedIdeationSessionId && activeConversation?.contextType === "project",
    staleTime: 5_000,
  });

  const activeRuntime = selectedConversationId
    ? runtimeByConversationId[selectedConversationId] ??
      runtimeFromConversation(activeConversation) ??
      null
    : null;
  const normalizedActiveRuntime = normalizeRuntimeSelection(activeRuntime);

  useEffect(() => {
    if (!focusedProjectId && projectId) {
      setFocusedProject(projectId);
    }
  }, [focusedProjectId, projectId, setFocusedProject]);

  useEffect(() => {
    if (!activeProjectId || selectedConversationId || !focusedConversations.isSuccess) {
      return;
    }
    const firstConversation = focusedConversations.data?.[0];
    if (firstConversation) {
      selectConversation(activeProjectId, firstConversation.id);
      setActiveConversation(
        getAgentConversationStoreKey(firstConversation),
        firstConversation.id
      );
    }
  }, [
    activeProjectId,
    focusedConversations.data,
    focusedConversations.isSuccess,
    selectConversation,
    selectedConversationId,
    setActiveConversation,
  ]);

  useEffect(() => {
    if (!selectedConversationId || !activeProjectId || focusedConversations.isLoading) {
      return;
    }
    const selectedStillExists = focusedConversations.data?.some(
      (conversation) => conversation.id === selectedConversationId
    );
    if (selectedStillExists === false) {
      const replacement = focusedConversations.data?.[0];
      if (replacement) {
        selectConversation(activeProjectId, replacement.id);
        setActiveConversation(
          getAgentConversationStoreKey(replacement),
          replacement.id
        );
      } else {
        clearSelection();
      }
    }
  }, [
    activeProjectId,
    clearSelection,
    focusedConversations.data,
    focusedConversations.isLoading,
    selectConversation,
    selectedConversationId,
    setActiveConversation,
  ]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || !selectedConversationId) {
        return;
      }
      const activeElement = document.activeElement;
      if (
        activeElement instanceof HTMLInputElement ||
        activeElement instanceof HTMLTextAreaElement
      ) {
        return;
      }

      if (event.key === "\\") {
        event.preventDefault();
        setArtifactOpen(selectedConversationId, !artifactState.isOpen);
        return;
      }

      const tabByKey: Record<string, AgentArtifactTab> = {
        "1": "plan",
        "2": "verification",
        "3": "proposal",
        "4": "tasks",
      };
      const tab = tabByKey[event.key];
      if (tab) {
        event.preventDefault();
        setArtifactTab(selectedConversationId, tab);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    artifactState.isOpen,
    selectedConversationId,
    setArtifactOpen,
    setArtifactTab,
  ]);

  const handleSelectConversation = useCallback(
    (conversationProjectId: string, conversation: AgentConversation) => {
      selectConversation(conversationProjectId, conversation.id);
      setActiveConversation(
        getAgentConversationStoreKey(conversation),
        conversation.id
      );
    },
    [selectConversation, setActiveConversation]
  );

  const handleCreateAgent = useCallback(
    async ({
      projectId: targetProjectId,
      title,
      runtime,
    }: {
      projectId: string;
      title: string;
      runtime: AgentRuntimeSelection;
    }) => {
      const trimmedTitle = title.trim();
      const conversation = await chatApi.createConversation(
        "project",
        targetProjectId,
        trimmedTitle
      );
      const agentConversation = toProjectAgentConversation(conversation);
      for (const includeArchived of [false, true]) {
        queryClient.setQueryData<AgentConversation[]>(
          agentConversationKeys.projectList(targetProjectId, includeArchived),
          (previous) =>
            sortAgentConversations([
              agentConversation,
              ...(previous ?? []).filter((item) => item.id !== agentConversation.id),
            ])
        );
      }
      setRuntimeForConversation(conversation.id, targetProjectId, runtime);
      selectConversation(targetProjectId, conversation.id);
      setActiveConversation(getAgentConversationStoreKey(agentConversation), conversation.id);
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: agentConversationKeys.project(targetProjectId),
        }),
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversationList("project", targetProjectId),
        }),
      ]);
    },
    [
      queryClient,
      selectConversation,
      setActiveConversation,
      setRuntimeForConversation,
    ]
  );

  const handleQuickCreateAgent = useCallback(async (quickProjectId?: string) => {
    if (isQuickCreating) {
      return;
    }
    const targetProjectId = quickProjectId || focusedProjectId || selectedProjectId || projectId || projects[0]?.id;
    if (!targetProjectId) {
      return;
    }
    const runtime =
      lastRuntimeByProjectId[targetProjectId] ??
      runtimeByConversationId[selectedConversationId ?? ""] ??
      DEFAULT_AGENT_RUNTIME;
    try {
      setIsQuickCreating(true);
      await handleCreateAgent({
        projectId: targetProjectId,
        title: "",
        runtime: normalizeRuntimeSelection(runtime),
      });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to create agent");
    } finally {
      setIsQuickCreating(false);
    }
  }, [
    focusedProjectId,
    handleCreateAgent,
    isQuickCreating,
    lastRuntimeByProjectId,
    projectId,
    projects,
    runtimeByConversationId,
    selectedConversationId,
    selectedProjectId,
  ]);

  const handleSelectArtifact = useCallback(
    (tab: AgentArtifactTab) => {
      if (!selectedConversationId) {
        return;
      }
      if (artifactState.isOpen && artifactState.activeTab === tab) {
        setArtifactOpen(selectedConversationId, false);
        return;
      }
      setArtifactTab(selectedConversationId, tab);
    },
    [
      artifactState.activeTab,
      artifactState.isOpen,
      selectedConversationId,
      setArtifactOpen,
      setArtifactTab,
    ]
  );

  const invalidateProjectConversations = useCallback(
    async (targetProjectId: string) => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: agentConversationKeys.project(targetProjectId),
        }),
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversationList("project", targetProjectId),
        }),
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() }),
      ]);
    },
    [queryClient]
  );

  useEffect(() => {
    if (
      activeConversation?.contextType !== "project" ||
      !attachedIdeationSessionQuery.data ||
      activeConversation.archivedAt ||
      childArchiveSyncRef.current.has(activeConversation.id)
    ) {
      return;
    }
    const session = attachedIdeationSessionQuery.data.session;
    const sessionArchived = session.status === "archived" || Boolean(session.archivedAt);
    if (!sessionArchived) {
      return;
    }
    childArchiveSyncRef.current.add(activeConversation.id);
    void chatApi.archiveConversation(activeConversation.id)
      .then(() => invalidateProjectConversations(activeConversation.projectId))
      .catch(() => {
        childArchiveSyncRef.current.delete(activeConversation.id);
        // Status sync is best-effort; manual archive remains available.
      });
  }, [
    activeConversation,
    attachedIdeationSessionQuery.data,
    invalidateProjectConversations,
  ]);

  const handleRemoveProject = useCallback(
    async (targetProjectId: string) => {
      try {
        try {
          await projectsApi.archive(targetProjectId);
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          if (!message.includes("currently active project")) {
            throw err;
          }
          await executionApi.setActiveProject(undefined);
          await projectsApi.archive(targetProjectId);
        }
        if (focusedProjectId === targetProjectId) {
          setFocusedProject(null);
        }
        if (selectedProjectId === targetProjectId) {
          clearSelection();
        }
        await queryClient.invalidateQueries({ queryKey: projectKeys.list() });
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to remove project");
      }
    },
    [
      clearSelection,
      focusedProjectId,
      queryClient,
      selectedProjectId,
      setFocusedProject,
    ]
  );

  const handleArchiveConversation = useCallback(
    async (conversation: AgentConversation) => {
      try {
        if (conversation.contextType === "ideation") {
          await ideationApi.sessions.archive(conversation.contextId);
        }
        await chatApi.archiveConversation(conversation.id);
        if (selectedConversationId === conversation.id) {
          clearSelection();
        }
        await invalidateProjectConversations(conversation.projectId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to archive session");
      }
    },
    [clearSelection, invalidateProjectConversations, selectedConversationId]
  );

  const handleRestoreConversation = useCallback(
    async (conversation: AgentConversation) => {
      try {
        if (conversation.contextType === "ideation") {
          await ideationApi.sessions.reopen(conversation.contextId);
        }
        await chatApi.restoreConversation(conversation.id);
        await invalidateProjectConversations(conversation.projectId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to restore session");
      }
    },
    [invalidateProjectConversations]
  );

  const handleRenameConversation = useCallback(
    async (conversationId: string, title: string) => {
      const trimmed = title.trim();
      if (!trimmed) {
        return;
      }
      const conversation = focusedConversations.data?.find(
        (item) => item.id === conversationId
      );
      if (conversation?.contextType === "ideation") {
        await Promise.all([
          chatApi.updateConversationTitle(conversationId, trimmed),
          ideationApi.sessions.updateTitle(conversation.contextId, trimmed),
        ]);
      } else {
        await chatApi.updateConversationTitle(conversationId, trimmed);
      }
      autoTitleStateRef.current.delete(conversationId);
      await invalidateProjectConversations(conversation?.projectId ?? activeProjectId ?? projectId);
    },
    [activeProjectId, focusedConversations.data, invalidateProjectConversations, projectId]
  );

  const handleAgentUserMessageSent = useCallback(
    ({ content, result }: { content: string; result: { conversationId: string } }) => {
      const conversationId = result.conversationId || selectedConversationId;
      if (!conversationId || !activeProjectId) {
        return;
      }

      const conversation = focusedConversations.data?.find(
        (item) => item.id === conversationId
      );
      const titleIsAutoManaged =
        isDefaultAgentTitle(conversation?.title) ||
        autoTitleStateRef.current.get(conversationId)?.lastTitle === conversation?.title;
      if (!titleIsAutoManaged) {
        return;
      }

      const state = autoTitleStateRef.current.get(conversationId) ?? {
        messages: [],
        lastTitle: null,
      };
      if (state.messages.length >= 3) {
        return;
      }

      state.messages = [...state.messages, content].slice(0, 3);
      const nextTitle = deriveAgentTitleFromMessages(state.messages);
      if (!nextTitle || nextTitle === conversation?.title || nextTitle === state.lastTitle) {
        autoTitleStateRef.current.set(conversationId, state);
        return;
      }

      state.lastTitle = nextTitle;
      autoTitleStateRef.current.set(conversationId, state);
      const titleUpdate =
        conversation?.contextType === "ideation"
          ? Promise.all([
              chatApi.updateConversationTitle(conversationId, nextTitle),
              ideationApi.sessions.updateTitle(conversation.contextId, nextTitle),
            ])
          : chatApi.updateConversationTitle(conversationId, nextTitle);
      void titleUpdate
        .then(() => {
          void invalidateProjectConversations(conversation?.projectId ?? activeProjectId);
        })
        .catch(() => {
          // Auto-titling is best-effort; manual title editing remains available.
        });
    },
    [
      activeProjectId,
      focusedConversations.data,
      invalidateProjectConversations,
      selectedConversationId,
    ]
  );

  const defaultRuntime =
    (defaultProjectId ? lastRuntimeByProjectId[defaultProjectId] : null) ??
    (selectedConversationId ? runtimeByConversationId[selectedConversationId] : null) ??
    DEFAULT_AGENT_RUNTIME;

  return (
    <TooltipProvider delayDuration={300}>
      <section
        className="h-full min-h-0 w-full flex overflow-hidden"
        style={{ background: "var(--bg-base)" }}
        data-testid="agents-view"
      >
        <AgentsSidebar
          projects={projects}
          focusedProjectId={focusedProjectId ?? defaultProjectId}
          selectedConversationId={selectedConversationId}
          onFocusProject={setFocusedProject}
          onSelectConversation={handleSelectConversation}
          onCreateAgent={() => onNewAgentDialogOpenChange(true)}
          onCreateProject={onCreateProject}
          onQuickCreateAgent={handleQuickCreateAgent}
          onRemoveProject={handleRemoveProject}
          onArchiveConversation={handleArchiveConversation}
          onRestoreConversation={handleRestoreConversation}
          isCreatingAgent={isQuickCreating}
          showArchived={showArchived}
          onShowArchivedChange={setShowArchived}
        />

        <div className="relative flex-1 min-w-0 h-full flex overflow-hidden">
          {activeProjectId && selectedConversationId && activeConversation ? (
            <div className="flex-1 min-w-0 h-full">
              <IntegratedChatPanel
                key={selectedConversationId}
                projectId={activeProjectId}
                {...(activeConversation.contextType === "ideation"
                  ? { ideationSessionId: activeConversation.contextId }
                  : {})}
                conversationIdOverride={selectedConversationId}
                storeContextKeyOverride={getAgentConversationStoreKey(activeConversation)}
                agentProcessContextIdOverride={
                  activeConversation.contextType === "project"
                    ? selectedConversationId
                    : undefined
                }
                sendOptions={{
                  conversationId: selectedConversationId,
                  providerHarness: normalizedActiveRuntime.provider,
                  modelId: normalizedActiveRuntime.modelId,
                }}
                onUserMessageSent={handleAgentUserMessageSent}
                hideHeaderSessionControls
                hideSessionToolbar
                surfaceBackground="var(--bg-base)"
                {...(activeConversation.contextType === "project" && attachedIdeationSessionId
                  ? { additionalQuestionSessionIds: [attachedIdeationSessionId] }
                  : {})}
                headerContent={
                  <AgentsChatHeader
                    conversation={activeConversation}
                    runtime={normalizedActiveRuntime}
                    artifactOpen={artifactState.isOpen}
                    activeArtifactTab={artifactState.activeTab}
                    onRenameConversation={handleRenameConversation}
                    onToggleArtifacts={() => setArtifactOpen(selectedConversationId, !artifactState.isOpen)}
                    onSelectArtifact={handleSelectArtifact}
                  />
                }
                emptyState={<div />}
              />
            </div>
          ) : (
            <div className="flex-1 min-w-0 h-full flex items-center justify-center">
              <div className="flex flex-col items-center gap-3 text-center">
                <div className="space-y-1">
                  <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                    Pick a conversation from the sidebar
                  </div>
                  <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                    or start a new one.
                  </div>
                </div>
                <Button type="button" onClick={() => onNewAgentDialogOpenChange(true)} disabled={isLoadingProjects}>
                  New agent
                </Button>
              </div>
            </div>
          )}

          {selectedConversationId && artifactState.isOpen && (
            <AgentsArtifactPane
              conversation={activeConversation}
              activeTab={artifactState.activeTab}
              taskMode={artifactState.taskMode}
              onTabChange={handleSelectArtifact}
              onTaskModeChange={(mode) => setTaskArtifactMode(selectedConversationId, mode)}
              onClose={() => setArtifactOpen(selectedConversationId, false)}
            />
          )}
        </div>

        <NewAgentDialog
          open={isNewAgentDialogOpen}
          projects={projects}
          defaultProjectId={defaultProjectId}
          defaultRuntime={normalizeRuntimeSelection(defaultRuntime)}
          onOpenChange={onNewAgentDialogOpenChange}
          onCreate={handleCreateAgent}
          onCreateProject={onCreateProject}
        />
      </section>
    </TooltipProvider>
  );
}

interface AgentsChatHeaderProps {
  conversation: AgentConversation | null;
  runtime: AgentRuntimeSelection;
  artifactOpen: boolean;
  activeArtifactTab: AgentArtifactTab;
  onRenameConversation: (conversationId: string, title: string) => Promise<void>;
  onToggleArtifacts: () => void;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
}

function AgentsChatHeader({
  conversation,
  runtime,
  artifactOpen,
  activeArtifactTab,
  onRenameConversation,
  onToggleArtifacts,
  onSelectArtifact,
}: AgentsChatHeaderProps) {
  const title = conversation?.title || "Untitled agent";
  const modelLabel = getModelLabel(runtime.modelId);
  const providerLabel = runtime.provider === "codex" ? "Codex" : "Claude";
  const modeLabel = runtime.provider === "codex" ? "Medium" : "Default";
  const [isEditing, setIsEditing] = useState(false);
  const [draftTitle, setDraftTitle] = useState(title);

  useEffect(() => {
    if (!isEditing) {
      setDraftTitle(title);
    }
  }, [isEditing, title]);

  const commitTitle = useCallback(async () => {
    if (!conversation) {
      setIsEditing(false);
      return;
    }
    const trimmed = draftTitle.trim();
    if (!trimmed || trimmed === title) {
      setDraftTitle(title);
      setIsEditing(false);
      return;
    }
    await onRenameConversation(conversation.id, trimmed);
    setIsEditing(false);
  }, [conversation, draftTitle, onRenameConversation, title]);

  return (
    <div className="flex w-full flex-1 items-center justify-between gap-3 min-w-0">
      <div className="min-w-0 shrink">
        {isEditing ? (
          <Input
            value={draftTitle}
            onChange={(event) => setDraftTitle(event.target.value)}
            onBlur={() => void commitTitle()}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void commitTitle();
              }
              if (event.key === "Escape") {
                event.preventDefault();
                setDraftTitle(title);
                setIsEditing(false);
              }
            }}
            className="h-7 max-w-[260px] text-sm font-semibold"
            autoFocus
            aria-label="Agent title"
          />
        ) : (
          <button
            type="button"
            className="block max-w-[420px] text-left text-sm font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
            onClick={() => conversation && setIsEditing(true)}
            aria-label="Edit agent title"
          >
            {title}
          </button>
        )}
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] leading-none">
          <RuntimeMetaItem label="Provider" value={providerLabel} />
          <RuntimeMetaItem label="Model" value={modelLabel} />
          <RuntimeMetaItem label="Mode" value={modeLabel} />
        </div>
      </div>

      <div className="hidden md:flex items-center gap-1 ml-auto shrink-0">
        {HEADER_ARTIFACT_TABS.map(({ id, label, icon: Icon }) => {
          const isActive = activeArtifactTab === id && artifactOpen;
          return (
            <Tooltip key={id}>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className={cn("h-8 w-8 p-0", isActive ? "" : "opacity-80")}
                  onClick={() => onSelectArtifact(id)}
                  style={{
                    color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
                    background: isActive ? "var(--accent-muted)" : "transparent",
                  }}
                  aria-label={label}
                >
                  <Icon className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                {label}
              </TooltipContent>
            </Tooltip>
          );
        })}

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              onClick={onToggleArtifacts}
              aria-label={artifactOpen ? "Close artifacts" : "Open artifacts"}
            >
              {artifactOpen ? (
                <PanelRightClose className="w-4 h-4" />
              ) : (
                <PanelRightOpen className="w-4 h-4" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs">
            {artifactOpen ? "Close artifacts" : "Open artifacts"}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}

function RuntimeMetaItem({ label, value }: { label: string; value: string }) {
  return (
    <span className="inline-flex min-w-0 items-baseline gap-1">
      <span className="text-[var(--text-muted)]">{label}</span>
      <span className="truncate font-medium text-[var(--text-secondary)]">{value}</span>
    </span>
  );
}

function runtimeFromConversation(
  conversation: AgentConversation | null
): AgentRuntimeSelection | null {
  if (!conversation?.providerHarness) {
    return null;
  }

  if (conversation.providerHarness === "claude") {
    return {
      provider: "claude",
      modelId: "sonnet",
    };
  }

  if (conversation.providerHarness === "codex") {
    return {
      provider: "codex",
      modelId: DEFAULT_AGENT_RUNTIME.modelId,
    };
  }

  return null;
}
