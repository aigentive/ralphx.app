/**
 * WorkflowSelector - Dropdown for selecting custom workflows
 *
 * Features:
 * - Dropdown listing available workflows
 * - Shows current workflow with default badge if applicable
 * - Column count per workflow
 * - Keyboard navigation (Escape to close)
 * - Click outside to close
 */

import { useState, useCallback, useEffect, useRef } from "react";
import type { WorkflowSchema } from "@/types/workflow";

// ============================================================================
// Types
// ============================================================================

interface WorkflowSelectorProps {
  workflows: WorkflowSchema[];
  currentWorkflowId: string | null;
  onSelectWorkflow: (workflowId: string) => void;
  isLoading?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function WorkflowSelector({
  workflows,
  currentWorkflowId,
  onSelectWorkflow,
  isLoading = false,
}: WorkflowSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const currentWorkflow = workflows.find((w) => w.id === currentWorkflowId);

  // Close dropdown on click outside
  useEffect(() => {
    if (!isOpen) return;

    const handleMouseDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [isOpen]);

  // Close dropdown on Escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  const handleToggle = useCallback(() => {
    if (!isLoading) {
      setIsOpen((prev) => !prev);
    }
  }, [isLoading]);

  const handleSelect = useCallback(
    (workflowId: string) => {
      onSelectWorkflow(workflowId);
      setIsOpen(false);
    },
    [onSelectWorkflow]
  );

  return (
    <div
      ref={containerRef}
      data-testid="workflow-selector"
      className="relative inline-flex items-center gap-2"
      style={{ backgroundColor: "var(--bg-surface)" }}
    >
      {/* Dropdown Trigger */}
      <button
        data-testid="dropdown-trigger"
        onClick={handleToggle}
        disabled={isLoading}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        className="flex items-center gap-2 px-3 py-1.5 rounded text-sm transition-colors hover:bg-[--bg-hover] disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <span
          data-testid="current-workflow-name"
          className="font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          {currentWorkflow?.name ?? "Select Workflow"}
        </span>
        {currentWorkflow?.isDefault && (
          <span
            data-testid="default-badge"
            className="px-1.5 py-0.5 text-xs rounded"
            style={{ backgroundColor: "var(--accent-primary)", color: "var(--bg-base)" }}
          >
            Default
          </span>
        )}
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="currentColor"
          style={{ color: "var(--text-secondary)" }}
        >
          <path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>
      </button>

      {/* Loading Indicator */}
      {isLoading && (
        <span data-testid="loading-indicator" className="text-xs" style={{ color: "var(--text-muted)" }}>
          Loading...
        </span>
      )}

      {/* Dropdown */}
      {isOpen && (
        <div
          data-testid="workflow-dropdown"
          role="listbox"
          className="absolute top-full left-0 mt-1 w-64 max-h-80 overflow-y-auto rounded shadow-lg border z-50"
          style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
        >
          {workflows.length === 0 ? (
            <div className="px-3 py-4 text-sm text-center" style={{ color: "var(--text-muted)" }}>
              No workflows available
            </div>
          ) : (
            workflows.map((workflow) => {
              const isSelected = workflow.id === currentWorkflowId;
              return (
                <div
                  key={workflow.id}
                  data-testid="workflow-item"
                  data-selected={isSelected ? "true" : "false"}
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => handleSelect(workflow.id)}
                  className="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-[--bg-hover] transition-colors"
                  style={{ backgroundColor: isSelected ? "var(--bg-hover)" : undefined }}
                >
                  <div className="flex items-center gap-2 flex-1 min-w-0">
                    <span className="text-sm truncate" style={{ color: "var(--text-primary)" }}>
                      {workflow.name}
                    </span>
                    {workflow.isDefault && (
                      <span
                        data-testid="workflow-default-indicator"
                        className="px-1 py-0.5 text-xs rounded"
                        style={{ backgroundColor: "var(--accent-primary)", color: "var(--bg-base)" }}
                      >
                        Default
                      </span>
                    )}
                  </div>
                  <span
                    data-testid="column-count"
                    className="text-xs flex-shrink-0"
                    style={{ color: "var(--text-muted)" }}
                  >
                    {workflow.columns.length} columns
                  </span>
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}
