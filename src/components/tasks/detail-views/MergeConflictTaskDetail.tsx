/**
 * MergeConflictTaskDetail - View for tasks in merge_conflict state
 *
 * Shows conflict files, read-only chat history from merger agent,
 * and action buttons for manual resolution.
 */

import { useState, useCallback, useMemo } from "react";
import {
  AlertTriangle,
  FileWarning,
  CheckCircle2,
  GitMerge,
  Loader2,
  Ban,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { extractErrorMessage } from "@/lib/errors";
import { Button } from "@/components/ui/button";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import type { Task } from "@/types/task";
import { BranchBadge } from "@/components/shared/BranchBadge";
import { useQueryClient } from "@tanstack/react-query";
import { taskKeys } from "@/hooks/useTasks";
import { useConfirmation } from "@/hooks/useConfirmation";
import { api } from "@/lib/tauri";
import { useConflictDetection } from "@/hooks/useConflictDetection";
import { useConflictDiff } from "@/hooks/useConflictDiff";
import { ConflictDiffViewer } from "@/components/diff/ConflictDiffViewer";

interface MergeConflictTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
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

  if (files.length === 0) {
    return (
      <p className="text-[13px] text-white/50 italic">
        No conflict files recorded
      </p>
    );
  }

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

/**
 * ResolutionInstructions - Guide for resolving conflicts manually
 */
function ResolutionInstructions({ branchName }: { branchName: string }) {
  return (
    <div className="space-y-3">
      <p className="text-[13px] text-white/60">
        The AI agent could not automatically resolve the merge conflicts.
        Please resolve them manually:
      </p>
      <ol className="list-decimal list-inside space-y-2 text-[13px] text-white/50">
        <li>Open the conflicting files in your editor</li>
        <li>Resolve the conflicts (remove conflict markers)</li>
        <li>Stage the resolved files: <code className="text-white/70 bg-white/5 px-1 rounded">git add .</code></li>
        <li>Commit the merge: <code className="text-white/70 bg-white/5 px-1 rounded">git commit</code></li>
        <li>Click "Conflicts Resolved" below to continue</li>
      </ol>
      <div className="pt-2">
        <BranchBadge branch={branchName} variant="muted" size="sm" />
      </div>
    </div>
  );
}

/**
 * ActionButtonsCard - Actions for conflict resolution
 */
function ActionButtonsCard({
  onResolve,
  onRetry,
  onCancel,
  isProcessing,
}: {
  onResolve: () => void;
  onRetry: () => void;
  onCancel: () => void;
  isProcessing: boolean;
}) {
  return (
    <div className="flex gap-2 justify-end flex-wrap">
      <Button
        data-testid="retry-merge-button"
        onClick={onRetry}
        disabled={isProcessing}
        variant="ghost"
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "hsl(220 10% 70%)",
          backgroundColor: "hsl(220 10% 16%)",
        }}
      >
        <GitMerge className="w-4 h-4" />
        Retry Merge
      </Button>
      <Button
        data-testid="resolve-conflict-button"
        onClick={onResolve}
        disabled={isProcessing}
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "white",
          backgroundColor: "#34c759",
        }}
      >
        {isProcessing ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <CheckCircle2 className="w-4 h-4" />
        )}
        Conflicts Resolved
      </Button>
      <Button
        data-testid="cancel-task-button"
        onClick={onCancel}
        disabled={isProcessing}
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "white",
          backgroundColor: "#ff4545",
        }}
      >
        {isProcessing ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <Ban className="w-4 h-4" />
        )}
        Cancel
      </Button>
    </div>
  );
}

export function MergeConflictTaskDetail({ task, isHistorical = false }: MergeConflictTaskDetailProps) {
  const queryClient = useQueryClient();
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { confirm } = useConfirmation();

  // Parse conflict files from task metadata (for historical view or fallback)
  const metadataConflicts: string[] = useMemo(() => {
    if (!task.metadata) return [];
    try {
      const metadata = typeof task.metadata === "string"
        ? JSON.parse(task.metadata)
        : task.metadata;
      return Array.isArray(metadata?.conflict_files) ? metadata.conflict_files : [];
    } catch {
      return [];
    }
  }, [task.metadata]);

  // Live conflict detection (only active for non-historical views)
  const {
    conflicts: liveConflicts,
    isLoading: isLoadingConflicts,
    isEnabled: isConflictDetectionEnabled,
  } = useConflictDetection({
    taskId: task.id,
    internalStatus: task.internalStatus,
    isHistorical,
    hasBranch: !!task.taskBranch,
  });

  // Hybrid data source: use live conflicts for active states, metadata for historical
  const conflictFiles: string[] = isHistorical
    ? metadataConflicts
    : (isConflictDetectionEnabled && liveConflicts.length > 0 ? liveConflicts : metadataConflicts);

  const branchName = task.taskBranch ?? "task branch";

  const handleResolveConflicts = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("resolve_merge_conflict", { taskId: task.id });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(extractErrorMessage(err, "Failed to mark conflicts as resolved"));
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

  const handleRetryMerge = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("retry_merge", { taskId: task.id });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(extractErrorMessage(err, "Failed to retry merge"));
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

  const handleCancel = useCallback(async () => {
    const confirmed = await confirm({
      title: "Cancel task?",
      description: "This will transition the task to Cancelled status. This action cannot be undone.",
      confirmText: "Cancel",
      variant: "destructive",
    });

    if (!confirmed) return;

    setIsProcessing(true);
    setError(null);
    try {
      await api.tasks.move(task.id, "cancelled");
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(extractErrorMessage(err, "Failed to cancel task"));
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient, confirm]);

  return (
    <TwoColumnLayout
      description={task.description}
      testId="merge-conflict-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={AlertTriangle}
        title="Merge Conflict"
        subtitle={isHistorical ? "Manual resolution was required" : "Manual resolution required"}
        variant="warning"
        badge={
          <StatusPill
            icon={AlertTriangle}
            label="Conflict"
            variant="warning"
            size="md"
          />
        }
      />

      {/* Conflict Files */}
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

      {/* Resolution Instructions (not in historical mode) */}
      {!isHistorical && (
        <section data-testid="resolution-instructions-section">
          <SectionTitle>How to Resolve</SectionTitle>
          <DetailCard>
            <ResolutionInstructions branchName={branchName} />
          </DetailCard>
        </section>
      )}

      {/* Error Display */}
      {error && (
        <div
          className="p-3 rounded-lg text-[13px]"
          style={{
            backgroundColor: "rgba(255, 69, 58, 0.12)",
            color: "#ff6961",
          }}
        >
          {error}
        </div>
      )}

      {/* Actions (hidden in historical mode) */}
      {!isHistorical && (
        <section data-testid="action-buttons">
          <ActionButtonsCard
            onResolve={handleResolveConflicts}
            onRetry={handleRetryMerge}
            onCancel={handleCancel}
            isProcessing={isProcessing}
          />
        </section>
      )}
    </TwoColumnLayout>
  );
}
