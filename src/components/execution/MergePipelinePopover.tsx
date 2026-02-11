/**
 * MergePipelinePopover - Shows merge pipeline status with three sections
 *
 * Displays active merges, waiting merges, and merges needing attention.
 * Each section is collapsible and shows relevant task cards.
 */

import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
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

interface MergePipelinePopoverProps {
  /** Tasks currently being merged */
  active: MergePipelineTask[];
  /** Tasks waiting in the merge queue */
  waiting: MergePipelineTask[];
  /** Tasks needing attention (conflicts/incomplete) */
  needsAttention: MergePipelineTask[];
  /** Trigger element (e.g., merge count button) */
  children: React.ReactNode;
}

interface CollapsibleSectionProps {
  title: string;
  count: number;
  isOpen: boolean;
  onToggle: () => void;
  highlight?: boolean;
  children: React.ReactNode;
}

/**
 * Collapsible section header with expand/collapse
 */
function CollapsibleSection({
  title,
  count,
  isOpen,
  onToggle,
  highlight = false,
  children,
}: CollapsibleSectionProps) {
  return (
    <div className="mb-3 last:mb-0">
      {/* Header */}
      <button
        onClick={onToggle}
        className="flex items-center justify-between w-full px-3 py-2 rounded-lg transition-colors"
        style={{
          backgroundColor: highlight ? "hsla(45 90% 55% / 0.1)" : "transparent",
          border: highlight ? "1px solid hsla(45 90% 55% / 0.2)" : "1px solid transparent",
        }}
      >
        <div className="flex items-center gap-2">
          {isOpen ? (
            <ChevronDown className="w-4 h-4" style={{ color: "hsl(220 10% 65%)" }} />
          ) : (
            <ChevronRight className="w-4 h-4" style={{ color: "hsl(220 10% 65%)" }} />
          )}
          <span className="text-xs font-semibold uppercase tracking-wider" style={{ color: "hsl(220 10% 90%)" }}>
            {title} ({count})
          </span>
        </div>
      </button>

      {/* Content */}
      {isOpen && count > 0 && (
        <div className="mt-2 space-y-2">
          {children}
        </div>
      )}
    </div>
  );
}

export function MergePipelinePopover({
  active,
  waiting,
  needsAttention,
  children,
}: MergePipelinePopoverProps) {
  const [activeOpen, setActiveOpen] = useState(true);
  const [waitingOpen, setWaitingOpen] = useState(true);
  const [attentionOpen, setAttentionOpen] = useState(true);

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
      // Move back to pending_merge to re-trigger the merge pipeline
      await api.tasks.move(taskId, "pending_merge");
    } catch (error) {
      console.error("Failed to retry merge:", error);
    }
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        {children}
      </PopoverTrigger>
      <PopoverContent
        side="top"
        align="start"
        className="w-[480px] p-4"
        style={{
          backgroundColor: "hsl(220 10% 12%)",
          border: "1px solid hsla(220 20% 100% / 0.1)",
          borderRadius: "12px",
          boxShadow: "0 8px 24px hsla(220 20% 0% / 0.5)",
        }}
      >
        {/* Header */}
        <div className="mb-4">
          <h3 className="text-sm font-semibold" style={{ color: "hsl(220 10% 90%)" }}>
            Merge Pipeline
          </h3>
        </div>

        {/* Sections */}
        <div className="space-y-1">
          {/* Active */}
          <CollapsibleSection
            title="Active"
            count={active.length}
            isOpen={activeOpen}
            onToggle={() => setActiveOpen(!activeOpen)}
          >
            {active.map((task) => (
              <ActiveMergeCard key={task.taskId} task={task} onStop={handleStopMerge} />
            ))}
          </CollapsibleSection>

          {/* Waiting */}
          <CollapsibleSection
            title="Waiting"
            count={waiting.length}
            isOpen={waitingOpen}
            onToggle={() => setWaitingOpen(!waitingOpen)}
          >
            {waiting.map((task) => (
              <WaitingMergeCard key={task.taskId} task={task} />
            ))}
          </CollapsibleSection>

          {/* Needs Attention */}
          <CollapsibleSection
            title="Needs Attention"
            count={needsAttention.length}
            isOpen={attentionOpen}
            onToggle={() => setAttentionOpen(!attentionOpen)}
            highlight={needsAttention.length > 0}
          >
            {needsAttention.map((task) => (
              <AttentionMergeCard
                key={task.taskId}
                task={task}
                onViewDetails={handleViewDetails}
                onRetry={handleRetryMerge}
              />
            ))}
          </CollapsibleSection>
        </div>

        {/* Info Footer */}
        <div
          className="mt-4 pt-4 text-xs leading-relaxed"
          style={{
            borderTop: "1px solid hsla(220 20% 100% / 0.1)",
            color: "hsl(220 10% 65%)",
          }}
        >
          ⓘ Two-phase merge: fast programmatic merge first, then AI agent for conflicts.
          Merges run one at a time per target branch to avoid concurrent git conflicts.
          Deferred merges auto-retry when the active merge completes.
        </div>
      </PopoverContent>
    </Popover>
  );
}
