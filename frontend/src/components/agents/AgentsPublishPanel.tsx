import {
  CheckCircle2,
  Code,
  ExternalLink,
  FileText,
  GitPullRequestArrow,
  GitBranch,
  Loader2,
  X,
} from "lucide-react";
import type { ElementType } from "react";
import { lazy, Suspense, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";

import { diffApi } from "@/api/diff";
import {
  chatApi,
  type AgentConversationWorkspace,
  type AgentConversationWorkspacePublicationEvent,
} from "@/api/chat";
import type { FileChange as DiffViewerFileChange } from "@/components/diff";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useDeferredAgentHydration } from "./useDeferredAgentHydration";
import { EmptyArtifactState } from "./AgentsArtifactEmptyState";

const LazyDiffViewer = lazy(() =>
  import("@/components/diff").then((module) => ({ default: module.DiffViewer })),
);
export function AgentPublishPanel({
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
  const canHydratePublishFacts = useDeferredAgentHydration(conversationId);
  const changesQuery = useQuery({
    queryKey: ["agents", "workspace-diff", conversationId],
    queryFn: () => diffApi.getAgentConversationWorkspaceFileChanges(conversationId!),
    enabled: canHydratePublishFacts && !!conversationId,
    staleTime: 2_000,
  });
  const publicationEventsQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-publication-events", conversationId],
    queryFn: () =>
      chatApi.listAgentConversationWorkspacePublicationEvents(conversationId!),
    enabled: canHydratePublishFacts && !!conversationId,
    staleTime: 0,
    refetchInterval: isPublishingWorkspace ? 1_500 : false,
  });
  const freshnessQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-freshness", conversationId],
    queryFn: () => chatApi.getAgentConversationWorkspaceFreshness(conversationId!),
    enabled: canHydratePublishFacts && !!conversationId && workspace?.mode === "edit",
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
  const isChangesLoading =
    Boolean(conversationId) && (!canHydratePublishFacts || changesQuery.isLoading);
  const isPublicationEventsLoading =
    Boolean(conversationId) &&
    (!canHydratePublishFacts || publicationEventsQuery.isLoading);

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
                {isChangesLoading
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
                disabled={isChangesLoading || changes.length === 0}
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
          isLoading={isPublicationEventsLoading}
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
          {reviewOpen && (
            <Suspense fallback={<EmptyArtifactState title="Loading workspace diff..." />}>
              <LazyDiffViewer
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
            </Suspense>
          )}
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
