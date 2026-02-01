/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { StepList } from "../StepList";
import { SectionTitle, DescriptionBlock } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { Loader2 } from "lucide-react";
import type { Task } from "@/types/task";

interface BasicTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

export function BasicTaskDetail({ task, isHistorical = false }: BasicTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;

  return (
    <div
      data-testid="basic-task-detail"
      data-task-id={task.id}
      className="space-y-6"
    >
      {/* Description Section */}
      <section>
        <SectionTitle>Description</SectionTitle>
        <DescriptionBlock
          description={task.description}
          testId="basic-task-description"
        />
      </section>

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="basic-task-steps-loading"
          className="flex items-center justify-center py-8"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      )}

      {!stepsLoading && hasSteps && (
        <section data-testid="basic-task-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} hideCompletionNotes={isHistorical} />
        </section>
      )}
    </div>
  );
}
