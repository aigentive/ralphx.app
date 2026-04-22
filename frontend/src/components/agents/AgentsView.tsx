import { useCallback, useEffect, useMemo, useState, type ElementType } from "react";
import { useQueryClient } from "@tanstack/react-query";
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
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { chatKeys } from "@/hooks/useChat";
import { useProjects } from "@/hooks/useProjects";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { getModelLabel } from "@/lib/model-utils";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import {
  selectArtifactState,
  useAgentSessionStore,
  type AgentArtifactTab,
  type AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import type { ChatConversation } from "@/types/chat-conversation";
import { AgentsArtifactPane } from "./AgentsArtifactPane";
import { AgentsSidebar } from "./AgentsSidebar";
import {
  DEFAULT_AGENT_RUNTIME,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { NewAgentDialog } from "./NewAgentDialog";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

const HEADER_ARTIFACT_TABS: Array<{
  id: AgentArtifactTab;
  label: string;
  icon: ElementType;
}> = [
  { id: "plan", label: "Plan", icon: FileText },
  { id: "verification", label: "Verification", icon: CheckCircle2 },
  { id: "proposal", label: "Proposal", icon: GitPullRequestArrow },
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
  const focusedConversations = useProjectAgentConversations(activeProjectId);
  const artifactState = useAgentSessionStore(selectArtifactState(selectedConversationId));
  const activeChatConversationId = useChatStore((s) =>
    activeProjectId ? s.activeConversationIds[buildStoreKey("project", activeProjectId)] ?? null : null
  );

  const activeConversation = useMemo(() => {
    if (!selectedConversationId) {
      return null;
    }
    return focusedConversations.data?.find((conversation) => conversation.id === selectedConversationId) ?? null;
  }, [focusedConversations.data, selectedConversationId]);

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
      setActiveConversation(buildStoreKey("project", activeProjectId), firstConversation.id);
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
        setActiveConversation(buildStoreKey("project", activeProjectId), replacement.id);
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
    if (
      !activeProjectId ||
      !activeChatConversationId ||
      activeChatConversationId === selectedConversationId
    ) {
      return;
    }
    const conversation = focusedConversations.data?.find(
      (item) => item.id === activeChatConversationId
    );
    if (conversation) {
      selectConversation(activeProjectId, conversation.id);
    }
  }, [
    activeChatConversationId,
    activeProjectId,
    focusedConversations.data,
    selectConversation,
    selectedConversationId,
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
    (conversationProjectId: string, conversation: ChatConversation) => {
      selectConversation(conversationProjectId, conversation.id);
      setActiveConversation(buildStoreKey("project", conversationProjectId), conversation.id);
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
      const conversation = await chatApi.createConversation("project", targetProjectId, title);
      setRuntimeForConversation(conversation.id, targetProjectId, runtime);
      selectConversation(targetProjectId, conversation.id);
      setActiveConversation(buildStoreKey("project", targetProjectId), conversation.id);
      await queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList("project", targetProjectId),
      });
    },
    [queryClient, selectConversation, setActiveConversation, setRuntimeForConversation]
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
          isCreatingAgent={isQuickCreating}
        />

        <div className="relative flex-1 min-w-0 h-full flex overflow-hidden">
          {activeProjectId && selectedConversationId ? (
            <div className="flex-1 min-w-0 h-full">
              <IntegratedChatPanel
                projectId={activeProjectId}
                conversationIdOverride={selectedConversationId}
                sendOptions={{
                  conversationId: selectedConversationId,
                  providerHarness: normalizedActiveRuntime.provider,
                  modelId: normalizedActiveRuntime.modelId,
                }}
                headerContent={
                  <AgentsChatHeader
                    conversation={activeConversation}
                    runtime={normalizedActiveRuntime}
                    artifactOpen={artifactState.isOpen}
                    activeArtifactTab={artifactState.activeTab}
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
  conversation: ChatConversation | null;
  runtime: AgentRuntimeSelection;
  artifactOpen: boolean;
  activeArtifactTab: AgentArtifactTab;
  onToggleArtifacts: () => void;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
}

function AgentsChatHeader({
  conversation,
  runtime,
  artifactOpen,
  activeArtifactTab,
  onToggleArtifacts,
  onSelectArtifact,
}: AgentsChatHeaderProps) {
  const title = conversation?.title || "Untitled agent";
  const modelLabel = getModelLabel(runtime.modelId);

  return (
    <div className="flex items-center gap-3 min-w-0">
      <div className="min-w-0">
        <div className="text-sm font-semibold truncate" style={{ color: "var(--text-primary)" }}>
          {title}
        </div>
        <div className="text-[11px] truncate" style={{ color: "var(--text-muted)" }}>
          {runtime.provider} · {modelLabel}
        </div>
      </div>

      <div className="hidden md:flex items-center gap-1">
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

function runtimeFromConversation(
  conversation: ChatConversation | null
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
