/**
 * TaskBoardWithHeader - TaskBoard with workflow selector header
 *
 * Design spec: specs/design/refined-studio-patterns.md
 * - Refined Studio aesthetic with glass effect header
 * - Compact sizing for application UI
 */

import { useState, useCallback } from "react";
import { useWorkflows } from "@/hooks/useWorkflows";
import { WorkflowSelector } from "@/components/workflows/WorkflowSelector";
import { TaskBoard } from "./TaskBoard";
import type { WorkflowSchema } from "@/types/workflow";

// ============================================================================
// Types
// ============================================================================

interface TaskBoardWithHeaderProps {
  projectId: string;
}

// ============================================================================
// Component
// ============================================================================

export function TaskBoardWithHeader({ projectId }: TaskBoardWithHeaderProps) {
  const { data: workflowsResponse, isLoading: isLoadingWorkflows } = useWorkflows();

  // API now returns WorkflowSchema directly (transforms applied in API layer)
  const workflows: WorkflowSchema[] = workflowsResponse ?? [];

  // Find default workflow
  const defaultWorkflow = workflows.find((w) => w.isDefault);

  // Track selected workflow ID (default to first default workflow)
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);

  // Resolved current workflow ID
  const currentWorkflowId = selectedWorkflowId ?? defaultWorkflow?.id ?? null;

  const handleSelectWorkflow = useCallback((workflowId: string) => {
    setSelectedWorkflowId(workflowId);
  }, []);

  return (
    <div data-testid="task-board-with-header" className="flex flex-col h-full">
      {/* Header - Glass effect */}
      <div
        className="flex items-center justify-between px-3 py-1.5 border-b backdrop-blur-sm"
        style={{
          borderColor: "rgba(255,255,255,0.06)",
          background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%)",
        }}
      >
        <WorkflowSelector
          workflows={workflows}
          currentWorkflowId={currentWorkflowId}
          onSelectWorkflow={handleSelectWorkflow}
          isLoading={isLoadingWorkflows}
        />
      </div>

      {/* Task Board */}
      <TaskBoard projectId={projectId} />
    </div>
  );
}
