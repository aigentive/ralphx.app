/**
 * MergingTaskDetail - Handles both pending_merge and merging states
 *
 * pending_merge: Programmatic merge attempt in progress (Phase 1)
 * merging: Agent-assisted conflict resolution in progress (Phase 2)
 *
 * This combined view provides a seamless UX during the merge process.
 * PendingMerge is typically very brief (1-3 seconds).
 */

import { Loader2, GitMerge, AlertTriangle, Bot, FileWarning, CheckCircle2 } from "lucide-react";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import type { Task } from "@/types/task";

interface MergingTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
  viewStatus?: string | undefined;
}

/**
 * ConflictFilesList - Shows files with merge conflicts
 */
function ConflictFilesList({ files }: { files: string[] }) {
  if (files.length === 0) return null;

  return (
    <div className="space-y-2">
      {files.map((file, index) => (
        <div
          key={index}
          className="flex items-center gap-2 py-2 px-3 rounded-lg"
          style={{ backgroundColor: "rgba(255, 159, 10, 0.08)" }}
        >
          <FileWarning className="w-4 h-4" style={{ color: "#ff9f0a" }} />
          <span
            className="text-[13px] font-mono text-white/70 truncate"
            title={file}
          >
            {file}
          </span>
        </div>
      ))}
    </div>
  );
}

/**
 * MergeProgressSteps - Shows progress through merge phases
 */
function MergeProgressSteps({
  isProgrammaticPhase,
  isHistorical,
  historicalMode,
}: {
  isProgrammaticPhase: boolean;
  isHistorical?: boolean | undefined;
  historicalMode?: "attempted" | "resolving" | undefined;
}) {
  const steps = isHistorical
    ? historicalMode === "attempted"
      ? ([
          { label: "Fetching latest changes", status: "completed" },
          { label: "Rebasing onto base branch", status: "completed" },
          { label: "Resolving conflicts", status: "pending" },
          { label: "Completing merge", status: "pending" },
        ] as const)
      : ([
          { label: "Fetching latest changes", status: "completed" },
          { label: "Rebasing onto base branch", status: "completed" },
          { label: "Resolving conflicts", status: "active" },
          { label: "Completing merge", status: "pending" },
        ] as const)
    : ([
        {
          label: "Fetching latest changes",
          status: isProgrammaticPhase ? "active" : "completed",
        },
        {
          label: "Rebasing onto base branch",
          status: isProgrammaticPhase ? "pending" : "completed",
        },
        {
          label: "Resolving conflicts",
          status: isProgrammaticPhase ? "pending" : "active",
        },
        {
          label: "Completing merge",
          status: "pending",
        },
      ] as const);

  return (
    <div className="divide-y divide-white/5">
      {steps.map((step, index) => (
        <div key={index} className="flex items-center gap-3 py-2.5">
          <div className="relative">
            {step.status === "completed" && (
              <CheckCircle2 className="w-5 h-5" style={{ color: "#34c759" }} />
            )}
            {step.status === "active" && !isHistorical && (
              <div className="relative">
                <Loader2
                  className="w-5 h-5 animate-spin"
                  style={{ color: "#0a84ff" }}
                />
                <div
                  className="absolute inset-0 rounded-full animate-pulse"
                  style={{
                    background: "radial-gradient(circle, rgba(10,132,255,0.3) 0%, transparent 70%)",
                  }}
                />
              </div>
            )}
            {step.status === "active" && isHistorical && (
              <div
                className="w-5 h-5 rounded-full border-2"
                style={{
                  borderColor: "rgba(255,255,255,0.2)",
                  backgroundColor: "rgba(255, 159, 10, 0.35)",
                }}
              />
            )}
            {step.status === "pending" && (
              <div
                className="w-5 h-5 rounded-full border-2"
                style={{ borderColor: "rgba(255,255,255,0.2)" }}
              />
            )}
          </div>
          <span
            className="text-[13px] font-medium"
            style={{
              color:
                step.status === "completed"
                  ? "rgba(255,255,255,0.6)"
                  : step.status === "active"
                  ? isHistorical
                    ? "rgba(255,255,255,0.35)"
                    : "#64d2ff"
                  : "rgba(255,255,255,0.35)",
            }}
          >
            {step.label}
          </span>
        </div>
      ))}
    </div>
  );
}

export function MergingTaskDetail({ task, isHistorical, viewStatus }: MergingTaskDetailProps) {
  const status = isHistorical && viewStatus ? viewStatus : task.internalStatus;
  const isProgrammaticPhase = status === "pending_merge";
  const isAgentPhase = status === "merging";
  const historicalOutcome = isHistorical
    ? task.internalStatus === "merged" || task.internalStatus === "merge_conflict"
      ? task.internalStatus
      : null
    : null;
  const historicalMode = isHistorical
    ? isProgrammaticPhase
      ? "attempted"
      : "resolving"
    : undefined;

  // Parse conflict files from task metadata if available
  const conflictFiles: string[] = (() => {
    if (!task.metadata) return [];
    const metadata = typeof task.metadata === "string"
      ? JSON.parse(task.metadata)
      : task.metadata;
    return Array.isArray(metadata?.conflict_files) ? metadata.conflict_files : [];
  })();

  const branchName = task.taskBranch ?? "task branch";
  const statusLabel = historicalOutcome
    ? historicalOutcome === "merged"
      ? "Merged"
      : "Conflict"
    : isHistorical
    ? isProgrammaticPhase
      ? "Attempted"
      : "Resolving"
    : isProgrammaticPhase
    ? "Merging"
    : "Resolving";
  const statusIcon = historicalOutcome
    ? historicalOutcome === "merged"
      ? CheckCircle2
      : AlertTriangle
    : isHistorical
    ? isProgrammaticPhase
      ? GitMerge
      : AlertTriangle
    : isProgrammaticPhase
    ? GitMerge
    : AlertTriangle;

  return (
    <TwoColumnLayout
      description={task.description}
      testId="merging-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? CheckCircle2
              : AlertTriangle
            : isHistorical
            ? isProgrammaticPhase
              ? GitMerge
              : AlertTriangle
            : isProgrammaticPhase
            ? GitMerge
            : Bot
        }
        title={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Merge Completed"
              : "Merge Conflict"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge Attempted"
              : "Resolving Conflicts"
            : isProgrammaticPhase
            ? "Merging Changes..."
            : "Resolving Merge Conflicts"
        }
        subtitle={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Final outcome: merged into base branch"
              : "Final outcome: manual resolution required"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge attempt captured in history"
              : "Agent was resolving conflicts"
            : isProgrammaticPhase
            ? `Attempting to merge ${branchName}`
            : "AI agent is resolving conflicts"
        }
        variant={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "success"
              : "warning"
            : isHistorical
            ? isProgrammaticPhase
              ? "info"
              : "warning"
            : isProgrammaticPhase
            ? "accent"
            : "warning"
        }
        animated={!isHistorical}
        badge={
          <StatusPill
            icon={statusIcon}
            label={statusLabel}
            variant={
              historicalOutcome
                ? historicalOutcome === "merged"
                  ? "success"
                  : "warning"
                : isHistorical
                ? isProgrammaticPhase
                  ? "info"
                  : "warning"
                : isProgrammaticPhase
                ? "accent"
                : "warning"
            }
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Merge Progress */}
      <section data-testid="merge-progress-section">
        <SectionTitle>{isHistorical ? "Process Details" : "Merge Progress"}</SectionTitle>
        <DetailCard variant="default">
          <p className="text-[12px] text-white/50 mb-3">
            {historicalOutcome
              ? "Process context captured during the merge lifecycle."
              : isHistorical
              ? historicalMode === "attempted"
                ? "Programmatic merge attempt captured in history."
                : "Agent was resolving conflicts at this point."
              : isProgrammaticPhase
              ? "Programmatic merge attempt in progress."
              : "Agent is resolving conflicts; manual resolution may be required."}
          </p>
          <MergeProgressSteps
            isProgrammaticPhase={isProgrammaticPhase}
            isHistorical={isHistorical}
            historicalMode={historicalMode}
          />
        </DetailCard>
      </section>

      {/* Conflict Files (only for agent phase) */}
      {isAgentPhase && conflictFiles.length > 0 && (
        <section data-testid="conflict-files-section">
          <SectionTitle>Conflict Files ({conflictFiles.length})</SectionTitle>
          <DetailCard variant="warning">
            <ConflictFilesList files={conflictFiles} />
          </DetailCard>
        </section>
      )}

      {/* Branch Info */}
      <section data-testid="branch-info-section">
        <SectionTitle muted>Branch</SectionTitle>
        <p className="text-[12px] text-white/50 font-mono">
          {branchName}
        </p>
      </section>
    </TwoColumnLayout>
  );
}
