import {
  CheckCircle2,
  Code,
  ExternalLink,
  FileText,
  GitPullRequestArrow,
  GitBranch,
  LayoutGrid,
  Network,
  ClipboardList,
  Loader2,
  X,
} from "lucide-react";
import type { ElementType } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";

import { artifactApi } from "@/api/artifact";
import { diffApi } from "@/api/diff";
import { ideationApi, toTaskProposal } from "@/api/ideation";
import {
  chatApi,
  type AgentConversationWorkspace,
  type AgentConversationWorkspacePublicationEvent,
} from "@/api/chat";
import { DiffViewer, type FileChange as DiffViewerFileChange } from "@/components/diff";
import { TaskGraphView } from "@/components/TaskGraph";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { withAlpha } from "@/lib/theme-colors";
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

const PUBLISH_TAB = {
  id: "publish" as const,
  label: "Commit & Publish",
  icon: GitPullRequestArrow,
};

interface AgentsArtifactPaneProps {
  conversation: AgentConversation | null;
  workspace?: AgentConversationWorkspace | null;
  activeTab: AgentArtifactTab;
  taskMode: AgentTaskArtifactMode;
  onTabChange: (tab: AgentArtifactTab) => void;
  onTaskModeChange: (mode: AgentTaskArtifactMode) => void;
  onPublishWorkspace: ((conversationId: string) => Promise<void>) | undefined;
  isPublishingWorkspace?: boolean;
  onClose: () => void;
}

export function AgentsArtifactPane({
  conversation,
  workspace = null,
  activeTab,
  taskMode,
  onTabChange,
  onTaskModeChange,
  onPublishWorkspace,
  isPublishingWorkspace = false,
  onClose,
}: AgentsArtifactPaneProps) {
  const queryClient = useQueryClient();
  const showIdeationTabs = workspace?.mode === "ideation";
  const showPublishTab =
    workspace?.mode === "edit" && !workspace.linkedIdeationSessionId && !workspace.linkedPlanBranchId;
  const shouldLoadIdeationData = showIdeationTabs;
  const conversationQuery = useConversation(conversation?.id ?? null, {
    enabled: shouldLoadIdeationData && !!conversation?.id,
  });
  const conversationData = conversationQuery.data;
  const conversationMessages = useMemo(
    () =>
      shouldLoadIdeationData &&
      conversationData &&
      conversationData.conversation?.id === conversation?.id
        ? conversationData.messages
        : [],
    [conversationData, conversation?.id, shouldLoadIdeationData],
  );
  const attachedSessionId = useMemo(
    () =>
      shouldLoadIdeationData
        ? resolveAttachedIdeationSessionId(conversation, conversationMessages)
        : null,
    [conversation, conversationMessages, shouldLoadIdeationData],
  );
  const sessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedSessionId!),
    enabled: shouldLoadIdeationData && !!attachedSessionId,
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
  const planArtifactId = shouldLoadIdeationData
    ? sessionData?.session.planArtifactId ?? sessionData?.session.inheritedPlanArtifactId ?? null
    : null;
  const planArtifactQuery = useQuery({
    queryKey: ["agents", "artifact", planArtifactId],
    queryFn: () => artifactApi.get(planArtifactId!),
    enabled: shouldLoadIdeationData && !!planArtifactId,
    staleTime: 5_000,
  });
  const verificationQuery = useVerificationStatus(
    shouldLoadIdeationData ? attachedSessionId ?? undefined : undefined,
  );
  const dependencyQuery = useDependencyGraph(
    shouldLoadIdeationData ? attachedSessionId ?? "" : "",
  );
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
  const visibleTabs = useMemo(
    () => [
      ...(showIdeationTabs ? ARTIFACT_TABS : []),
      ...(showPublishTab ? [PUBLISH_TAB] : []),
    ],
    [showIdeationTabs, showPublishTab],
  );
  const effectiveActiveTab =
    visibleTabs.some((tab) => tab.id === activeTab)
      ? activeTab
      : showPublishTab
        ? "publish"
        : "plan";
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
        data-testid="agents-artifact-tab-row"
        className="h-11 px-4 flex items-center gap-0 border-b shrink-0"
        style={{
          background: withAlpha("var(--bg-surface)", 60),
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <div className="flex h-full items-stretch gap-0 min-w-0 self-stretch">
          {visibleTabs.map(({ id, label, icon: Icon }) => {
            const isActive = effectiveActiveTab === id;
            const count = id === "proposal" ? proposalCount : 0;
            const showVerificationDot =
              id === "verification" &&
              (verificationInProgress ||
                verificationState === "verified" ||
                verificationState === "imported_verified" ||
                verificationState === "needs_revision");
            return (
              <button
                key={id}
                type="button"
                onClick={() => onTabChange(id)}
                className={cn(
                  "relative flex h-full self-stretch items-center gap-1.5 bg-transparent px-3 text-[12px] font-medium transition-colors duration-150 rounded-none shadow-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0 appearance-none",
                  id === "tasks" ? "hidden xl:flex" : ""
                )}
                style={{
                  color: isActive ? "var(--text-primary)" : "var(--text-muted)",
                  background: "transparent",
                  boxShadow: "none",
                }}
                data-testid={`agents-artifact-tab-${id}`}
                data-theme-button-skip="true"
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
                        ? withAlpha("var(--accent-primary)", 15)
                        : "var(--overlay-weak)",
                      color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
                    }}
                  >
                    {count}
                  </span>
                )}
                {isActive && (
                  <span
                    className="absolute -bottom-px left-3 right-3 h-[2px] rounded-full"
                    style={{ background: "var(--accent-primary)" }}
                  />
                )}
              </button>
            );
          })}
        </div>

        <div className="ml-auto flex items-center gap-1">
          {effectiveActiveTab === "tasks" && (
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
        data-testid={`agents-artifact-content-${effectiveActiveTab}`}
      >
        <ArtifactContent
          activeTab={effectiveActiveTab}
          workspace={workspace}
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
          onPublishWorkspace={onPublishWorkspace}
          isPublishingWorkspace={isPublishingWorkspace}
        />
      </div>
    </aside>
  );
}

type ArtifactContentProps = {
  activeTab: AgentArtifactTab;
  workspace: AgentConversationWorkspace | null;
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
  onPublishWorkspace: ((conversationId: string) => Promise<void>) | undefined;
  isPublishingWorkspace: boolean;
};

function ArtifactContent({
  activeTab,
  workspace,
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
  onPublishWorkspace,
  isPublishingWorkspace,
}: ArtifactContentProps) {
  if (activeTab === "publish") {
    return (
      <AgentPublishPanel
        workspace={workspace}
        onPublishWorkspace={onPublishWorkspace}
        isPublishingWorkspace={isPublishingWorkspace}
      />
    );
  }

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

function AgentPublishPanel({
  workspace,
  onPublishWorkspace,
  isPublishingWorkspace,
}: {
  workspace: AgentConversationWorkspace | null;
  onPublishWorkspace: ((conversationId: string) => Promise<void>) | undefined;
  isPublishingWorkspace: boolean;
}) {
  const queryClient = useQueryClient();
  const [reviewOpen, setReviewOpen] = useState(false);
  const [commitFiles, setCommitFiles] = useState<DiffViewerFileChange[]>([]);
  const conversationId = workspace?.conversationId ?? null;
  const changesQuery = useQuery({
    queryKey: ["agents", "workspace-diff", conversationId],
    queryFn: () => diffApi.getAgentConversationWorkspaceFileChanges(conversationId!),
    enabled: !!conversationId,
    staleTime: 2_000,
  });
  const publicationEventsQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-publication-events", conversationId],
    queryFn: () =>
      chatApi.listAgentConversationWorkspacePublicationEvents(conversationId!),
    enabled: !!conversationId,
    staleTime: 0,
    refetchInterval: isPublishingWorkspace ? 1_500 : false,
  });
  const freshnessQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-freshness", conversationId],
    queryFn: () => chatApi.getAgentConversationWorkspaceFreshness(conversationId!),
    enabled: !!conversationId && workspace?.mode === "edit",
    staleTime: 5_000,
  });
  const updateFromBaseMutation = useMutation({
    mutationFn: () => chatApi.updateAgentConversationWorkspaceFromBase(conversationId!),
    onSuccess: async (result) => {
      queryClient.setQueryData(
        ["agents", "conversation-workspace", result.workspace.conversationId],
        result.workspace,
      );
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["agents", "conversation-workspace", result.workspace.conversationId],
        }),
        queryClient.invalidateQueries({
          queryKey: ["agents", "conversation-workspace-freshness", result.workspace.conversationId],
        }),
        queryClient.invalidateQueries({
          queryKey: [
            "agents",
            "conversation-workspace-publication-events",
            result.workspace.conversationId,
          ],
        }),
        queryClient.invalidateQueries({
          queryKey: ["agents", "workspace-diff", result.workspace.conversationId],
        }),
      ]);
      toast.success(
        result.updated
          ? `Updated from ${result.targetRef}`
          : `Already current with ${result.targetRef}`,
      );
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to update from base",
      );
      if (conversationId) {
        void Promise.all([
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace", conversationId],
          }),
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace-freshness", conversationId],
          }),
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace-publication-events", conversationId],
          }),
        ]);
      }
    },
  });
  const changes = changesQuery.data ?? [];
  const publicationEvents = publicationEventsQuery.data ?? [];

  if (!workspace) {
    return <EmptyArtifactState title="No workspace selected" />;
  }

  const branch = workspace.branchName;
  const base = workspace.baseDisplayName ?? workspace.baseRef;
  const prLabel = workspace.publicationPrNumber
    ? `PR #${workspace.publicationPrNumber}`
    : workspace.publicationPrUrl
      ? "Published PR"
      : "No PR yet";
  const prUrlLabel = workspace.publicationPrUrl
    ? formatPullRequestUrlLabel(workspace.publicationPrUrl)
    : null;
  const freshness = freshnessQuery.data;
  const isBranchUpdateNeeded = Boolean(freshness?.isBaseAhead);
  const isUpdatingFromBase = updateFromBaseMutation.isPending;
  const effectivePublishing = isPublishingWorkspace || isUpdatingFromBase;
  const pipelineStatus = isUpdatingFromBase
    ? "refreshing"
    : isPublishingWorkspace &&
        !["checking", "committing", "refreshing", "refreshed", "pushing", "pushed"].includes(
          workspace.publicationPushStatus ?? "",
        )
      ? "checking"
      : workspace.publicationPushStatus;
  const baseActionLabel = workspace.baseRef || base;
  const publishDisabled =
    !onPublishWorkspace || effectivePublishing || workspace.status === "missing";

  return (
    <div className="min-h-full p-4" data-testid="agents-publish-pane">
      <div className="mx-auto flex max-w-3xl flex-col gap-4">
        <section
          className="rounded-lg border p-4"
          style={{
            background: "var(--bg-surface)",
            borderColor: "var(--border-subtle)",
          }}
        >
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <div className="text-sm font-semibold text-[var(--text-primary)]">
                Review Changes
              </div>
              <div className="mt-1 text-xs text-[var(--text-muted)]">
                Review this agent workspace before publishing its draft PR.
              </div>
            </div>
            <span
              className="rounded-full border px-2.5 py-1 text-xs capitalize"
              style={{
                borderColor: "var(--overlay-weak)",
                color: "var(--text-secondary)",
              }}
            >
              {workspace.publicationPushStatus ?? workspace.status}
            </span>
          </div>

          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            <PublishFact icon={GitBranch} label="Branch" value={branch} />
            <PublishFact icon={FileText} label="Base" value={base} />
            <PublishFact
              icon={GitPullRequestArrow}
              label="Pull Request"
              value={prLabel}
              description={prUrlLabel}
              descriptionAction={
                workspace.publicationPrUrl
                  ? {
                      label: `Open ${prUrlLabel}`,
                      testId: "agents-open-pr-url",
                      onClick: async () => {
                        await openUrl(workspace.publicationPrUrl!);
                      },
                    }
                  : undefined
              }
              action={
                workspace.publicationPrUrl
                  ? {
                      label: "Open pull request",
                      testId: "agents-open-pr",
                      onClick: async () => {
                        await openUrl(workspace.publicationPrUrl!);
                      },
                    }
                  : undefined
              }
            />
            <PublishFact
              icon={CheckCircle2}
              label="Mode"
              value={workspace.mode === "edit" ? "Edit agent" : workspace.mode}
            />
          </div>
          {effectivePublishing && (
            <PublishPipelineSteps
              status={pipelineStatus}
              isPublishing={effectivePublishing}
            />
          )}
        </section>

        <section
          className="rounded-lg border p-4"
          style={{
            background: "var(--bg-surface)",
            borderColor: "var(--border-subtle)",
          }}
        >
          {isBranchUpdateNeeded && (
            <div
              className="mb-3 rounded-md border px-3 py-2 text-xs leading-relaxed"
              style={{
                background: "var(--bg-subtle)",
                borderColor: "var(--status-warning)",
                color: "var(--text-secondary)",
              }}
              data-testid="agents-base-stale"
            >
              Base branch {freshness?.baseRef ?? baseActionLabel} has new commits.
            </div>
          )}
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="text-sm font-semibold text-[var(--text-primary)]">
                Commit & Publish
              </div>
              <div className="mt-1 text-xs leading-relaxed text-[var(--text-muted)]">
                {changesQuery.isLoading
                  ? "Loading changed files..."
                  : changes.length > 0
                    ? `${changes.length} changed file${changes.length === 1 ? "" : "s"} ready for review.`
                    : "No changed files detected yet."}
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="ghost"
                className="h-9 gap-2 px-3 text-xs"
                onClick={() => setReviewOpen(true)}
                disabled={changesQuery.isLoading || changes.length === 0}
                data-testid="agents-review-changes"
              >
                <Code className="h-3.5 w-3.5" />
                Review Changes
              </Button>
              {isBranchUpdateNeeded ? (
                <Button
                  type="button"
                  className="h-9 gap-2 px-3 text-xs"
                  onClick={() => updateFromBaseMutation.mutate()}
                  disabled={isUpdatingFromBase || workspace.status === "missing"}
                  data-testid="agents-update-from-base"
                >
                  {isUpdatingFromBase ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <GitBranch className="h-3.5 w-3.5" />
                  )}
                  Update from {baseActionLabel}
                </Button>
              ) : (
                <Button
                  type="button"
                  className="h-9 gap-2 px-3 text-xs"
                  onClick={() => void onPublishWorkspace?.(workspace.conversationId)}
                  disabled={publishDisabled}
                  data-testid="agents-publish-confirm"
                >
                  {isPublishingWorkspace ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <GitPullRequestArrow className="h-3.5 w-3.5" />
                  )}
                  Commit & Publish
                </Button>
              )}
            </div>
          </div>
        </section>
        <PublishEventLog
          events={publicationEvents}
          isLoading={publicationEventsQuery.isLoading}
          isPublishing={effectivePublishing}
        />
      </div>
      <Dialog open={reviewOpen} onOpenChange={setReviewOpen}>
        <DialogContent
          className="flex h-[95vh] w-[95vw] max-w-[95vw] flex-col gap-0 overflow-hidden p-0"
          style={{
            backgroundColor: "var(--bg-surface)",
            border: "1px solid var(--border-subtle)",
          }}
        >
          <DiffViewer
            changes={changes}
            commits={[]}
            commitFiles={commitFiles}
            onFetchDiff={async (filePath) => {
              if (!conversationId) {
                return null;
              }
              const diff = await diffApi.getAgentConversationWorkspaceFileDiff(
                conversationId,
                filePath,
              );
              return {
                filePath: diff.filePath,
                oldContent: diff.oldContent,
                newContent: diff.newContent,
                hunks: [],
                language: diff.language,
              };
            }}
            onFetchCommitFiles={async () => setCommitFiles([])}
            isLoadingChanges={changesQuery.isLoading}
            changesLabel="Workspace Changes"
            changesEmptyTitle="No workspace changes"
            changesEmptySubtitle="There are no changed files to review for this agent branch."
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}

function PublishEventLog({
  events,
  isLoading,
  isPublishing,
}: {
  events: AgentConversationWorkspacePublicationEvent[];
  isLoading: boolean;
  isPublishing: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  if (isLoading && events.length === 0) {
    return (
      <div className="text-xs text-[var(--text-muted)]">
        Loading publish history...
      </div>
    );
  }

  if (events.length === 0) {
    return null;
  }

  const activeStartedEventId =
    isPublishing && events.length > 0
      ? [...events].reverse().find((event) => event.status === "started")?.id
      : null;
  const visibleEvents = events
    .filter((event) =>
      event.status === "failed" ||
      event.status === "succeeded" ||
      event.status === "needs_agent" ||
      event.id === activeStartedEventId
    )
    .slice(-6)
    .reverse();

  if (visibleEvents.length === 0) {
    return null;
  }

  return (
    <div className="px-1" data-testid="agents-publish-events">
      <button
        type="button"
        className="flex items-center gap-2 bg-transparent p-0 text-[11px] font-medium text-[var(--text-muted)] transition-colors hover:text-[var(--text-secondary)]"
        onClick={() => setIsExpanded((current) => !current)}
        data-theme-button-skip="true"
        data-testid="agents-publish-history-toggle"
      >
        <span>{isExpanded ? "Hide publish history" : "Show publish history"}</span>
        <span className="text-[10px] text-[var(--text-muted)]">
          {visibleEvents.length}
        </span>
      </button>
      {isExpanded && (
        <div
          className="mt-3 space-y-2 border-l pl-3"
          style={{ borderColor: "var(--overlay-weak)" }}
        >
          {visibleEvents.map((event) => {
            const eventState =
              event.status === "failed" || event.status === "succeeded"
                ? event.status
                : event.id === activeStartedEventId
                  ? "active"
                  : "history";
            return (
              <div
                key={event.id}
                className="flex items-start gap-2 text-xs"
                data-testid={`agents-publish-event-${event.step}`}
              >
                <span
                  className="mt-1 flex h-3 w-3 shrink-0 items-center justify-center rounded-full"
                  data-state={eventState}
                  data-testid={`agents-publish-event-icon-${event.id}`}
                  style={{
                    background:
                      eventState === "failed"
                        ? "var(--status-danger)"
                        : eventState === "active"
                          ? "var(--accent-primary)"
                          : "var(--overlay-weak)",
                    color:
                      eventState === "failed"
                        ? "var(--status-danger)"
                        : eventState === "active"
                          ? "var(--accent-primary)"
                          : "var(--text-muted)",
                  }}
                >
                  {eventState === "failed" ? (
                    <X className="h-2.5 w-2.5 text-[var(--bg-base)]" />
                  ) : eventState === "active" ? (
                    <Loader2 className="h-2.5 w-2.5 animate-spin text-[var(--bg-base)]" />
                  ) : (
                    <span className="h-1.5 w-1.5 rounded-full bg-[var(--bg-base)]" />
                  )}
                </span>
                <div className="min-w-0">
                  <div className="font-medium text-[var(--text-secondary)]">
                    {event.summary}
                  </div>
                  <div className="mt-0.5 text-[11px] capitalize text-[var(--text-muted)]">
                    {event.step.replace(/_/g, " ")}
                    {event.classification
                      ? ` / ${event.classification.replace(/_/g, " ")}`
                      : ""}
                    {event.createdAt ? ` / ${formatPublishEventTime(event.createdAt)}` : ""}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function formatPublishEventTime(createdAt: string): string {
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) {
    return createdAt;
  }
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

function formatPullRequestUrlLabel(url: string): string {
  try {
    const parsed = new URL(url);
    return `${parsed.host}${parsed.pathname}`;
  } catch {
    return url;
  }
}

const PUBLISH_STEPS = [
  { id: "checking", label: "Check workspace" },
  { id: "committing", label: "Commit changes" },
  { id: "refreshing", label: "Refresh branch" },
  { id: "pushing", label: "Push branch" },
  { id: "pushed", label: "Open draft PR" },
] as const;

function PublishPipelineSteps({
  status,
  isPublishing,
}: {
  status: string | null;
  isPublishing: boolean;
}) {
  const normalizedStatus = status ?? "idle";
  const activeIndex = (() => {
    if (normalizedStatus === "pushed") {
      return PUBLISH_STEPS.length;
    }
    if (normalizedStatus === "pushing") {
      return 3;
    }
    if (normalizedStatus === "refreshed") {
      return 3;
    }
    if (normalizedStatus === "refreshing") {
      return 2;
    }
    if (normalizedStatus === "committing") {
      return 1;
    }
    return 0;
  })();
  const isRepairStatus = normalizedStatus === "needs_agent";
  const isTerminalFailure = normalizedStatus === "failed" || isRepairStatus;

  return (
    <div
      className="mt-4 rounded-md border p-3"
      style={{
        background: "var(--bg-subtle)",
        borderColor: "var(--border-subtle)",
      }}
      data-testid="agents-publish-pipeline"
    >
      <div className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-muted)]">
        Publish pipeline
      </div>
      <div className="grid gap-2 sm:grid-cols-5">
        {PUBLISH_STEPS.map((step, index) => {
          const isDone = activeIndex > index;
          const isActive = isPublishing && activeIndex === index;
          const isFailed = isTerminalFailure && index === 0;
          return (
            <div
              key={step.id}
              className="flex items-center gap-2 text-xs"
              data-testid={`agents-publish-step-${step.id}`}
              style={{
                color:
                  isDone || isActive || isFailed
                    ? "var(--text-primary)"
                    : "var(--text-muted)",
              }}
            >
              <span
                className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border"
                style={{
                  borderColor: isFailed
                    ? "var(--status-danger)"
                    : isDone
                      ? "var(--status-success)"
                      : isActive
                        ? "var(--accent-primary)"
                        : "var(--overlay-weak)",
                  color: isFailed
                    ? "var(--status-danger)"
                    : isDone
                      ? "var(--status-success)"
                      : isActive
                        ? "var(--accent-primary)"
                        : "var(--text-muted)",
                }}
              >
                {isActive ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : isDone ? (
                  <CheckCircle2 className="h-3 w-3" />
                ) : isFailed ? (
                  <X className="h-3 w-3" />
                ) : (
                  index + 1
                )}
              </span>
              <span>{step.label}</span>
            </div>
          );
        })}
      </div>
      {isTerminalFailure && (
        <div className="mt-3 text-xs text-[var(--text-muted)]">
          {isRepairStatus
            ? "The latest publish attempt found a fixable issue and sent it back to the workspace agent."
            : "The latest publish attempt failed. Fixable errors are sent back to the workspace agent."}
        </div>
      )}
    </div>
  );
}

function PublishFact({
  icon: Icon,
  label,
  value,
  description,
  descriptionAction,
  action,
}: {
  icon: ElementType;
  label: string;
  value: string;
  description?: string | null;
  descriptionAction?: {
    label: string;
    testId: string;
    onClick: () => void | Promise<void>;
  } | undefined;
  action?: {
    label: string;
    testId: string;
    onClick: () => void | Promise<void>;
  } | undefined;
}) {
  return (
    <div
      className="flex min-w-0 items-start gap-2 rounded-md border px-3 py-2"
      style={{
        background: "var(--bg-base)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-[var(--text-muted)]" />
      <div className="min-w-0 flex-1">
        <div className="text-[10px] font-medium uppercase tracking-[0.14em] text-[var(--text-muted)]">
          {label}
        </div>
        <div className="mt-1 flex min-w-0 items-center gap-2">
          <div className="truncate text-xs font-medium text-[var(--text-primary)]">
            {value}
          </div>
          {action && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  className="h-6 w-6 shrink-0 p-0"
                  aria-label={action.label}
                  data-testid={action.testId}
                  onClick={() => void action.onClick()}
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                {action.label}
              </TooltipContent>
            </Tooltip>
          )}
        </div>
        {description && (
          descriptionAction ? (
            <button
              type="button"
              className="mt-1 block max-w-full truncate bg-transparent p-0 text-left text-[10px] text-[var(--text-muted)] transition-colors hover:text-[var(--text-secondary)]"
              onClick={() => void descriptionAction.onClick()}
              aria-label={descriptionAction.label}
              data-theme-button-skip="true"
              data-testid={descriptionAction.testId}
            >
              {description}
            </button>
          ) : (
            <div className="mt-1 truncate text-[10px] text-[var(--text-muted)]">
              {description}
            </div>
          )
        )}
      </div>
    </div>
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
