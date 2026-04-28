import {
  CheckCircle2,
  Code,
  FileText,
  GitPullRequestArrow,
  GitBranch,
  Loader2,
} from "lucide-react";
import { lazy, Suspense, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";

import { diffApi } from "@/api/diff";
import {
  chatApi,
  type AgentConversationWorkspace,
} from "@/api/chat";
import type {
  Commit as DiffViewerCommit,
  FileChange as DiffViewerFileChange,
} from "@/components/diff";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { useDeferredAgentHydration } from "./useDeferredAgentHydration";
import { EmptyArtifactState } from "./AgentsArtifactEmptyState";
import { PublishEventLog } from "./AgentsPublishEventLog";
import { PublishFact } from "./AgentsPublishFact";
import { PublishPipelineSteps } from "./AgentsPublishPipelineSteps";
import { formatPullRequestUrlLabel } from "./agentPublishFormatting";
import {
  getAgentWorkspaceTerminalPublicationLabel,
  getAgentWorkspaceTerminalPublicationStatus,
  isPipelineOwnedAgentWorkspace,
  isAgentWorkspacePublishCurrent,
} from "./agentWorkspacePublishState";

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
  const [isLoadingCommitFiles, setIsLoadingCommitFiles] = useState(false);
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
  const commitsQuery = useQuery({
    queryKey: ["agents", "workspace-commits", conversationId],
    queryFn: async (): Promise<DiffViewerCommit[]> => {
      const commits = await diffApi.getAgentConversationWorkspaceCommits(
        conversationId!,
      );
      return commits
        .map((commit) => ({
          sha: commit.sha,
          shortSha: commit.shortSha,
          message: commit.message,
          author: commit.author,
          date: commit.date,
        }))
        .reverse();
    },
    enabled: canHydratePublishFacts && !!conversationId && reviewOpen,
    staleTime: 2_000,
  });
  const terminalPublicationStatus =
    getAgentWorkspaceTerminalPublicationStatus(workspace);
  const terminalPublicationLabel =
    getAgentWorkspaceTerminalPublicationLabel(workspace);
  const isPipelineOwnedWorkspace = isPipelineOwnedAgentWorkspace(workspace);
  const freshnessQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-freshness", conversationId],
    queryFn: () => chatApi.getAgentConversationWorkspaceFreshness(conversationId!),
    enabled:
      canHydratePublishFacts &&
      !!conversationId &&
      workspace?.mode === "edit" &&
      !terminalPublicationStatus,
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
        queryClient.invalidateQueries({
          queryKey: ["agents", "workspace-commits", result.workspace.conversationId],
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
  const commits = commitsQuery.data ?? [];
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
  const isBranchUpdateNeeded =
    !terminalPublicationStatus && Boolean(freshness?.isBaseAhead);
  const isPublishCurrent = isAgentWorkspacePublishCurrent(workspace, freshness);
  const isUpdatingFromBase = updateFromBaseMutation.isPending;
  const effectivePublishing = isPublishingWorkspace || isUpdatingFromBase;
  const isRepairPending = workspace.publicationPushStatus === "needs_agent";
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
    !onPublishWorkspace ||
    isPipelineOwnedWorkspace ||
    effectivePublishing ||
    isRepairPending ||
    isPublishCurrent ||
    Boolean(terminalPublicationStatus) ||
    workspace.status === "missing";
  const publishButtonLabel =
    terminalPublicationLabel ??
    (isPipelineOwnedWorkspace
      ? "Managed by Tasks"
      : isPublishCurrent
        ? "Published"
        : "Commit & Publish");
  const terminalPrLabel =
    workspace.publicationPrNumber != null
      ? `PR #${workspace.publicationPrNumber}`
      : "This pull request";
  const publishSummary =
    terminalPublicationStatus === "merged"
      ? `${terminalPrLabel} has been merged. By continuing this conversation, a new workspace branch will be created automatically.`
      : terminalPublicationStatus === "closed"
        ? `${terminalPrLabel} is closed. By continuing this conversation, a new workspace branch will be created automatically.`
        : isPipelineOwnedWorkspace
          ? workspace.publicationPrNumber || workspace.publicationPrUrl
            ? `${terminalPrLabel} is managed by this ideation plan's task pipeline.`
            : "Publishing is managed by this ideation plan's task pipeline."
        : isChangesLoading
          ? "Loading changed files..."
          : isPublishCurrent
            ? changes.length > 0
              ? `${changes.length} changed file${changes.length === 1 ? "" : "s"} published for review.`
              : "Workspace is published and current."
            : changes.length > 0
              ? `${changes.length} changed file${changes.length === 1 ? "" : "s"} ready for review.`
              : "No changed files detected yet.";

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
                {isPipelineOwnedWorkspace
                  ? "Review this ideation workspace's execution branch and pull request."
                  : "Review this agent workspace before publishing its draft PR."}
              </div>
            </div>
            <span
              className="rounded-full border px-2.5 py-1 text-xs capitalize"
              style={{
                borderColor: "var(--overlay-weak)",
                color: "var(--text-secondary)",
              }}
            >
              {terminalPublicationLabel ??
                workspace.publicationPushStatus ??
                workspace.status}
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
              value={
                workspace.mode === "edit"
                  ? "Edit agent"
                  : isPipelineOwnedWorkspace
                    ? "Ideation plan"
                    : workspace.mode
              }
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
                {terminalPublicationLabel
                  ? `Pull Request ${terminalPublicationLabel}`
                  : "Commit & Publish"}
              </div>
              <div className="mt-1 text-xs leading-relaxed text-[var(--text-muted)]">
                {publishSummary}
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
                  disabled={
                    effectivePublishing ||
                    isRepairPending ||
                    workspace.status === "missing"
                  }
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
                  ) : isPublishCurrent || terminalPublicationStatus ? (
                    <CheckCircle2 className="h-3.5 w-3.5" />
                  ) : (
                    <GitPullRequestArrow className="h-3.5 w-3.5" />
                  )}
                  {publishButtonLabel}
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
                commits={commits}
                commitFiles={commitFiles}
                onFetchDiff={async (filePath, commitSha) => {
                  if (!conversationId) {
                    return null;
                  }
                  const diff = commitSha
                    ? await diffApi.getAgentConversationWorkspaceCommitFileDiff(
                        conversationId,
                        commitSha,
                        filePath,
                      )
                    : await diffApi.getAgentConversationWorkspaceFileDiff(
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
                onFetchCommitFiles={async (commitSha) => {
                  if (!conversationId) {
                    setCommitFiles([]);
                    return;
                  }
                  setIsLoadingCommitFiles(true);
                  setCommitFiles([]);
                  try {
                    setCommitFiles(
                      await diffApi.getAgentConversationWorkspaceCommitFileChanges(
                        conversationId,
                        commitSha,
                      ),
                    );
                  } catch {
                    setCommitFiles([]);
                  } finally {
                    setIsLoadingCommitFiles(false);
                  }
                }}
                isLoadingChanges={changesQuery.isLoading}
                isLoadingHistory={commitsQuery.isLoading}
                isLoadingCommitFiles={isLoadingCommitFiles}
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
