import {
  CheckCircle2,
  FileText,
  GitPullRequestArrow,
  LayoutGrid,
  Network,
  ClipboardList,
  X,
} from "lucide-react";
import type { ElementType } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { artifactApi } from "@/api/artifact";
import { ideationApi, toTaskProposal } from "@/api/ideation";
import { chatApi } from "@/api/chat";
import { TaskGraphView } from "@/components/TaskGraph";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { ExportPlanDialog } from "@/components/Ideation/ExportPlanDialog";
import { PlanDisplay } from "@/components/Ideation/PlanDisplay";
import type { TeamMetadata } from "@/components/Ideation/PlanDisplay";
import { PlanEditor } from "@/components/Ideation/PlanEditor";
import { PlanEmptyState } from "@/components/Ideation/PlanEmptyState";
import { ProposalsTabContent } from "@/components/Ideation/ProposalsTabContent";
import { VerificationPanel } from "@/components/Ideation/VerificationPanel";
import type {
  AgentArtifactTab,
  AgentTaskArtifactMode,
} from "@/stores/agentSessionStore";
import { useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { useDependencyGraph } from "@/hooks/useDependencyGraph";
import { useVerificationStatus } from "@/hooks/useVerificationStatus";
import type { Artifact } from "@/types/artifact";
import type { IdeationSession, TaskProposal } from "@/types/ideation";
import type { DependencyGraphResponse } from "@/api/ideation.types";
import type { AgentConversation } from "./agentConversations";
import { resolveAttachedIdeationSessionId } from "./attachedIdeationSession";

const EMPTY_PROPOSAL_HIGHLIGHTS = new Set<string>();

function noop() {}

const ARTIFACT_TABS: Array<{
  id: AgentArtifactTab;
  label: string;
  icon: ElementType;
}> = [
  { id: "plan", label: "Plan", icon: FileText },
  { id: "verification", label: "Verification", icon: CheckCircle2 },
  { id: "proposal", label: "Proposals", icon: GitPullRequestArrow },
  { id: "tasks", label: "Tasks", icon: ClipboardList },
];

interface AgentsArtifactPaneProps {
  conversation: AgentConversation | null;
  activeTab: AgentArtifactTab;
  taskMode: AgentTaskArtifactMode;
  onTabChange: (tab: AgentArtifactTab) => void;
  onTaskModeChange: (mode: AgentTaskArtifactMode) => void;
  onClose: () => void;
}

export function AgentsArtifactPane({
  conversation,
  activeTab,
  taskMode,
  onTabChange,
  onTaskModeChange,
  onClose,
}: AgentsArtifactPaneProps) {
  const queryClient = useQueryClient();
  const conversationQuery = useConversation(conversation?.id ?? null, {
    enabled: !!conversation?.id,
  });
  const conversationData = conversationQuery.data;
  const conversationMessages = useMemo(
    () =>
      conversationData && conversationData.conversation?.id === conversation?.id
        ? conversationData.messages
        : [],
    [conversationData, conversation?.id],
  );
  const attachedSessionId = useMemo(
    () => resolveAttachedIdeationSessionId(conversation, conversationMessages),
    [conversation, conversationMessages],
  );
  const sessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedSessionId!),
    enabled: !!attachedSessionId,
    staleTime: 0,
    refetchInterval: (query) =>
      query.state.data?.session.verificationInProgress ||
      query.state.data?.session.acceptanceStatus === "pending"
        ? 3_000
        : false,
  });
  const rawSessionData = sessionQuery.data;
  const sessionData =
    attachedSessionId && rawSessionData?.session.id === attachedSessionId
      ? rawSessionData
      : null;
  const session = sessionData?.session ? (sessionData.session as IdeationSession) : null;
  const proposals = useMemo<TaskProposal[]>(
    () => (sessionData?.proposals ?? []).map(toTaskProposal),
    [sessionData?.proposals],
  );
  const planArtifactId =
    sessionData?.session.planArtifactId ?? sessionData?.session.inheritedPlanArtifactId ?? null;
  const planArtifactQuery = useQuery({
    queryKey: ["agents", "artifact", planArtifactId],
    queryFn: () => artifactApi.get(planArtifactId!),
    enabled: !!planArtifactId,
    staleTime: 5_000,
  });
  const verificationQuery = useVerificationStatus(attachedSessionId ?? undefined);
  const dependencyQuery = useDependencyGraph(attachedSessionId ?? "");
  const verificationData =
    attachedSessionId && verificationQuery.data?.sessionId === attachedSessionId
      ? verificationQuery.data
      : null;
  const dependencyGraph = attachedSessionId && sessionData ? dependencyQuery.data ?? null : null;
  const proposalCount = proposals.length;
  const verificationState =
    verificationData?.status ?? sessionData?.session.verificationStatus ?? "unverified";
  const verificationInProgress =
    verificationData?.inProgress ?? sessionData?.session.verificationInProgress ?? false;
  const handlePlanUpdated = useCallback(
    (updatedPlan: Artifact) => {
      queryClient.setQueryData(["agents", "artifact", updatedPlan.id], updatedPlan);
    },
    [queryClient],
  );

  return (
    <aside
      className="h-full w-full min-w-0 flex flex-col overflow-hidden border-l"
      style={{
        background: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
      }}
      data-testid="agents-artifact-pane"
    >
      <div
        className="h-11 px-4 flex items-center gap-0 border-b shrink-0"
        style={{
          backgroundColor: "color-mix(in srgb, var(--text-primary) 2%, transparent)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <div className="flex items-center gap-0 min-w-0">
          {ARTIFACT_TABS.map(({ id, label, icon: Icon }) => {
            const isActive = activeTab === id;
            const count = id === "proposal" ? proposalCount : 0;
            const showVerificationDot =
              id === "verification" &&
              (verificationInProgress ||
                verificationState === "verified" ||
                verificationState === "imported_verified" ||
                verificationState === "needs_revision");
            return (
              <Tooltip key={id}>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    onClick={() => onTabChange(id)}
                    className={cn(
                      "relative h-11 px-3 flex items-center gap-1.5 text-[12px] font-medium transition-colors duration-150",
                      id === "tasks" ? "hidden xl:flex" : ""
                    )}
                    style={{
                      color: isActive ? "var(--text-primary)" : "var(--text-muted)",
                    }}
                    data-testid={`agents-artifact-tab-${id}`}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span>{label}</span>
                    {showVerificationDot && (
                      <span
                        className={cn(
                          "w-2 h-2 rounded-full shrink-0",
                          verificationInProgress ? "animate-pulse" : ""
                        )}
                        style={{
                          background:
                            verificationState === "needs_revision"
                              ? "var(--status-warning)"
                              : verificationState === "verified" || verificationState === "imported_verified"
                                ? "var(--status-success)"
                                : "var(--accent-primary)",
                        }}
                      />
                    )}
                    {count > 0 && (
                      <span
                        className="text-[10px] font-semibold px-1.5 py-0.5 rounded-full"
                        style={{
                          background: isActive
                            ? "color-mix(in srgb, var(--accent-primary) 15%, transparent)"
                            : "var(--bg-hover)",
                          color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
                        }}
                      >
                        {count}
                      </span>
                    )}
                    {isActive && (
                      <span
                        className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                        style={{ background: "var(--accent-primary)" }}
                      />
                    )}
                  </button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  {label}
                </TooltipContent>
              </Tooltip>
            );
          })}
        </div>

        <div className="ml-auto flex items-center gap-1">
          {activeTab === "tasks" && (
            <div
              className="h-8 p-0.5 flex items-center rounded-md border"
              style={{
                borderColor: "var(--border-subtle)",
                background: "var(--bg-base)",
              }}
              data-testid="agents-task-mode-toggle"
            >
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => onTaskModeChange("graph")}
                    className="h-7 w-7 p-0"
                    style={{
                      color: taskMode === "graph" ? "var(--accent-primary)" : "var(--text-muted)",
                      background: taskMode === "graph" ? "var(--accent-muted)" : "transparent",
                    }}
                    aria-label="Graph"
                  >
                    <Network className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  Graph
                </TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => onTaskModeChange("kanban")}
                    className="h-7 w-7 p-0"
                    style={{
                      color: taskMode === "kanban" ? "var(--accent-primary)" : "var(--text-muted)",
                      background: taskMode === "kanban" ? "var(--accent-muted)" : "transparent",
                    }}
                    aria-label="Kanban"
                  >
                    <LayoutGrid className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  Kanban
                </TooltipContent>
              </Tooltip>
            </div>
          )}

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={onClose}
                className="h-8 w-8 p-0"
                aria-label="Close artifacts"
                data-testid="agents-artifact-close"
              >
                <X className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              Close artifacts
            </TooltipContent>
          </Tooltip>
        </div>
      </div>

      <div
        className="flex-1 min-h-0 overflow-y-auto"
        data-testid={`agents-artifact-content-${activeTab}`}
      >
        <ArtifactContent
          activeTab={activeTab}
          isLoading={conversationQuery.isLoading || sessionQuery.isLoading}
          attachedSessionId={attachedSessionId}
          projectId={conversation?.projectId ?? null}
          session={session}
          sessionTitle={sessionData?.session.title ?? null}
          taskMode={taskMode}
          planArtifact={planArtifactQuery.data ?? null}
          isPlanLoading={planArtifactQuery.isLoading}
          onPlanUpdated={handlePlanUpdated}
          dependencyGraph={dependencyGraph}
          proposals={proposals}
        />
      </div>
    </aside>
  );
}

type ArtifactContentProps = {
  activeTab: AgentArtifactTab;
  isLoading: boolean;
  attachedSessionId: string | null;
  projectId: string | null;
  session: IdeationSession | null;
  sessionTitle: string | null;
  taskMode: AgentTaskArtifactMode;
  planArtifact: Artifact | null;
  isPlanLoading: boolean;
  onPlanUpdated: (updatedPlan: Artifact) => void;
  dependencyGraph: DependencyGraphResponse | null;
  proposals: TaskProposal[];
};

function ArtifactContent({
  activeTab,
  isLoading,
  attachedSessionId,
  projectId,
  session,
  sessionTitle,
  taskMode,
  planArtifact,
  isPlanLoading,
  onPlanUpdated,
  dependencyGraph,
  proposals,
}: ArtifactContentProps) {
  if (isLoading) {
    return <EmptyArtifactState title="Loading attached run..." />;
  }

  if (!attachedSessionId) {
    return (
      <EmptyArtifactState
        title="No ideation run attached"
        detail="Start ideation from this agent chat to populate plan, verification, proposals, and tasks here."
      />
    );
  }

  if (activeTab === "plan") {
    return (
      <AgentPlanPanel
        session={session}
        sessionTitle={sessionTitle}
        planArtifact={planArtifact}
        isPlanLoading={isPlanLoading}
        proposals={proposals}
        onPlanUpdated={onPlanUpdated}
      />
    );
  }

  if (activeTab === "verification") {
    if (!session) {
      return <EmptyArtifactState title="No verification data yet" />;
    }
    return (
      <div className="flex h-full min-h-0 flex-col">
        <VerificationPanel session={session} />
      </div>
    );
  }

  if (activeTab === "proposal") {
    if (!session || proposals.length === 0) {
      return <EmptyArtifactState title="No proposals yet" />;
    }
    return (
      <ProposalsTabContent
        session={session}
        proposals={proposals}
        dependencyGraph={dependencyGraph}
        criticalPathSet={new Set(dependencyGraph?.criticalPath ?? [])}
        highlightedIds={EMPTY_PROPOSAL_HIGHLIGHTS}
        isReadOnly
        onEditProposal={noop}
        onNavigateToTask={noop}
        onViewHistoricalPlan={noop}
        onImportPlan={noop}
        onClearAll={noop}
        onAcceptPlan={noop}
        onReviewSync={noop}
        onUndoSync={noop}
        onDismissSync={noop}
        hideToolbar
      />
    );
  }

  return (
    <TaskArtifactSurface
      projectId={projectId}
      sessionId={attachedSessionId}
      mode={taskMode}
    />
  );
}

function EmptyArtifactState({ title, detail }: { title: string; detail?: string | undefined }) {
  return (
    <div className="h-full min-h-[220px] flex items-center justify-center p-6 text-center">
      <div className="max-w-sm">
        <div className="text-sm font-medium text-[var(--text-primary)]">{title}</div>
        {detail && <div className="mt-2 text-xs leading-relaxed text-[var(--text-muted)]">{detail}</div>}
      </div>
    </div>
  );
}

function AgentPlanPanel({
  session,
  sessionTitle,
  planArtifact,
  isPlanLoading,
  proposals,
  onPlanUpdated,
}: {
  session: IdeationSession | null;
  sessionTitle: string | null;
  planArtifact: Artifact | null;
  isPlanLoading: boolean;
  proposals: TaskProposal[];
  onPlanUpdated: (updatedPlan: Artifact) => void;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const [isPlanExpanded, setIsPlanExpanded] = useState(true);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);

  useEffect(() => {
    setIsEditing(false);
    setIsPlanExpanded(true);
  }, [planArtifact?.id, planArtifact?.metadata.version]);

  const teamMetadata = useMemo<TeamMetadata | undefined>(() => {
    if (!session?.teamMode || session.teamMode === "solo") {
      return undefined;
    }
    return {
      teamIdeated: true,
      teamMode: session.teamMode as "research" | "debate",
      teammateCount: session.teamConfig?.maxTeammates ?? 0,
      findings: [],
    };
  }, [session?.teamConfig?.maxTeammates, session?.teamMode]);

  const handleCreateProposals = useCallback(async () => {
    if (!session) return;
    try {
      await chatApi.sendAgentMessage("ideation", session.id, "create task proposals from the approved plan");
    } catch (err) {
      console.error("Failed to create proposals:", err);
      toast.error("Failed to request proposal creation");
    }
  }, [session]);

  if (isPlanLoading) {
    return <EmptyArtifactState title="Loading plan..." />;
  }

  return (
    <div className="min-h-full p-4">
      {planArtifact ? (
        isEditing ? (
          <PlanEditor
            plan={planArtifact}
            onSave={(updated) => {
              onPlanUpdated(updated);
              setIsEditing(false);
            }}
            onCancel={() => setIsEditing(false)}
          />
        ) : (
          <PlanDisplay
            plan={planArtifact}
            linkedProposalsCount={proposals.filter((proposal) => proposal.planArtifactId === planArtifact.id).length}
            onEdit={() => setIsEditing(true)}
            onExport={() => setExportDialogOpen(true)}
            isExpanded={isPlanExpanded}
            onExpandedChange={setIsPlanExpanded}
            {...(teamMetadata !== undefined && { teamMetadata })}
            {...(session !== null && { onCreateProposals: handleCreateProposals })}
          />
        )
      ) : (
        <PlanEmptyState />
      )}

      {session && (
        <ExportPlanDialog
          open={exportDialogOpen}
          onOpenChange={setExportDialogOpen}
          sessionId={session.id}
          sessionTitle={sessionTitle}
          verificationStatus={session.verificationStatus ?? "unverified"}
          planArtifact={planArtifact}
          projectId={session.projectId}
        />
      )}
    </div>
  );
}

function TaskArtifactSurface({
  projectId,
  sessionId,
  mode,
}: {
  projectId: string | null;
  sessionId: string;
  mode: AgentTaskArtifactMode;
}) {
  if (!projectId) {
    return <EmptyArtifactState title="No project selected" />;
  }

  if (mode === "kanban") {
    return (
      <div className="h-full min-h-[520px] overflow-hidden bg-[var(--bg-base)]">
        <TaskBoard projectId={projectId} ideationSessionId={sessionId} />
      </div>
    );
  }

  return (
    <div className="h-full min-h-[520px] overflow-hidden bg-[var(--bg-base)]">
      <TaskGraphView
        projectId={projectId}
        ideationSessionId={sessionId}
        hideCanvasControls
      />
    </div>
  );
}
