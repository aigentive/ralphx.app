/**
 * BasicTaskDetail - Basic task detail view for backlog, ready, blocked states
 *
 * This component renders core task information without state-specific behavior.
 * Used as the default view for tasks that don't need specialized UI.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Badge } from "@/components/ui/badge";
import { StepList } from "../StepList";
import { SectionTitle } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { Loader2 } from "lucide-react";
import type { Task, InternalStatus } from "@/types/task";

interface BasicTaskDetailProps {
  task: Task;
}

// Priority colors matching TaskDetailPanel design spec
const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-base)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

const DEFAULT_PRIORITY_COLOR = {
  bg: "var(--bg-hover)",
  text: "var(--text-secondary)",
};

// Status badge configuration matching TaskDetailPanel
const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; bg: string; text: string }
> = {
  backlog: {
    label: "Backlog",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  ready: {
    label: "Ready",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  blocked: {
    label: "Blocked",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  executing: {
    label: "Executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  approved: {
    label: "Approved",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  failed: {
    label: "Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  reviewing: {
    label: "AI Review in Progress",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  review_passed: {
    label: "AI Review Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
};

function PriorityBadge({ priority }: { priority: number }) {
  const colors = PRIORITY_COLORS[priority] ?? DEFAULT_PRIORITY_COLOR;
  return (
    <span
      data-testid="basic-task-priority"
      className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium"
      style={{ backgroundColor: colors.bg, color: colors.text }}
    >
      P{priority}
    </span>
  );
}

function StatusBadge({ status }: { status: InternalStatus }) {
  const config = STATUS_CONFIG[status];
  return (
    <Badge
      data-testid="basic-task-status"
      data-status={status}
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0"
      style={{ backgroundColor: config.bg, color: config.text }}
    >
      {config.label}
    </Badge>
  );
}

/**
 * BasicTaskDetail Component
 *
 * Renders basic task information suitable for backlog, ready, and blocked states.
 * Shows: status badge, title, priority, category, description, and steps (if any).
 * Does not include edit buttons - parent component handles those.
 */
export function BasicTaskDetail({ task }: BasicTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;

  return (
    <div
      data-testid="basic-task-detail"
      data-task-id={task.id}
      className="space-y-6"
    >
      {/* Header: Priority, Title, Category, Status */}
      <div className="space-y-2">
        <div className="flex items-start gap-2.5">
          <PriorityBadge priority={task.priority} />
          <div className="flex-1 min-w-0">
            <h2
              data-testid="basic-task-title"
              className="text-base font-semibold text-white/90"
              style={{
                letterSpacing: "-0.02em",
                lineHeight: "1.3",
              }}
            >
              {task.title}
            </h2>
            <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
              <span
                data-testid="basic-task-category"
                className="px-1.5 py-0.5 rounded text-[10px] font-medium"
                style={{
                  backgroundColor: "rgba(255,255,255,0.05)",
                  border: "1px solid rgba(255,255,255,0.08)",
                  color: "rgba(255,255,255,0.6)",
                }}
              >
                {task.category}
              </span>
              <StatusBadge status={task.internalStatus} />
            </div>
          </div>
        </div>
      </div>

      {/* Description Section */}
      {task.description ? (
        <div>
          <p
            data-testid="basic-task-description"
            className="text-[13px] text-white/60"
            style={{
              lineHeight: "1.6",
              wordBreak: "break-word",
            }}
          >
            {task.description}
          </p>
        </div>
      ) : (
        <p className="text-[13px] italic text-white/35">
          No description provided
        </p>
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="basic-task-steps-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-6 h-6 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}
      {!stepsLoading && hasSteps && (
        <div data-testid="basic-task-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </div>
      )}
    </div>
  );
}
