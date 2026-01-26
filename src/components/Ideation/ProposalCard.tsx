/**
 * ProposalCard - Task proposal card for the ideation view
 *
 * Features:
 * - Checkbox for selection
 * - Title and description preview
 * - Priority badge (Critical=red, High=orange, Medium=yellow, Low=gray)
 * - Category badge
 * - Dependency info (depends on X, blocks Y)
 * - Edit and Remove action buttons
 * - Selected state (orange border)
 * - Modified indicator
 */

import type { TaskProposal, Priority } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

export interface ProposalCardProps {
  /** The proposal to display */
  proposal: TaskProposal;
  /** Callback when checkbox is toggled */
  onSelect: (proposalId: string) => void;
  /** Callback when edit button is clicked */
  onEdit: (proposalId: string) => void;
  /** Callback when remove button is clicked */
  onRemove: (proposalId: string) => void;
  /** Number of proposals this depends on */
  dependsOnCount?: number;
  /** Number of proposals this blocks */
  blocksCount?: number;
  /** Show complexity indicator */
  showComplexity?: boolean;
  /** Current version of the linked plan artifact (if any) */
  currentPlanVersion?: number;
  /** Callback when "View plan as of creation" link is clicked */
  onViewHistoricalPlan?: (artifactId: string, version: number) => void;
}

// ============================================================================
// Priority Configuration
// ============================================================================

const PRIORITY_COLORS: Record<Priority, string> = {
  critical: "#ef4444",
  high: "#ff6b35",
  medium: "#ffa94d",
  low: "#6b7280",
};

const PRIORITY_LABELS: Record<Priority, string> = {
  critical: "Critical",
  high: "High",
  medium: "Medium",
  low: "Low",
};

// ============================================================================
// Icons
// ============================================================================

function EditIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M10.5 1.5L12.5 3.5L4.5 11.5L1.5 12.5L2.5 9.5L10.5 1.5Z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function RemoveIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M11 3L3 11M3 3L11 11"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

function DependencyIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path
        d="M6 2V10M6 2L3 5M6 2L9 5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function BlocksIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path
        d="M6 10V2M6 10L3 7M6 10L9 7"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ============================================================================
// Component
// ============================================================================

export function ProposalCard({
  proposal,
  onSelect,
  onEdit,
  onRemove,
  dependsOnCount,
  blocksCount,
  showComplexity = false,
  currentPlanVersion,
  onViewHistoricalPlan,
}: ProposalCardProps) {
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const isSelected = proposal.selected;
  const showDependencyInfo =
    (dependsOnCount !== undefined && dependsOnCount > 0) ||
    (blocksCount !== undefined && blocksCount > 0);

  // Check if we should show the historical plan link
  const showHistoricalPlanLink =
    proposal.planArtifactId &&
    proposal.planVersionAtCreation &&
    currentPlanVersion &&
    proposal.planVersionAtCreation !== currentPlanVersion;

  const handleCheckboxChange = () => {
    onSelect(proposal.id);
  };

  const handleEdit = (e: React.MouseEvent) => {
    e.stopPropagation();
    onEdit(proposal.id);
  };

  const handleRemove = (e: React.MouseEvent) => {
    e.stopPropagation();
    onRemove(proposal.id);
  };

  const handleViewHistoricalPlan = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (proposal.planArtifactId && proposal.planVersionAtCreation && onViewHistoricalPlan) {
      onViewHistoricalPlan(proposal.planArtifactId, proposal.planVersionAtCreation);
    }
  };

  return (
    <article
      data-testid={`proposal-card-${proposal.id}`}
      role="article"
      aria-labelledby={`proposal-title-${proposal.id}`}
      className="group relative p-3 rounded-lg border transition-all"
      style={{
        backgroundColor: "var(--bg-elevated)",
        borderColor: isSelected ? "#ff6b35" : "var(--border-subtle)",
        borderWidth: isSelected ? "2px" : "1px",
      }}
    >
      <div className="flex items-start gap-3">
        {/* Checkbox */}
        <div className="pt-0.5">
          <input
            type="checkbox"
            data-testid="proposal-checkbox"
            checked={isSelected}
            onChange={handleCheckboxChange}
            aria-label={`Select ${proposal.title}`}
            className="w-4 h-4 rounded border cursor-pointer"
            style={{
              accentColor: "#ff6b35",
            }}
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Title row */}
          <div className="flex items-start justify-between gap-2">
            <h3
              id={`proposal-title-${proposal.id}`}
              data-testid="proposal-title"
              className="font-medium leading-tight"
              style={{ color: "var(--text-primary)" }}
            >
              {proposal.title}
            </h3>

            {/* Action buttons (visible on hover) */}
            <div
              data-testid="proposal-actions"
              className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
            >
              <button
                data-testid="proposal-edit"
                onClick={handleEdit}
                aria-label="Edit proposal"
                className="p-1 rounded hover:bg-white/10 transition-colors"
                style={{ color: "var(--text-secondary)" }}
              >
                <EditIcon />
              </button>
              <button
                data-testid="proposal-remove"
                onClick={handleRemove}
                aria-label="Remove proposal"
                className="p-1 rounded hover:bg-white/10 transition-colors"
                style={{ color: "var(--text-secondary)" }}
              >
                <RemoveIcon />
              </button>
            </div>
          </div>

          {/* Description */}
          <p
            data-testid="proposal-description"
            className="text-sm mt-1 line-clamp-2"
            style={{ color: "var(--text-secondary)" }}
          >
            {proposal.description || "No description"}
          </p>

          {/* Badges row */}
          <div className="flex flex-wrap items-center gap-2 mt-2">
            {/* Priority badge */}
            <span
              data-testid="priority-badge"
              className="px-2 py-0.5 rounded text-xs font-medium"
              style={{
                backgroundColor: PRIORITY_COLORS[effectivePriority],
                color: "white",
              }}
            >
              {PRIORITY_LABELS[effectivePriority]}
            </span>

            {/* Category badge */}
            <span
              data-testid="category-badge"
              className="px-2 py-0.5 rounded text-xs"
              style={{
                backgroundColor: "var(--bg-hover)",
                color: "var(--text-secondary)",
              }}
            >
              {proposal.category}
            </span>

            {/* Complexity indicator */}
            {showComplexity && (
              <span
                data-testid="complexity-indicator"
                className="text-xs"
                style={{ color: "var(--text-muted)" }}
              >
                {proposal.estimatedComplexity}
              </span>
            )}

            {/* Modified indicator */}
            {proposal.userModified && (
              <span
                data-testid="modified-indicator"
                className="px-1.5 py-0.5 rounded text-xs italic"
                style={{
                  backgroundColor: "var(--bg-surface)",
                  color: "var(--text-muted)",
                }}
              >
                Modified
              </span>
            )}
          </div>

          {/* Dependency info */}
          {showDependencyInfo && (
            <div
              data-testid="dependency-info"
              className="flex items-center gap-3 mt-2 text-xs"
              style={{ color: "var(--text-muted)" }}
            >
              {dependsOnCount !== undefined && dependsOnCount > 0 && (
                <span className="flex items-center gap-1">
                  <DependencyIcon />
                  Depends on {dependsOnCount}
                </span>
              )}
              {blocksCount !== undefined && blocksCount > 0 && (
                <span className="flex items-center gap-1">
                  <BlocksIcon />
                  Blocks {blocksCount}
                </span>
              )}
            </div>
          )}

          {/* Historical plan link */}
          {showHistoricalPlanLink && (
            <div className="mt-2">
              <button
                data-testid="view-historical-plan"
                onClick={handleViewHistoricalPlan}
                className="text-xs underline hover:no-underline transition-all"
                style={{ color: "#ff6b35" }}
              >
                View plan as of proposal creation (v{proposal.planVersionAtCreation})
              </button>
            </div>
          )}
        </div>
      </div>
    </article>
  );
}
