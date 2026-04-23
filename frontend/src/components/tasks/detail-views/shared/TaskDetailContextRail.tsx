import type { ReactNode } from "react";
import {
  Clock,
  ExternalLink,
  FileText,
  GitBranch,
  GitCommit,
  GitPullRequest,
  Lightbulb,
} from "lucide-react";
import { BranchBadge, BranchFlow } from "@/components/shared/BranchBadge";
import { getTaskCategoryLabel } from "@/lib/task-category";
import type { InternalStatus } from "@/types/task";
import { DescriptionBlock } from "./DescriptionBlock";
import { DetailCard } from "./DetailCard";
import { PrStatusBadge } from "./PrStatusBadge";
import { SectionTitle } from "./SectionTitle";
import type {
  TaskDetailContextModel,
  TaskDetailViewMode,
} from "./TaskDetailContext";

const STATUS_LABELS: Partial<Record<InternalStatus, string>> = {
  pending_review: "Pending Review",
  qa_refining: "QA Refining",
  qa_testing: "QA Testing",
  qa_passed: "QA Passed",
  qa_failed: "QA Failed",
  review_passed: "AI Review Passed",
  waiting_on_pr: "Waiting on PR",
  merge_incomplete: "Merge Incomplete",
  merge_conflict: "Merge Conflict",
};

function titleCaseStatus(status: InternalStatus): string {
  const knownLabel = STATUS_LABELS[status];
  if (knownLabel) return knownLabel;

  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

function shortSha(value: string): string {
  return value.slice(0, 7);
}

function RailSection({
  title,
  children,
}: {
  title: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="space-y-2">
      <SectionTitle muted>{title}</SectionTitle>
      {children}
    </section>
  );
}

function HistoricalLensCard({ viewMode }: { viewMode: TaskDetailViewMode }) {
  if (viewMode.kind !== "historical") return null;

  return (
    <RailSection title="View">
      <DetailCard variant="info">
        <div className="space-y-2.5">
          <div className="flex items-center gap-2">
            <Clock className="w-4 h-4 text-text-primary/45 shrink-0" />
            <div className="min-w-0">
              <div className="text-[13px] font-medium text-text-primary/80">
                Historical State
              </div>
              <div className="text-[12px] text-text-primary/45">
                {titleCaseStatus(viewMode.status)}
              </div>
              <div className="text-[12px] text-text-primary/35">
                {formatTimestamp(viewMode.timestamp)}
              </div>
            </div>
          </div>
          <p className="text-[12px] leading-relaxed text-text-primary/45">
            Plan, branch, and PR values show the latest task context.
          </p>
        </div>
      </DetailCard>
    </RailSection>
  );
}

function PlanCard({ model }: { model: TaskDetailContextModel }) {
  const { taskContext, sessionId } = model;
  const planArtifact = taskContext?.planArtifact;
  const sourceProposal = taskContext?.sourceProposal;

  if (!planArtifact && !sourceProposal && !sessionId) return null;

  return (
    <RailSection title="Plan">
      <DetailCard>
        <div className="space-y-3">
          {planArtifact && (
            <div className="flex gap-2.5">
              <FileText className="w-4 h-4 mt-0.5 text-text-primary/40 shrink-0" />
              <div className="min-w-0">
                <div className="text-[13px] font-medium leading-snug text-text-primary/80">
                  {planArtifact.title}
                </div>
                {planArtifact.contentPreview && (
                  <p className="mt-1 text-[12px] leading-relaxed text-text-primary/45">
                    {planArtifact.contentPreview}
                  </p>
                )}
                <div className="mt-2 flex flex-wrap items-center gap-1.5">
                  <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-[var(--overlay-weak)] text-text-primary/45">
                    v{planArtifact.currentVersion}
                  </span>
                  <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-[var(--overlay-weak)] text-text-primary/45">
                    {planArtifact.artifactType}
                  </span>
                </div>
              </div>
            </div>
          )}

          {sourceProposal && (
            <div
              className="flex gap-2.5 pt-3 border-t"
              style={{ borderColor: "var(--overlay-weak)" }}
            >
              <Lightbulb className="w-4 h-4 mt-0.5 text-text-primary/40 shrink-0" />
              <div className="min-w-0">
                <div className="text-[11px] uppercase tracking-wider text-text-primary/35">
                  Source Proposal
                </div>
                <div className="mt-1 text-[13px] font-medium leading-snug text-text-primary/75">
                  {sourceProposal.title}
                </div>
                {sourceProposal.description && (
                  <p className="mt-1 text-[12px] leading-relaxed text-text-primary/45">
                    {sourceProposal.description}
                  </p>
                )}
              </div>
            </div>
          )}

          {!planArtifact && !sourceProposal && sessionId && (
            <div className="text-[12px] text-text-primary/45">
              Ideation session {sessionId.slice(0, 8)}
            </div>
          )}
        </div>
      </DetailCard>
    </RailSection>
  );
}

function BranchCard({
  branch,
}: {
  branch: NonNullable<TaskDetailContextModel["branch"]>;
}) {
  return (
    <RailSection title={branch.label}>
      <DetailCard>
        <div className="flex items-center gap-2.5 min-w-0">
          <GitBranch className="w-4 h-4 text-text-primary/40 shrink-0" />
          <div className="min-w-0">
            {branch.target ? (
              <BranchFlow source={branch.source} target={branch.target} size="sm" />
            ) : (
              <BranchBadge branch={branch.source} variant="muted" size="sm" />
            )}
            {branch.status && (
              <div className="mt-1 text-[11px] capitalize text-text-primary/35">
                {branch.status.replace(/_/g, " ")}
              </div>
            )}
          </div>
        </div>
      </DetailCard>
    </RailSection>
  );
}

function PullRequestCard({
  pullRequest,
}: {
  pullRequest: NonNullable<TaskDetailContextModel["pullRequest"]>;
}) {
  const handleOpen = async () => {
    if (!pullRequest.url) return;
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(pullRequest.url);
  };

  return (
    <RailSection title="Pull Request">
      <DetailCard variant={pullRequest.status === "Merged" ? "success" : "default"}>
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0">
            <GitPullRequest className="w-4 h-4 text-text-primary/45 shrink-0" />
            <span className="text-[13px] font-medium text-text-primary/80">
              PR #{pullRequest.number}
            </span>
          </div>
          {pullRequest.status && <PrStatusBadge status={pullRequest.status} />}
        </div>
        {pullRequest.url && (
          <button
            type="button"
            onClick={handleOpen}
            className="mt-3 inline-flex items-center gap-1.5 text-[12px] font-medium text-status-success hover:text-status-success/85"
          >
            <ExternalLink className="w-3.5 h-3.5" />
            View PR
          </button>
        )}
      </DetailCard>
    </RailSection>
  );
}

function MergeCard({
  merge,
}: {
  merge: NonNullable<TaskDetailContextModel["merge"]>;
}) {
  return (
    <RailSection title="Merge">
      <DetailCard variant="success">
        <div className="space-y-2.5">
          {merge.target && (
            <div className="flex items-center gap-2">
              <GitBranch className="w-4 h-4 text-text-primary/40 shrink-0" />
              <div className="min-w-0">
                <div className="text-[11px] uppercase tracking-wider text-text-primary/35">
                  Target
                </div>
                <BranchBadge branch={merge.target} variant="target" size="sm" />
              </div>
            </div>
          )}
          {merge.commitSha && (
            <div className="flex items-center gap-2">
              <GitCommit className="w-4 h-4 text-text-primary/40 shrink-0" />
              <div>
                <div className="text-[11px] uppercase tracking-wider text-text-primary/35">
                  Commit
                </div>
                <span className="text-[12px] font-mono text-text-primary/70">
                  {shortSha(merge.commitSha)}
                </span>
              </div>
            </div>
          )}
          {merge.mergedAt && (
            <div className="text-[12px] text-text-primary/40">
              Merged {formatTimestamp(merge.mergedAt)}
            </div>
          )}
        </div>
      </DetailCard>
    </RailSection>
  );
}

export function TaskContextRail({
  model,
  fallbackDescription,
}: {
  model: TaskDetailContextModel;
  fallbackDescription?: string | null | undefined;
}) {
  const description = model.task.description ?? fallbackDescription;
  const categoryLabel = getTaskCategoryLabel(model.task.category);

  return (
    <div className="space-y-5">
      <RailSection title="Task">
        <div className="space-y-2">
          {categoryLabel && (
            <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-[var(--overlay-weak)] text-[11px] font-medium text-text-primary/45">
              <FileText className="w-3.5 h-3.5" />
              {categoryLabel}
            </div>
          )}
          <DescriptionBlock description={description} />
        </div>
      </RailSection>

      <HistoricalLensCard viewMode={model.viewMode} />
      <PlanCard model={model} />
      {model.branch && <BranchCard branch={model.branch} />}
      {model.pullRequest && <PullRequestCard pullRequest={model.pullRequest} />}
      {model.merge && <MergeCard merge={model.merge} />}
    </div>
  );
}
