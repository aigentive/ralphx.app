import {
  CheckCircle2,
  Copy,
  FileText,
  GitPullRequestArrow,
  LayoutGrid,
  Network,
  ClipboardList,
  X,
} from "lucide-react";
import type { ElementType } from "react";
import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";

import { artifactApi } from "@/api/artifact";
import { ideationApi, toTaskProposal } from "@/api/ideation";
import { TaskGraphView } from "@/components/TaskGraph";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
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
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
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

const PLAN_MARKDOWN_CLASSNAME = cn(
  "p-6 text-[13px] leading-[1.65] text-[var(--text-secondary)]",
  "[&_p]:mb-3.5 [&_p]:text-[13px] [&_p]:leading-[1.65] [&_p:last-child]:mb-0",
  "[&_h1]:mb-5 [&_h1]:mt-0 [&_h1]:border-b [&_h1]:border-[var(--border-subtle)] [&_h1]:pb-3 [&_h1]:text-[20px] [&_h1]:font-semibold [&_h1]:tracking-normal [&_h1]:text-[var(--text-primary)]",
  "[&_h2]:mb-3 [&_h2]:mt-7 [&_h2]:border-l-[3px] [&_h2]:border-[var(--accent-primary)] [&_h2]:pl-2.5 [&_h2]:text-[15px] [&_h2]:font-semibold [&_h2]:tracking-normal [&_h2]:text-[var(--text-primary)]",
  "[&_h3]:mb-2 [&_h3]:mt-5 [&_h3]:text-[13px] [&_h3]:font-semibold [&_h3]:tracking-normal [&_h3]:text-[var(--accent-primary)]",
  "[&_h4]:mb-1.5 [&_h4]:mt-4 [&_h4]:text-[13px] [&_h4]:font-semibold [&_h4]:tracking-normal [&_h4]:text-[var(--text-primary)]",
  "[&_ul]:mb-4 [&_ul]:space-y-1.5 [&_ul]:pl-5 [&_ol]:mb-4 [&_ol]:space-y-1.5 [&_ol]:pl-6",
  "[&_li]:mb-0 [&_li]:text-[13px] [&_li]:leading-[1.65] [&_li]:text-[var(--text-secondary)]",
  "[&_blockquote]:my-4 [&_blockquote]:rounded-r-md [&_blockquote]:border-l-[3px] [&_blockquote]:border-[var(--accent-primary)] [&_blockquote]:bg-[var(--overlay-faint)] [&_blockquote]:py-2 [&_blockquote]:pl-4 [&_blockquote]:pr-3",
  "[&_hr]:my-6 [&_hr]:border-0 [&_hr]:border-t [&_hr]:border-[var(--border-subtle)]",
  "[&_strong]:font-semibold [&_strong]:text-[var(--text-primary)]",
  "[&_code]:text-[12px] [&_pre_code]:text-[12px]",
  "[&_table]:w-full [&_table]:text-[12.5px] [&_th]:px-3.5 [&_th]:py-2.5 [&_td]:px-3.5 [&_td]:py-2.5"
);

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
  const conversationQuery = useConversation(conversation?.id ?? null, {
    enabled: !!conversation?.id,
  });
  const attachedSessionId = useMemo(
    () => resolveAttachedIdeationSessionId(conversation, conversationQuery.data?.messages ?? []),
    [conversation, conversationQuery.data?.messages],
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
  const sessionData = sessionQuery.data ?? null;
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
  const proposalCount = proposals.length;
  const verificationState =
    verificationQuery.data?.status ?? sessionData?.session.verificationStatus ?? "unverified";
  const verificationInProgress =
    verificationQuery.data?.inProgress ?? sessionData?.session.verificationInProgress ?? false;

  return (
    <aside
      className="w-1/2 min-w-[360px] max-w-[720px] h-full flex flex-col overflow-hidden border-l max-lg:absolute max-lg:inset-y-0 max-lg:right-0 max-lg:z-20 max-lg:w-[min(100%,420px)] max-lg:min-w-0 max-lg:max-w-none"
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
          planContent={getArtifactText(planArtifactQuery.data)}
          isPlanLoading={planArtifactQuery.isLoading}
          dependencyGraph={dependencyQuery.data ?? null}
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
  planContent: string | null;
  isPlanLoading: boolean;
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
  planContent,
  isPlanLoading,
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
    if (isPlanLoading) {
      return <EmptyArtifactState title="Loading plan..." />;
    }
    if (!planContent) {
      return (
        <EmptyArtifactState
          title="No plan artifact yet"
          detail={sessionTitle ? `${sessionTitle} has not produced a plan artifact yet.` : undefined}
        />
      );
    }
    return <MarkdownPanel content={planContent} />;
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

function MarkdownPanel({ content }: { content: string }) {
  const handleCopy = () => {
    if (!navigator.clipboard) {
      toast.error("Clipboard is unavailable");
      return;
    }
    navigator.clipboard
      .writeText(content)
      .then(() => toast.success("Copied to clipboard"))
      .catch(() => toast.error("Failed to copy"));
  };

  return (
    <div className="flex min-h-full flex-col">
      <div
        className="sticky top-0 z-10 flex h-10 items-center justify-end border-b px-4"
        style={{
          background: "color-mix(in srgb, var(--bg-surface) 94%, transparent)",
          borderColor: "var(--border-subtle)",
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
        }}
      >
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          className="h-7 gap-1.5 px-2 text-[12px]"
          style={{ color: "var(--text-secondary)" }}
        >
          <Copy className="w-3.5 h-3.5" />
          Copy Markdown
        </Button>
      </div>
      <MarkdownBody content={content} className={PLAN_MARKDOWN_CLASSNAME} />
    </div>
  );
}

function MarkdownBody({
  content,
  className,
  compact = false,
}: {
  content: string;
  className?: string | undefined;
  compact?: boolean | undefined;
}) {
  return (
    <div
      className={cn(
        "max-w-none overflow-hidden break-words [&_*]:max-w-full",
        compact && "[&>p]:mb-0 [&_ul]:mb-0 [&_ol]:mb-0",
        className,
      )}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
        {content}
      </ReactMarkdown>
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
      <TaskGraphView projectId={projectId} ideationSessionId={sessionId} />
    </div>
  );
}

function getArtifactText(artifact: Artifact | null | undefined): string | null {
  if (!artifact) {
    return null;
  }
  if (artifact.content.type === "inline") {
    return artifact.content.text;
  }
  return `Artifact content is stored at ${artifact.content.path}.`;
}
