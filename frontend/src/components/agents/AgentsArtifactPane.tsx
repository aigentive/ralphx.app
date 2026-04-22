import {
  CheckCircle2,
  Copy,
  FileText,
  GitPullRequestArrow,
  LayoutGrid,
  Network,
  PanelRightClose,
  ClipboardList,
} from "lucide-react";
import type { ElementType } from "react";
import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";

import { artifactApi } from "@/api/artifact";
import { ideationApi } from "@/api/ideation";
import { useTaskGraph } from "@/components/TaskGraph";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type {
  AgentArtifactTab,
  AgentTaskArtifactMode,
} from "@/stores/agentSessionStore";
import { useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import type { ChatMessageResponse } from "@/api/chat";
import type {
  TaskProposalResponse,
  VerificationStatusResponse,
} from "@/api/ideation";
import type { Artifact } from "@/types/artifact";
import type { ContentBlockItem } from "@/components/Chat/MessageItem";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import { parseMcpToolResultRaw } from "@/components/Chat/tool-widgets/shared.constants";
import type { AgentConversation } from "./agentConversations";

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
    staleTime: 5_000,
  });
  const sessionData = sessionQuery.data ?? null;
  const planArtifactId =
    sessionData?.session.planArtifactId ?? sessionData?.session.inheritedPlanArtifactId ?? null;
  const planArtifactQuery = useQuery({
    queryKey: ["agents", "artifact", planArtifactId],
    queryFn: () => artifactApi.get(planArtifactId!),
    enabled: !!planArtifactId,
    staleTime: 5_000,
  });
  const verificationQuery = useQuery({
    queryKey: attachedSessionId
      ? ["agents", "ideation-verification", attachedSessionId]
      : ["agents", "ideation-verification", ""],
    queryFn: () => ideationApi.verification.getStatus(attachedSessionId!),
    enabled: !!attachedSessionId && activeTab === "verification",
    staleTime: 5_000,
    refetchInterval: (query) => query.state.data?.inProgress ? 5_000 : false,
  });
  const proposalCount = sessionData?.proposals.length ?? 0;
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
                <PanelRightClose className="w-4 h-4" />
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
          sessionTitle={sessionData?.session.title ?? null}
          taskMode={taskMode}
          planContent={getArtifactText(planArtifactQuery.data)}
          isPlanLoading={planArtifactQuery.isLoading}
          verification={verificationQuery.data ?? null}
          isVerificationLoading={verificationQuery.isLoading}
          proposals={sessionData?.proposals ?? []}
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
  sessionTitle: string | null;
  taskMode: AgentTaskArtifactMode;
  planContent: string | null;
  isPlanLoading: boolean;
  verification: VerificationStatusResponse | null;
  isVerificationLoading: boolean;
  proposals: TaskProposalResponse[];
};

function ArtifactContent({
  activeTab,
  isLoading,
  attachedSessionId,
  projectId,
  sessionTitle,
  taskMode,
  planContent,
  isPlanLoading,
  verification,
  isVerificationLoading,
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
    if (isVerificationLoading) {
      return <EmptyArtifactState title="Loading verification..." />;
    }
    if (!verification) {
      return <EmptyArtifactState title="No verification data yet" />;
    }
    return <VerificationSummary verification={verification} />;
  }

  if (activeTab === "proposal") {
    if (proposals.length === 0) {
      return <EmptyArtifactState title="No proposals yet" />;
    }
    return <ProposalSummary proposals={proposals} />;
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

function VerificationSummary({
  verification,
}: {
  verification: VerificationStatusResponse;
}) {
  return (
    <div className="p-5 space-y-4">
      <div className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-4">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-[var(--text-primary)]">
            {verification.status.replace("_", " ")}
          </span>
          {verification.inProgress && (
            <span className="rounded-full bg-[var(--status-success-muted)] px-2 py-0.5 text-[11px] font-medium text-[var(--status-success)]">
              Running
            </span>
          )}
        </div>
        <div className="mt-2 text-xs text-[var(--text-muted)]">
          Round {verification.currentRound ?? 0}/{verification.maxRounds ?? 0}
          {typeof verification.gapScore === "number" ? ` · Gap score ${verification.gapScore}` : ""}
        </div>
        {verification.convergenceReason && (
          <MarkdownBody
            compact
            content={verification.convergenceReason}
            className="mt-3 text-sm text-[var(--text-secondary)]"
          />
        )}
      </div>

      {verification.gaps.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">Gaps</h3>
          {verification.gaps.map((gap, index) => (
            <div key={`${gap.severity}-${index}`} className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-3">
              <div className="flex items-center gap-2">
                <div className="text-xs font-medium text-[var(--accent-primary)]">{gap.severity}</div>
                <div className="text-xs text-[var(--text-muted)]">{gap.category}</div>
              </div>
              <MarkdownBody compact content={gap.description} className="mt-1 text-sm text-[var(--text-secondary)]" />
              {gap.whyItMatters && (
                <MarkdownBody compact content={gap.whyItMatters} className="mt-2 text-xs text-[var(--text-muted)]" />
              )}
            </div>
          ))}
        </div>
      )}

      {verification.roundDetails.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">Rounds</h3>
          {verification.roundDetails.map((round) => (
            <div key={round.round} className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-3">
              <div className="flex items-center justify-between gap-3 text-xs">
                <span className="font-medium text-[var(--text-primary)]">Round {round.round}</span>
                <span className="text-[var(--text-muted)]">
                  {round.gapCount} gaps · score {round.gapScore}
                </span>
              </div>
              {round.gaps.slice(0, 3).map((gap, index) => (
                <MarkdownBody
                  key={`${round.round}-${gap.severity}-${index}`}
                  compact
                  content={gap.description}
                  className="mt-2 text-xs text-[var(--text-secondary)]"
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ProposalSummary({ proposals }: { proposals: ArtifactContentProps["proposals"] }) {
  return (
    <div className="p-5 space-y-3">
      {proposals.map((proposal) => (
        <div key={proposal.id} className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-4">
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-sm font-semibold text-[var(--text-primary)]">{proposal.title}</h3>
            <span className="rounded-full bg-[var(--bg-hover)] px-2 py-0.5 text-[11px] text-[var(--text-muted)]">
              {proposal.suggestedPriority}
            </span>
          </div>
          {proposal.description && (
            <MarkdownBody content={proposal.description} className="mt-2 text-sm leading-relaxed text-[var(--text-secondary)]" />
          )}
          {proposal.steps.length > 0 && (
            <div className="mt-3">
              <div className="mb-1.5 text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">
                Proposal Tasks
              </div>
              <ol className="list-decimal space-y-1 pl-4 text-sm text-[var(--text-secondary)]">
                {proposal.steps.map((step, index) => (
                  <li key={`${proposal.id}-step-${index}`}>
                    <MarkdownBody compact content={step} />
                  </li>
                ))}
              </ol>
            </div>
          )}
          {proposal.acceptanceCriteria.length > 0 && (
            <div className="mt-3">
              <div className="mb-1.5 text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">
                Acceptance
              </div>
              <ul className="list-disc space-y-1 pl-4 text-sm text-[var(--text-secondary)]">
                {proposal.acceptanceCriteria.map((criterion, index) => (
                  <li key={`${proposal.id}-criterion-${index}`}>
                    <MarkdownBody compact content={criterion} />
                  </li>
                ))}
              </ul>
            </div>
          )}
          {proposal.createdTaskId && (
            <div className="mt-3 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-base)] px-3 py-2">
              <div className="text-xs font-medium text-[var(--text-muted)]">Kanban task</div>
              <div className="mt-1 font-mono text-[11px] text-[var(--text-secondary)]">
                {proposal.createdTaskId}
              </div>
            </div>
          )}
        </div>
      ))}
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

  return <TaskGraphSummary projectId={projectId} sessionId={sessionId} />;
}

function TaskGraphSummary({
  projectId,
  sessionId,
}: {
  projectId: string;
  sessionId: string;
}) {
  const graphQuery = useTaskGraph(projectId, false, null, sessionId);

  if (graphQuery.isLoading) {
    return <EmptyArtifactState title="Loading graph..." />;
  }
  if (graphQuery.error) {
    return <EmptyArtifactState title="Failed to load graph" detail={graphQuery.error.message} />;
  }

  const graph = graphQuery.data;
  if (!graph || graph.nodes.length === 0) {
    return (
      <EmptyArtifactState
        title="No graph tasks yet"
        detail="Apply proposals to Kanban to populate the graph for this plan."
      />
    );
  }

  const nodeTitleById = new Map(graph.nodes.map((node) => [node.taskId, node.title]));

  return (
    <div className="p-5 space-y-4">
      <div className="grid grid-cols-3 gap-2">
        <MetricCard label="Tasks" value={graph.nodes.length} />
        <MetricCard label="Edges" value={graph.edges.length} />
        <MetricCard label="Critical" value={graph.criticalPath.length} />
      </div>

      <div className="space-y-2">
        <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">Task Graph</h3>
        {graph.nodes.map((node) => (
          <div key={node.taskId} className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0 text-sm font-medium text-[var(--text-primary)]">
                {node.title}
              </div>
              <span className="shrink-0 rounded-full bg-[var(--bg-hover)] px-2 py-0.5 text-[11px] text-[var(--text-muted)]">
                {node.internalStatus}
              </span>
            </div>
            <div className="mt-1 flex flex-wrap gap-2 text-[11px] text-[var(--text-muted)]">
              <span>Tier {node.tier}</span>
              <span>{node.inDegree} in</span>
              <span>{node.outDegree} out</span>
              <span className="font-mono">{node.taskId}</span>
            </div>
            {node.description && (
              <MarkdownBody compact content={node.description} className="mt-2 text-xs text-[var(--text-secondary)]" />
            )}
          </div>
        ))}
      </div>

      {graph.edges.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">Dependencies</h3>
          {graph.edges.map((edge) => (
            <div key={`${edge.source}-${edge.target}`} className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-base)] px-3 py-2 text-xs text-[var(--text-secondary)]">
              <span>{nodeTitleById.get(edge.source) ?? edge.source}</span>
              <span className="mx-2 text-[var(--text-muted)]">→</span>
              <span>{nodeTitleById.get(edge.target) ?? edge.target}</span>
              {edge.isCriticalPath && (
                <span className="ml-2 rounded-full bg-[var(--status-warning-muted)] px-1.5 py-0.5 text-[10px] text-[var(--status-warning)]">
                  critical
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-elevated)] p-3">
      <div className="text-[11px] font-medium uppercase tracking-[0.08em] text-[var(--text-muted)]">{label}</div>
      <div className="mt-1 text-lg font-semibold text-[var(--text-primary)]">{value}</div>
    </div>
  );
}

function resolveAttachedIdeationSessionId(
  conversation: AgentConversation | null,
  messages: ChatMessageResponse[],
): string | null {
  if (!conversation) {
    return null;
  }
  if (conversation.contextType === "ideation") {
    return conversation.contextId;
  }

  for (const message of [...messages].reverse()) {
    const toolCalls = [
      ...(message.toolCalls ?? []),
      ...(message.contentBlocks ?? [])
        .filter((block): block is ContentBlockItem & { type: "tool_use" } => block.type === "tool_use")
        .map((block) => ({
          id: block.id ?? "",
          name: block.name ?? "",
          arguments: block.arguments,
          result: block.result,
        })),
    ];
    for (const toolCall of toolCalls.reverse()) {
      const sessionId = extractAttachedSessionId(toolCall);
      if (sessionId) {
        return sessionId;
      }
    }
  }

  return null;
}

function extractAttachedSessionId(toolCall: ToolCall): string | null {
  const name = toolCall.name.toLowerCase();
  if (
    !name.includes("start_ideation_session") &&
    !name.includes("v1_start_ideation") &&
    !name.includes("create_child_session")
  ) {
    return null;
  }
  return extractSessionIdFromValue(toolCall.result);
}

function extractSessionIdFromValue(value: unknown): string | null {
  const parsed = parseMcpToolResultRaw(value);
  if (parsed !== null) {
    const parsedSessionId = extractSessionIdFromParsedValue(parsed);
    if (parsedSessionId) {
      return parsedSessionId;
    }
  }
  return extractSessionIdFromParsedValue(value);
}

function extractSessionIdFromParsedValue(value: unknown): string | null {
  if (!value) {
    return null;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const nested = extractSessionIdFromValue(item);
      if (nested) {
        return nested;
      }
    }
    return null;
  }
  if (typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.session_id === "string") {
      return record.session_id;
    }
    if (typeof record.sessionId === "string") {
      return record.sessionId;
    }
    if (typeof record.child_session_id === "string") {
      return record.child_session_id;
    }
    if (typeof record.childSessionId === "string") {
      return record.childSessionId;
    }
    for (const nestedKey of [
      "result",
      "data",
      "session",
      "ideation_session",
      "structured_content",
      "structuredContent",
      "content",
    ]) {
      const nested = extractSessionIdFromValue(record[nestedKey]);
      if (nested) {
        return nested;
      }
    }
    if (typeof record.text === "string") {
      try {
        return extractSessionIdFromValue(JSON.parse(record.text));
      } catch {
        return null;
      }
    }
  }
  return null;
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
