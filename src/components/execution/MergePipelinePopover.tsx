/**
 * MergePipelinePopover - Compact merge pipeline status
 *
 * Dense row-based layout inspired by macOS Finder list view.
 * Empty sections are hidden. Collapsible section headers.
 */

import { useState } from "react";
import { ChevronRight, Loader2, RotateCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import type { MergePipelineTask } from "@/api/merge-pipeline";
import { ActiveMergeCard } from "./ActiveMergeCard";
import { WaitingMergeCard } from "./WaitingMergeCard";
import { AttentionMergeCard } from "./AttentionMergeCard";
import { api } from "@/lib/tauri";
import { useUiStore } from "@/stores/uiStore";
import { cn } from "@/lib/utils";
import { getStatusIconConfig } from "@/types/status-icons";
import { toast } from "sonner";

interface MergePipelinePopoverProps {
  /** Tasks currently being merged */
  active: MergePipelineTask[];
  /** Tasks waiting in the merge queue */
  waiting: MergePipelineTask[];
  /** Tasks needing attention (conflicts/incomplete) */
  needsAttention: MergePipelineTask[];
  /** Trigger element (e.g., merge count button) */
  children: React.ReactNode;
  /** Optional horizontal alignment offset for popover content */
  alignOffset?: number;
}

interface SectionHeaderProps {
  title: string;
  count: number;
  isOpen: boolean;
  onToggle: () => void;
  highlight?: boolean;
}

function SectionHeader({ title, count, isOpen, onToggle, highlight = false }: SectionHeaderProps) {
  const attentionStyle = getStatusIconConfig("merge_incomplete");

  return (
    <button
      onClick={onToggle}
      className="flex items-center gap-1.5 w-full px-2 py-1 rounded hover:bg-white/[0.03] transition-colors"
    >
      <ChevronRight
        className={cn(
          "w-3 h-3 transition-transform duration-150",
          isOpen && "rotate-90"
        )}
        style={{ color: "hsl(220 10% 40%)" }}
      />
      <span
        className="text-[10px] font-semibold uppercase tracking-wider"
        style={{ color: highlight ? attentionStyle.color : "hsl(220 10% 50%)" }}
      >
        {title}
      </span>
      <span
        className="text-[10px] tabular-nums ml-auto"
        style={{ color: "hsl(220 10% 40%)" }}
      >
        {count}
      </span>
    </button>
  );
}

export function MergePipelinePopover({
  active,
  waiting,
  needsAttention,
  children,
  alignOffset = -24,
}: MergePipelinePopoverProps) {
  const [sections, setSections] = useState({
    active: true,
    waiting: true,
    attention: true,
  });
  const [isRetryingAllAttention, setIsRetryingAllAttention] = useState(false);

  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);

  const handleStopMerge = async (taskId: string) => {
    try {
      await api.tasks.move(taskId, "stopped");
    } catch (error) {
      console.error("Failed to stop merge:", error);
    }
  };

  const handleViewDetails = (taskId: string) => {
    setSelectedTaskId(taskId);
  };

  const handleRetryMerge = async (taskId: string) => {
    try {
      await invoke("retry_merge", { taskId });
      await invoke("drain_merge_recovery_now");
    } catch (error) {
      console.error("Failed to retry merge:", error);
    }
  };

  const handleRetryAllAttention = async () => {
    if (isRetryingAllAttention || needsAttention.length === 0) {
      return;
    }

    setIsRetryingAllAttention(true);
    let successCount = 0;
    let failedCount = 0;

    for (const task of needsAttention) {
      try {
        await invoke("retry_merge", { taskId: task.taskId });
        successCount += 1;
      } catch (error) {
        failedCount += 1;
        console.error("Failed to retry merge task:", task.taskId, error);
      }
    }

    try {
      await invoke("drain_merge_recovery_now");
    } catch (error) {
      console.error("Failed to trigger manual merge reconciliation drain:", error);
    }

    if (successCount > 0) {
      toast.success(
        `Queued ${successCount} merge retry${successCount === 1 ? "" : "ies"}.`
      );
    }
    if (failedCount > 0) {
      toast.error(
        `${failedCount} merge retr${failedCount === 1 ? "y" : "ies"} failed to queue.`
      );
    }

    setIsRetryingAllAttention(false);
  };

  const toggleSection = (key: "active" | "waiting" | "attention") => {
    setSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const total = active.length + waiting.length + needsAttention.length;

  return (
    <Popover>
      <PopoverTrigger asChild>
        {children}
      </PopoverTrigger>
      <PopoverContent
        side="top"
        align="start"
        alignOffset={alignOffset}
        sideOffset={24}
        className="w-[420px] p-3"
        style={{
          backgroundColor: "hsl(220 10% 11%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          borderRadius: "10px",
          boxShadow:
            "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-1.5 px-2">
          <h3
            className="text-xs font-semibold"
            style={{ color: "hsl(220 10% 80%)" }}
          >
            Merge Pipeline
          </h3>
          <span
            className="text-[11px] tabular-nums"
            style={{ color: "hsl(220 10% 42%)" }}
          >
            {total} total
          </span>
        </div>

        {/* Scrollable content */}
        <div
          className="max-h-[320px] overflow-y-auto -mx-1 px-1"
          style={{
            scrollbarWidth: "thin",
            scrollbarColor: "hsla(220 10% 100% / 0.1) transparent",
          }}
        >
          {/* Active */}
          {active.length > 0 && (
            <div className="mb-0.5">
              <SectionHeader
                title="Active"
                count={active.length}
                isOpen={sections.active}
                onToggle={() => toggleSection("active")}
              />
              {sections.active &&
                active.map((task) => (
                  <ActiveMergeCard
                    key={task.taskId}
                    task={task}
                    onStop={handleStopMerge}
                  />
                ))}
            </div>
          )}

          {/* Waiting */}
          {waiting.length > 0 && (
            <div className="mb-0.5">
              <SectionHeader
                title="Waiting"
                count={waiting.length}
                isOpen={sections.waiting}
                onToggle={() => toggleSection("waiting")}
              />
              {sections.waiting &&
                waiting.map((task) => (
                  <WaitingMergeCard key={task.taskId} task={task} />
                ))}
            </div>
          )}

          {/* Needs Attention */}
          {needsAttention.length > 0 && (
            <div className="mb-0.5">
              <SectionHeader
                title="Needs Attention"
                count={needsAttention.length}
                isOpen={sections.attention}
                onToggle={() => toggleSection("attention")}
                highlight
              />
              {sections.attention && needsAttention.length > 1 && (
                <div className="px-2 pb-1">
                  <button
                    onClick={handleRetryAllAttention}
                    disabled={isRetryingAllAttention}
                    className={cn(
                      "h-7 px-2.5 rounded-md text-[11px] font-medium inline-flex items-center gap-1.5",
                      "transition-colors",
                      isRetryingAllAttention
                        ? "opacity-70 cursor-not-allowed"
                        : "hover:bg-white/[0.08]"
                    )}
                    style={{
                      color: getStatusIconConfig("pending_merge").color,
                      backgroundColor: "hsl(220 10% 15%)",
                    }}
                    title="Retry all tasks in Needs Attention using merge retry flow"
                  >
                    {isRetryingAllAttention ? (
                      <Loader2 className="w-3 h-3 animate-spin" />
                    ) : (
                      <RotateCw className="w-3 h-3" />
                    )}
                    Retry All
                  </button>
                </div>
              )}
              {sections.attention &&
                needsAttention.map((task) => (
                  <AttentionMergeCard
                    key={task.taskId}
                    task={task}
                    onViewDetails={handleViewDetails}
                    onRetry={handleRetryMerge}
                  />
                ))}
            </div>
          )}

          {/* Empty state */}
          {total === 0 && (
            <div
              className="py-6 text-center text-xs"
              style={{ color: "hsl(220 10% 42%)" }}
            >
              No merge tasks
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="mt-2 pt-2 px-2 text-[11px]"
          style={{
            borderTop: "1px solid hsla(220 20% 100% / 0.06)",
            color: "hsl(220 10% 42%)",
          }}
        >
          Two-phase merge: programmatic first, then AI agent. One per branch.
        </div>
      </PopoverContent>
    </Popover>
  );
}
