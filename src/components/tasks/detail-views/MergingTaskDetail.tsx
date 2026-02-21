/**
 * MergingTaskDetail - Handles both pending_merge and merging states
 *
 * pending_merge: Programmatic merge attempt in progress (Phase 1)
 * merging: Agent-assisted conflict resolution in progress (Phase 2)
 *
 * This combined view provides a seamless UX during the merge process.
 * PendingMerge is typically very brief (1-3 seconds).
 */

import { useMemo, useState } from "react";
import {
  Loader2,
  GitMerge,
  AlertTriangle,
  Bot,
  FileWarning,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Wrench,
} from "lucide-react";
import { useConflictDiff } from "@/hooks/useConflictDiff";
import { ConflictDiffViewer } from "@/components/diff/ConflictDiffViewer";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { useMergeValidationEvents } from "@/hooks/useMergeValidationEvents";
import { useMergeProgressEvents } from "@/hooks/useMergeProgressEvents";
import { useConflictDetection } from "@/hooks/useConflictDetection";
import { MergePhaseTimeline } from "./MergePhaseTimeline";
import { ValidationProgress } from "./shared/ValidationProgress";
import type { Task } from "@/types/task";
import { BranchBadge } from "@/components/shared/BranchBadge";

interface MergingTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
  viewStatus?: string | undefined;
}

/**
 * ConflictFilesList - Shows files with merge conflicts, expandable to show diff
 */
function ConflictFilesList({
  files,
  taskId,
}: {
  files: string[];
  taskId: string;
}) {
  const [expandedFile, setExpandedFile] = useState<string | null>(null);
  const { data: conflictDiff, isLoading: isLoadingDiff } = useConflictDiff({
    taskId,
    filePath: expandedFile,
  });

  if (files.length === 0) return null;

  const toggleFile = (file: string) => {
    setExpandedFile(expandedFile === file ? null : file);
  };

  return (
    <div className="space-y-2">
      {files.map((file, index) => (
        <div key={index}>
          <button
            type="button"
            onClick={() => toggleFile(file)}
            className="w-full flex items-center gap-2 py-2 px-3 rounded-lg transition-colors cursor-pointer"
            style={{
              backgroundColor:
                expandedFile === file
                  ? "rgba(255, 159, 10, 0.15)"
                  : "rgba(255, 159, 10, 0.08)",
            }}
          >
            {expandedFile === file ? (
              <ChevronDown
                className="w-4 h-4 shrink-0"
                style={{ color: "#ff9f0a" }}
              />
            ) : (
              <ChevronRight
                className="w-4 h-4 shrink-0"
                style={{ color: "#ff9f0a" }}
              />
            )}
            <FileWarning className="w-4 h-4 shrink-0" style={{ color: "#ff9f0a" }} />
            <span
              className="text-[13px] font-mono text-white/70 truncate text-left"
              title={file}
            >
              {file}
            </span>
          </button>
          {expandedFile === file && (
            <div
              className="mt-2 rounded-lg overflow-hidden border"
              style={{
                borderColor: "rgba(255, 159, 10, 0.2)",
                height: "400px",
              }}
            >
              {isLoadingDiff ? (
                <div
                  className="flex items-center justify-center h-full"
                  style={{ backgroundColor: "hsl(220 10% 8%)" }}
                >
                  <Loader2 className="w-5 h-5 animate-spin text-white/50" />
                </div>
              ) : conflictDiff ? (
                <ConflictDiffViewer conflictDiff={conflictDiff} />
              ) : (
                <div
                  className="flex items-center justify-center h-full text-white/50"
                  style={{ backgroundColor: "hsl(220 10% 8%)" }}
                >
                  Failed to load conflict diff
                </div>
              )}
            </div>
          )}
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
  // Detect validation recovery mode from task metadata
  const isValidationRecovery = useMemo(() => {
    if (!task.metadata) return false;
    try {
      const parsed = typeof task.metadata === "string" ? JSON.parse(task.metadata) : task.metadata;
      return parsed?.validation_recovery === true;
    } catch {
      return false;
    }
  }, [task.metadata]);

  // Detect re-validating state from task metadata (set when fixer agent completes, cleared after re-validation)
  const isRevalidating = useMemo(() => {
    if (!task.metadata) return false;
    try {
      const parsed = typeof task.metadata === "string" ? JSON.parse(task.metadata) : task.metadata;
      return parsed?.revalidating === true;
    } catch {
      return false;
    }
  }, [task.metadata]);

  // Live validation events (only meaningful during active pending_merge)
  const liveSteps = useMergeValidationEvents(task.id);

  // High-level merge progress phases (dynamic from project analysis)
  const { phases: mergePhases, phaseList } = useMergeProgressEvents(task.id);

  // Parse conflict files from task metadata (for historical view or fallback)
  const metadataConflicts: string[] = useMemo(() => {
    if (!task.metadata) return [];
    const metadata = typeof task.metadata === "string"
      ? JSON.parse(task.metadata)
      : task.metadata;
    return Array.isArray(metadata?.conflict_files) ? metadata.conflict_files : [];
  }, [task.metadata]);

  // Live conflict detection (only active for non-historical views)
  const {
    conflicts: liveConflicts,
    isLoading: isLoadingConflicts,
    isEnabled: isConflictDetectionEnabled,
  } = useConflictDetection({
    taskId: task.id,
    internalStatus: status,
    isHistorical: isHistorical ?? false,
    hasBranch: !!task.taskBranch,
  });

  // Hybrid data source: use live conflicts for active states, metadata for historical
  const conflictFiles: string[] = isHistorical
    ? metadataConflicts
    : (isConflictDetectionEnabled && liveConflicts.length > 0 ? liveConflicts : metadataConflicts);

  const branchName = task.taskBranch ?? "task branch";

  const statusLabel = historicalOutcome
    ? historicalOutcome === "merged"
      ? "Merged"
      : "Conflict"
    : isHistorical
    ? isProgrammaticPhase
      ? "Attempted"
      : isValidationRecovery ? "Fixing" : "Resolving"
    : isProgrammaticPhase
    ? "Merging"
    : isValidationRecovery ? "Fixing" : "Resolving";
  const statusIcon = historicalOutcome
    ? historicalOutcome === "merged"
      ? CheckCircle2
      : AlertTriangle
    : isHistorical
    ? isProgrammaticPhase
      ? GitMerge
      : isValidationRecovery ? Wrench : AlertTriangle
    : isProgrammaticPhase
    ? GitMerge
    : isValidationRecovery ? Wrench : AlertTriangle;

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
              : isValidationRecovery ? Wrench : AlertTriangle
            : isProgrammaticPhase
            ? GitMerge
            : isValidationRecovery ? Wrench : Bot
        }
        title={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Merge Completed"
              : "Merge Conflict"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge Attempted"
              : isValidationRecovery ? "Fixing Validation Errors" : "Resolving Conflicts"
            : isProgrammaticPhase
            ? "Merging Changes..."
            : isValidationRecovery ? "Fixing Validation Errors..." : "Resolving Merge Conflicts"
        }
        subtitle={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Final outcome: merged into base branch"
              : "Final outcome: manual resolution required"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge attempt captured in history"
              : isValidationRecovery ? "Agent was fixing build errors" : "Agent was resolving conflicts"
            : isProgrammaticPhase
            ? "Attempting to merge..."
            : isValidationRecovery ? "AI agent is fixing build errors" : "AI agent is resolving conflicts"
        }
        variant={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "success"
              : "warning"
            : isHistorical
            ? isProgrammaticPhase
              ? "info"
              : isValidationRecovery ? "accent" : "warning"
            : isProgrammaticPhase
            ? "accent"
            : isValidationRecovery ? "accent" : "warning"
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
                  : isValidationRecovery ? "accent" : "warning"
                : isProgrammaticPhase
                ? "accent"
                : isValidationRecovery ? "accent" : "warning"
            }
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Merge Progress — single unified section using MergePhaseTimeline */}
      {!isHistorical && (isProgrammaticPhase || isRevalidating) && (
        mergePhases.length > 0 ? (
          <MergePhaseTimeline phases={mergePhases} phaseList={phaseList} />
        ) : (
          <section data-testid="merge-resuming-section">
            <SectionTitle>Merge Progress</SectionTitle>
            <DetailCard>
              <div className="flex items-center gap-2.5 py-1.5">
                <Loader2 className="w-4 h-4 animate-spin" style={{ color: "#0a84ff" }} />
                <span className="text-[13px] text-white/50">
                  Waiting for merge progress...
                </span>
              </div>
            </DetailCard>
          </section>
        )
      )}

      {/* Validation Progress — live events in live mode, metadata fallback in historical mode */}
      <ValidationProgress
        taskId={task.id}
        metadata={isHistorical ? task.metadata : null}
        liveSteps={isHistorical ? undefined : liveSteps}
      />

      {/* Conflict Files (only for agent phase, non-recovery) */}
      {isAgentPhase && !isValidationRecovery && (conflictFiles.length > 0 || (isConflictDetectionEnabled && isLoadingConflicts)) && (
        <section data-testid="conflict-files-section">
          <SectionTitle>
            Conflict Files ({conflictFiles.length})
            {isConflictDetectionEnabled && isLoadingConflicts && (
              <Loader2 className="inline-block w-3.5 h-3.5 ml-2 animate-spin text-white/40" />
            )}
          </SectionTitle>
          <DetailCard variant="warning">
            {isConflictDetectionEnabled && isLoadingConflicts && conflictFiles.length === 0 ? (
              <div className="flex items-center gap-2 py-2">
                <Loader2 className="w-4 h-4 animate-spin" style={{ color: "#ff9f0a" }} />
                <span className="text-[13px] text-white/50">Detecting conflicts...</span>
              </div>
            ) : (
              <ConflictFilesList files={conflictFiles} taskId={task.id} />
            )}
          </DetailCard>
        </section>
      )}

      {/* Branch Info */}
      <section data-testid="branch-info-section">
        <SectionTitle muted>Branch</SectionTitle>
        <BranchBadge branch={branchName} variant="muted" size="sm" />
      </section>
    </TwoColumnLayout>
  );
}
