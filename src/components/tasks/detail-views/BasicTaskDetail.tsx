/**
 * BasicTaskDetail - Basic task detail view for backlog, ready, blocked states
 *
 * This component renders core task information without state-specific behavior.
 * Used as the default view for tasks that don't need specialized UI.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { StepList } from "../StepList";
import { SectionTitle } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { Loader2 } from "lucide-react";
import type { Task } from "@/types/task";

interface BasicTaskDetailProps {
  task: Task;
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
