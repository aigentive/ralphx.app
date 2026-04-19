/**
 * AcceptModal - Modal for accepting a plan and creating tasks in Kanban
 * Shows summary, dependency graph preview, and warnings before accepting.
 * Task status is automatically determined based on dependencies:
 * - Tasks with no blockers → Ready
 * - Tasks with blockers → Blocked
 */

import { useState, useCallback, useEffect } from "react";
import { ShieldAlert } from "lucide-react";
import { useVerificationGate } from "@/hooks/useVerificationGate";
import { getGitBranches } from "@/api/projects";
import type { TaskProposal } from "@/types/ideation";
import type { ApplyProposalsInput, DependencyGraphResponse } from "@/api/ideation.types";
import type { IdeationSessionResponse } from "@/api/ideation";

interface AcceptModalProps {
  isOpen: boolean;
  proposals: TaskProposal[];
  dependencyGraph: DependencyGraphResponse;
  sessionId: string;
  onAccept: (options: ApplyProposalsInput) => void;
  onCancel: () => void;
  isAccepting?: boolean;
  warnings?: string[];
  /** Session for verification gate — shows warning and blocks accept when unverified */
  session?: Pick<
    IdeationSessionResponse,
    "id" | "planArtifactId" | "sessionPurpose" | "verificationStatus" | "verificationInProgress"
  > | null;
  /** Working directory for git branch listing */
  workingDirectory?: string | undefined;
  /** Default base branch to pre-fill the selector */
  baseBranch?: string | undefined;
}

export function AcceptModal({
  isOpen,
  proposals,
  dependencyGraph,
  sessionId,
  onAccept,
  onCancel,
  isAccepting = false,
  warnings = [],
  session = null,
  workingDirectory,
  baseBranch = "main",
}: AcceptModalProps) {
  const verificationGate = useVerificationGate(session);
  const [baseBranchOverride, setBaseBranchOverride] = useState<string>(baseBranch);
  const [branches, setBranches] = useState<string[]>([]);
  const [branchLoadError, setBranchLoadError] = useState(false);
  const [branchesLoading, setBranchesLoading] = useState(false);

  // Handle Escape key to close modal
  useEffect(() => {
    if (!isOpen || isAccepting) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, isAccepting, onCancel]);

  // Load git branches for the base branch selector
  useEffect(() => {
    if (!workingDirectory) return;

    setBranchLoadError(false);
    setBranchesLoading(true);
    getGitBranches(workingDirectory)
      .then((result) => setBranches(result))
      .catch(() => {
        setBranchLoadError(true);
        setBranches([]);
      })
      .finally(() => setBranchesLoading(false));
  }, [workingDirectory]);

  const handleOverlayClick = useCallback(() => {
    if (!isAccepting) {
      onCancel();
    }
  }, [isAccepting, onCancel]);

  const handleContentClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  const normalizedBaseBranch = baseBranchOverride.trim();
  const baseBranchValidationError = normalizedBaseBranch.length === 0
    ? "Enter a base branch"
    : null;

  const handleAccept = useCallback(() => {
    const options: ApplyProposalsInput = {
      sessionId,
      proposalIds: proposals.map((p) => p.id),
      // Status is determined automatically by backend based on dependencies:
      // - No blockers → Ready
      // - Has blockers → Blocked
      targetColumn: "auto",
      ...(normalizedBaseBranch !== "" && {
        baseBranchOverride: normalizedBaseBranch,
      }),
    };
    onAccept(options);
  }, [sessionId, proposals, normalizedBaseBranch, onAccept]);

  if (!isOpen) return null;

  const proposalCount = proposals.length;
  const dependencyCount = dependencyGraph.edges.length;
  const hasCycles = dependencyGraph.hasCycles;
  const hasCriticalPath = dependencyGraph.criticalPath.length > 0;
  const verificationBlocked = !verificationGate.canAccept;
  const canAccept =
    proposalCount > 0 &&
    !isAccepting &&
    !verificationBlocked &&
    !baseBranchValidationError;

  return (
    <div
      data-testid="accept-modal"
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-labelledby="accept-modal-title"
      aria-modal="true"
    >
      <div
        data-testid="modal-overlay"
        className="absolute inset-0"
        style={{ backgroundColor: "var(--overlay-scrim-med)" }}
        onClick={handleOverlayClick}
      />
      <div
        data-testid="modal-content"
        className="relative w-full max-w-md max-h-[90vh] overflow-y-auto p-6 rounded-lg shadow-lg"
        style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
        onClick={handleContentClick}
      >
        <h2
          id="accept-modal-title"
          className="text-lg font-semibold mb-4"
          style={{ color: "var(--text-primary)" }}
        >
          Accept Plan
        </h2>

        {/* Plan Summary */}
        <div className="mb-4">
          <h3
            className="text-sm font-medium mb-2"
            style={{ color: "var(--text-primary)" }}
          >
            Tasks to Create
          </h3>
          <p
            className="text-sm mb-2"
            style={{ color: "var(--text-secondary)" }}
          >
            {proposalCount} task{proposalCount !== 1 ? "s" : ""} will be created
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

        {/* Auto-status info */}
        <div
          className="mb-4 p-3 rounded text-sm"
          style={{ backgroundColor: "var(--bg-base)", color: "var(--text-secondary)" }}
        >
          <p className="font-medium mb-1" style={{ color: "var(--text-primary)" }}>
            Task Status
          </p>
          <p>Tasks will be automatically assigned status based on dependencies:</p>
          <ul className="mt-1 ml-4 list-disc text-xs" style={{ color: "var(--text-muted)" }}>
            <li>Tasks with no blockers → <strong>Ready</strong></li>
            <li>Tasks with blockers → <strong>Blocked</strong></li>
          </ul>
        </div>

        {/* Feature Branch Info */}
        <div className="mb-6">
          <p
            className="text-xs mb-3"
            style={{ color: "var(--text-muted)" }}
          >
            A feature branch will be created from the base branch below. A merge-to-main task is added automatically.
          </p>

          <div className="flex items-center gap-2 mb-1">
            <label
              className="block text-xs font-medium"
              style={{ color: "var(--text-secondary)" }}
              htmlFor="base-branch-input"
            >
              Base branch
            </label>
            {branchesLoading && (
              <div
                data-testid="branch-loading-spinner"
                className="w-3 h-3 rounded-full border border-current border-t-transparent animate-spin"
                style={{ color: "var(--text-muted)" }}
                aria-label="Loading branches"
              />
            )}
          </div>
          <input
            id="base-branch-input"
            type="text"
            list="base-branch-datalist"
            value={baseBranchOverride}
            onChange={(e) => setBaseBranchOverride(e.target.value)}
            disabled={isAccepting}
            placeholder="e.g. main"
            data-testid="base-branch-input"
            aria-invalid={baseBranchValidationError ? "true" : "false"}
            className="w-full px-2 py-1.5 text-sm rounded border outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none"
            style={{
              backgroundColor: "var(--bg-base)",
              borderColor: baseBranchValidationError
                ? "var(--status-danger)"
                : "var(--border-subtle)",
              color: "var(--text-primary)",
              boxShadow: "none",
            }}
          />
          <datalist id="base-branch-datalist">
            {branches.map((branch) => (
              <option key={branch} value={branch} />
            ))}
          </datalist>
          {branchLoadError && (
            <p
              className="mt-1 text-xs"
              style={{ color: "var(--text-muted)" }}
              data-testid="branch-load-error"
            >
              Could not load branches — type branch name manually
            </p>
          )}
          {baseBranchValidationError && (
            <p
              className="mt-1 text-xs"
              style={{ color: "var(--status-danger)" }}
              data-testid="branch-validation-error"
            >
              {baseBranchValidationError}
            </p>
          )}
        </div>

        {/* Verification blocked warning */}
        {verificationBlocked && verificationGate.reason && (
          <div
            data-testid="verification-blocked-warning"
            className="mb-4 flex items-start gap-2 p-3 rounded text-sm"
            style={{
              backgroundColor: "var(--status-error-muted)",
              border: "1px solid var(--status-error-border)",
              color: "var(--status-error)",
            }}
          >
            <ShieldAlert className="w-4 h-4 shrink-0 mt-0.5" />
            <span>{verificationGate.reason}</span>
          </div>
        )}

        {/* Footer with buttons */}
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isAccepting}
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
            onClick={handleAccept}
            disabled={!canAccept}
            className="flex items-center gap-1.5 px-4 py-2 rounded text-sm font-medium transition-colors"
            style={{
              backgroundColor: canAccept ? "var(--accent-primary)" : "var(--bg-hover)",
              color: canAccept ? "var(--bg-base)" : "var(--text-secondary)",
              cursor: canAccept ? "pointer" : "not-allowed",
              opacity: isAccepting ? 0.7 : 1,
            }}
          >
            {isAccepting ? (
              "Accepting..."
            ) : (
              `Accept Plan (${proposalCount} ${proposalCount === 1 ? "task" : "tasks"})`
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
