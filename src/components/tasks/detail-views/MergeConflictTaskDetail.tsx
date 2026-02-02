/**
 * MergeConflictTaskDetail - View for tasks in merge_conflict state
 *
 * Shows conflict files, read-only chat history from merger agent,
 * and action buttons for manual resolution.
 */

import { useState, useCallback } from "react";
import {
  AlertTriangle,
  FileWarning,
  CheckCircle2,
  GitMerge,
  Loader2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import type { Task } from "@/types/task";
import { useQueryClient } from "@tanstack/react-query";
import { taskKeys } from "@/hooks/useTasks";

interface MergeConflictTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * ConflictFilesList - Shows files with merge conflicts
 */
function ConflictFilesList({ files }: { files: string[] }) {
  if (files.length === 0) {
    return (
      <p className="text-[13px] text-white/50 italic">
        No conflict files recorded
      </p>
    );
  }

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
        <span className="text-[11px] text-white/40">Branch: </span>
        <span className="text-[11px] text-white/60 font-mono">{branchName}</span>
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
  isProcessing,
}: {
  onResolve: () => void;
  onRetry: () => void;
  isProcessing: boolean;
}) {
  return (
    <div className="flex gap-2 justify-end">
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
    </div>
  );
}

export function MergeConflictTaskDetail({ task, isHistorical = false }: MergeConflictTaskDetailProps) {
  const queryClient = useQueryClient();
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Parse conflict files from task metadata if available
  const conflictFiles: string[] = (() => {
    if (!task.metadata) return [];
    const metadata = typeof task.metadata === "string"
      ? JSON.parse(task.metadata)
      : task.metadata;
    return Array.isArray(metadata?.conflict_files) ? metadata.conflict_files : [];
  })();

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
      setError(err instanceof Error ? err.message : "Failed to mark conflicts as resolved");
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
      setError(err instanceof Error ? err.message : "Failed to retry merge");
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

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
        <SectionTitle>Conflict Files ({conflictFiles.length})</SectionTitle>
        <DetailCard variant="warning">
          <ConflictFilesList files={conflictFiles} />
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
            isProcessing={isProcessing}
          />
        </section>
      )}
    </TwoColumnLayout>
  );
}
