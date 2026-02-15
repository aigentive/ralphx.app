/**
 * MergePipelinePopover - Compact merge pipeline status
 *
 * Dense row-based layout inspired by macOS Finder list view.
 * Empty sections are hidden. Collapsible section headers.
 */

import { useState } from "react";
import { ChevronRight } from "lucide-react";
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

interface MergePipelinePopoverProps {
  /** Tasks currently being merged */
  active: MergePipelineTask[];
  /** Tasks waiting in the merge queue */
  waiting: MergePipelineTask[];
  /** Tasks needing attention (conflicts/incomplete) */
  needsAttention: MergePipelineTask[];
  /** Number of currently running agents (for deferred merge indicator) */
  runningCount?: number;
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
  runningCount,
  children,
  alignOffset = -24,
}: MergePipelinePopoverProps) {
  const [sections, setSections] = useState({
    active: true,
    waiting: true,
    attention: true,
  });

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
      await api.tasks.move(taskId, "pending_merge");
    } catch (error) {
      console.error("Failed to retry merge:", error);
    }
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
                  <WaitingMergeCard key={task.taskId} task={task} runningCount={runningCount} />
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
