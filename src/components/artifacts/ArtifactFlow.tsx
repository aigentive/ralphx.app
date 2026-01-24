/**
 * ArtifactFlow - Visualizes artifact flow triggers and steps
 *
 * Features:
 * - Flow name and active/inactive status
 * - Trigger event with optional filter
 * - Step list with icons and arrows
 * - Simple diagram layout
 */

import type { ArtifactFlow as ArtifactFlowType, ArtifactFlowStep } from "@/types/artifact";

// ============================================================================
// Types
// ============================================================================

interface ArtifactFlowProps {
  flow: ArtifactFlowType;
}

// ============================================================================
// Helpers
// ============================================================================

function StepIcon({ type }: { type: "copy" | "spawn_process" }) {
  if (type === "copy") {
    return <span data-testid="step-icon-copy" className="text-base">📋</span>;
  }
  return <span data-testid="step-icon-spawn" className="text-base">🚀</span>;
}

function renderStep(step: ArtifactFlowStep, idx: number) {
  if (step.type === "copy") {
    return (
      <li key={idx} data-testid="flow-step" className="flex items-center gap-2 p-2 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
        <StepIcon type="copy" />
        <div className="text-sm">
          <span className="font-medium" style={{ color: "var(--text-primary)" }}>Copy</span>
          <span style={{ color: "var(--text-muted)" }}> → </span>
          <span style={{ color: "var(--text-secondary)" }}>{step.toBucket}</span>
        </div>
      </li>
    );
  }
  return (
    <li key={idx} data-testid="flow-step" className="flex items-center gap-2 p-2 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
      <StepIcon type="spawn_process" />
      <div className="text-sm">
        <span className="font-medium" style={{ color: "var(--text-primary)" }}>Spawn</span>
        <span style={{ color: "var(--text-muted)" }}> → </span>
        <span style={{ color: "var(--text-secondary)" }}>{step.processType}</span>
        <span style={{ color: "var(--text-muted)" }}> ({step.agentProfile})</span>
      </div>
    </li>
  );
}

// ============================================================================
// Component
// ============================================================================

export function ArtifactFlow({ flow }: ArtifactFlowProps) {
  const hasFilter = flow.trigger.filter && (flow.trigger.filter.artifactTypes?.length || flow.trigger.filter.sourceBucket);

  return (
    <article data-testid="artifact-flow" data-active={flow.isActive ? "true" : "false"} role="article"
      className="p-3 rounded border" style={{ backgroundColor: "var(--bg-surface)", borderColor: "var(--border-subtle)" }}>
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <span data-testid="flow-name" className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>{flow.name}</span>
        <span data-testid="flow-status" className="text-xs px-1.5 py-0.5 rounded"
          style={{ color: flow.isActive ? "var(--status-success)" : "var(--text-muted)", backgroundColor: "var(--bg-base)" }}>
          {flow.isActive ? "Active" : "Inactive"}
        </span>
      </div>

      {/* Trigger */}
      <div data-testid="flow-trigger" className="mb-2 p-2 rounded text-sm" style={{ backgroundColor: "var(--bg-base)" }}>
        <div className="flex items-center gap-1">
          <span style={{ color: "var(--text-muted)" }}>When:</span>
          <span style={{ color: "var(--text-primary)" }}>{flow.trigger.event}</span>
        </div>
        {hasFilter && (
          <div data-testid="trigger-filter" className="mt-1 text-xs" style={{ color: "var(--text-muted)" }}>
            {flow.trigger.filter?.artifactTypes && <span>Types: {flow.trigger.filter.artifactTypes.join(", ")}</span>}
            {flow.trigger.filter?.artifactTypes && flow.trigger.filter?.sourceBucket && <span> | </span>}
            {flow.trigger.filter?.sourceBucket && <span>From: {flow.trigger.filter.sourceBucket}</span>}
          </div>
        )}
      </div>

      {/* Arrow from trigger to steps */}
      <div data-testid="trigger-arrow" className="flex justify-center my-1" style={{ color: "var(--text-muted)" }}>↓</div>

      {/* Steps */}
      <ul role="list" className="space-y-1">
        {flow.steps.map((step, idx) => (
          <div key={idx}>
            {renderStep(step, idx)}
            {idx < flow.steps.length - 1 && (
              <div data-testid="step-arrow" className="flex justify-center my-1" style={{ color: "var(--text-muted)" }}>↓</div>
            )}
          </div>
        ))}
      </ul>
    </article>
  );
}
