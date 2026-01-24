/**
 * ApplyModal - Modal for applying selected proposals to Kanban board
 * Shows summary, dependency graph preview, target column selector,
 * and warnings before applying.
 */

import { useState, useCallback, useEffect } from "react";
import type {
  TaskProposal,
  DependencyGraph,
  ApplyProposalsInput,
} from "@/types/ideation";

const TARGET_COLUMNS = [
  { value: "draft", label: "Draft" },
  { value: "backlog", label: "Backlog" },
  { value: "todo", label: "Todo" },
];

interface ApplyModalProps {
  isOpen: boolean;
  proposals: TaskProposal[];
  dependencyGraph: DependencyGraph;
  sessionId: string;
  onApply: (options: ApplyProposalsInput) => void;
  onCancel: () => void;
  isApplying?: boolean;
  warnings?: string[];
}

export function ApplyModal({
  isOpen,
  proposals,
  dependencyGraph,
  sessionId,
  onApply,
  onCancel,
  isApplying = false,
  warnings = [],
}: ApplyModalProps) {
  const [targetColumn, setTargetColumn] = useState("backlog");
  const [preserveDependencies, setPreserveDependencies] = useState(true);

  // Handle Escape key to close modal
  useEffect(() => {
    if (!isOpen || isApplying) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, isApplying, onCancel]);

  const handleOverlayClick = useCallback(() => {
    if (!isApplying) {
      onCancel();
    }
  }, [isApplying, onCancel]);

  const handleContentClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  const handleApply = useCallback(() => {
    const options: ApplyProposalsInput = {
      sessionId,
      proposalIds: proposals.map((p) => p.id),
      targetColumn,
      preserveDependencies,
    };
    onApply(options);
  }, [sessionId, proposals, targetColumn, preserveDependencies, onApply]);

  if (!isOpen) return null;

  const proposalCount = proposals.length;
  const dependencyCount = dependencyGraph.edges.length;
  const hasCycles = dependencyGraph.hasCycles;
  const hasCriticalPath = dependencyGraph.criticalPath.length > 0;
  const canApply = proposalCount > 0 && !isApplying;

  const inputClasses =
    "w-full rounded-md px-3 py-2 text-sm border focus:outline-none focus:ring-2 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed";

  return (
    <div
      data-testid="apply-modal"
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-labelledby="apply-modal-title"
      aria-modal="true"
    >
      <div
        data-testid="modal-overlay"
        className="absolute inset-0"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={handleOverlayClick}
      />
      <div
        data-testid="modal-content"
        className="relative w-full max-w-md max-h-[90vh] overflow-y-auto p-6 rounded-lg shadow-lg"
        style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
        onClick={handleContentClick}
      >
        <h2
          id="apply-modal-title"
          className="text-lg font-semibold mb-4"
          style={{ color: "var(--text-primary)" }}
        >
          Apply Proposals
        </h2>

        {/* Selected Proposals Summary */}
        <div className="mb-4">
          <h3
            className="text-sm font-medium mb-2"
            style={{ color: "var(--text-primary)" }}
          >
            Selected Proposals
          </h3>
          <p
            className="text-sm mb-2"
            style={{ color: "var(--text-secondary)" }}
          >
            {proposalCount} proposal{proposalCount !== 1 ? "s" : ""} selected
          </p>
          <div
            className="max-h-32 overflow-y-auto rounded border p-2 space-y-1"
            style={{ borderColor: "var(--border-subtle)", backgroundColor: "var(--bg-base)" }}
          >
            {proposals.map((proposal) => (
              <div
                key={proposal.id}
                className="flex items-center justify-between text-sm"
              >
                <span style={{ color: "var(--text-primary)" }}>{proposal.title}</span>
                <span
                  className="text-xs px-1.5 py-0.5 rounded"
                  style={{
                    backgroundColor: "var(--bg-hover)",
                    color: "var(--text-muted)",
                  }}
                >
                  {proposal.category}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Dependency Graph Preview */}
        <div className="mb-4">
          <h3
            className="text-sm font-medium mb-2"
            style={{ color: "var(--text-primary)" }}
          >
            Dependencies
          </h3>
          {dependencyCount === 0 ? (
            <p
              className="text-sm italic"
              style={{ color: "var(--text-muted)" }}
            >
              No dependencies
            </p>
          ) : (
            <div data-testid="dependency-preview">
              <p
                className="text-sm mb-2"
                style={{ color: "var(--text-secondary)" }}
              >
                {dependencyCount} dependencies
              </p>
              {hasCriticalPath && (
                <p
                  className="text-xs mb-1"
                  style={{ color: "var(--text-muted)" }}
                >
                  Critical path: {dependencyGraph.criticalPath.length} tasks
                </p>
              )}
              <div
                className="rounded border p-2 space-y-1 max-h-24 overflow-y-auto"
                style={{ borderColor: "var(--border-subtle)", backgroundColor: "var(--bg-base)" }}
              >
                {dependencyGraph.edges.map((edge, idx) => {
                  const fromNode = dependencyGraph.nodes.find((n) => n.proposalId === edge.from);
                  const toNode = dependencyGraph.nodes.find((n) => n.proposalId === edge.to);
                  return (
                    <div
                      key={`${edge.from}-${edge.to}-${idx}`}
                      className="text-xs flex items-center gap-1"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      <span className="truncate max-w-[120px]">{fromNode?.title ?? edge.from}</span>
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                        <path d="M2 6h8M7 3l3 3-3 3" stroke="currentColor" strokeWidth="1.5" fill="none" />
                      </svg>
                      <span className="truncate max-w-[120px]">{toNode?.title ?? edge.to}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Warnings */}
        {(hasCycles || warnings.length > 0) && (
          <div className="mb-4 space-y-2">
            {hasCycles && (
              <div
                data-testid="warning-cycles"
                role="alert"
                className="text-sm flex items-center gap-2"
                style={{ color: "var(--status-warning)" }}
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 1L1 14h14L8 1zm0 4v4m0 2v1" stroke="currentColor" strokeWidth="1.5" fill="none" />
                </svg>
                Circular dependency detected
              </div>
            )}
            {warnings.map((warning, idx) => (
              <div
                key={idx}
                data-testid="warning-missing"
                role="alert"
                className="text-sm flex items-center gap-2"
                style={{ color: "var(--status-warning)" }}
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 1L1 14h14L8 1zm0 4v4m0 2v1" stroke="currentColor" strokeWidth="1.5" fill="none" />
                </svg>
                {warning}
              </div>
            ))}
          </div>
        )}

        {/* Target Column Selector */}
        <div className="mb-4">
          <label
            htmlFor="target-column"
            className="block text-sm font-medium mb-1"
            style={{ color: "var(--text-primary)" }}
          >
            Target Column
          </label>
          <select
            id="target-column"
            value={targetColumn}
            onChange={(e) => setTargetColumn(e.target.value)}
            disabled={isApplying}
            className={inputClasses}
            style={{
              backgroundColor: "var(--bg-base)",
              borderColor: "var(--border-subtle)",
              color: "var(--text-primary)",
            }}
          >
            {TARGET_COLUMNS.map((col) => (
              <option key={col.value} value={col.value}>
                {col.label}
              </option>
            ))}
          </select>
        </div>

        {/* Preserve Dependencies Checkbox */}
        <div className="mb-6">
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={preserveDependencies}
              onChange={(e) => setPreserveDependencies(e.target.checked)}
              disabled={isApplying}
              className="mt-1"
              aria-label="Preserve dependencies between tasks"
            />
            <div>
              <span
                className="text-sm font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                Preserve dependencies
              </span>
              <p
                className="text-xs"
                style={{ color: "var(--text-muted)" }}
              >
                Create task dependencies from proposal relationships
              </p>
            </div>
          </label>
        </div>

        {/* Footer with buttons */}
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isApplying}
            className="px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            style={{
              backgroundColor: "var(--bg-hover)",
              color: "var(--text-primary)",
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleApply}
            disabled={!canApply}
            className="px-4 py-2 rounded text-sm font-medium transition-colors"
            style={{
              backgroundColor: canApply ? "var(--accent-primary)" : "var(--bg-hover)",
              color: canApply ? "var(--bg-base)" : "var(--text-secondary)",
              cursor: canApply ? "pointer" : "not-allowed",
              opacity: isApplying ? 0.7 : 1,
            }}
          >
            {isApplying
              ? "Applying..."
              : `Apply ${proposalCount} ${proposalCount === 1 ? "Proposal" : "Proposals"}`}
          </button>
        </div>
      </div>
    </div>
  );
}
