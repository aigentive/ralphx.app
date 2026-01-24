/**
 * TaskBoardWithHeader - TaskBoard with workflow selector header
 *
 * Features:
 * - Header with WorkflowSelector dropdown
 * - Workflow switching re-renders columns
 * - Task data preserved during workflow switch
 */

import { useState, useCallback, useMemo } from "react";
import { useWorkflows } from "@/hooks/useWorkflows";
import { WorkflowSelector } from "@/components/workflows/WorkflowSelector";
import { TaskBoard } from "./TaskBoard";
import type { WorkflowSchema } from "@/types/workflow";
import type { WorkflowResponse } from "@/lib/api/workflows";

// ============================================================================
// Types
// ============================================================================

interface TaskBoardWithHeaderProps {
  projectId: string;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Convert WorkflowResponse (snake_case from API) to WorkflowSchema (camelCase)
 */
function toWorkflowSchema(response: WorkflowResponse): WorkflowSchema {
  return {
    id: response.id,
    name: response.name,
    description: response.description ?? undefined,
    columns: response.columns.map((col) => ({
      id: col.id,
      name: col.name,
      mapsTo: col.maps_to,
      color: col.color ?? undefined,
      icon: col.icon ?? undefined,
      behavior: {
        skipReview: col.skip_review ?? undefined,
        autoAdvance: col.auto_advance ?? undefined,
        agentProfile: col.agent_profile ?? undefined,
      },
    })),
    isDefault: response.is_default,
  };
}

// ============================================================================
// Component
// ============================================================================

export function TaskBoardWithHeader({ projectId }: TaskBoardWithHeaderProps) {
  const { data: workflowsResponse, isLoading: isLoadingWorkflows } = useWorkflows();

  // Convert API responses to WorkflowSchema format
  const workflows = useMemo(
    () => (workflowsResponse ?? []).map(toWorkflowSchema),
    [workflowsResponse]
  );

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
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b" style={{ borderColor: "var(--border-subtle)", backgroundColor: "var(--bg-surface)" }}>
        <WorkflowSelector
          workflows={workflows}
          currentWorkflowId={currentWorkflowId}
          onSelectWorkflow={handleSelectWorkflow}
          isLoading={isLoadingWorkflows}
        />
      </div>

      {/* Task Board */}
      {currentWorkflowId && (
        <TaskBoard projectId={projectId} workflowId={currentWorkflowId} />
      )}
    </div>
  );
}
