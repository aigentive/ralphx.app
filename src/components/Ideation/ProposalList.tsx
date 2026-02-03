/**
 * ProposalList - Sortable list of task proposals
 *
 * Features:
 * - List of ProposalCard components
 * - Drag-to-reorder with @dnd-kit/sortable
 * - Multi-select with Shift+click
 * - Select all / Deselect all buttons
 * - Sort by priority button
 * - Clear all button
 * - Empty state when no proposals
 */

import React, { useCallback, useMemo } from "react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ProposalCard } from "./ProposalCard";
import type { TaskProposal } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

export interface DependencyCounts {
  [proposalId: string]: {
    dependsOn: number;
    blocks: number;
  };
}

export interface ProposalListProps {
  /** List of proposals to display */
  proposals: TaskProposal[];
  /** Callback when edit is clicked */
  onEdit: (proposalId: string) => void;
  /** Callback when remove is clicked */
  onRemove: (proposalId: string) => void;
  /** Callback when proposals are reordered */
  onReorder: (proposalIds: string[]) => void;
  /** Callback for sort by priority */
  onSortByPriority: () => void;
  /** Callback for clear all */
  onClearAll: () => void;
  /** Dependency counts for each proposal */
  dependencyCounts?: DependencyCounts;
}

// ============================================================================
// Icons
// ============================================================================

function SortIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M3 5L7 1L11 5M3 9L7 13L11 9"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ClearIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M2 4H12M5 4V2H9V4M3 4V12C3 12.5523 3.44772 13 4 13H10C10.5523 13 11 12.5523 11 12V4"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ============================================================================
// Sortable Proposal Card
// ============================================================================

interface SortableProposalCardProps {
  proposal: TaskProposal;
  onEdit: (proposalId: string) => void;
  onRemove: (proposalId: string) => void;
  dependsOnCount?: number;
  blocksCount?: number;
}

const SortableProposalCard = React.memo(function SortableProposalCard({
  proposal,
  onEdit,
  onRemove,
  dependsOnCount,
  blocksCount,
}: SortableProposalCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: proposal.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  // Build optional props conditionally for exactOptionalPropertyTypes
  const optionalProps = {
    ...(dependsOnCount !== undefined && { dependsOnCount }),
    ...(blocksCount !== undefined && { blocksCount }),
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      data-draggable="true"
    >
      <ProposalCard
        proposal={proposal}
        onEdit={onEdit}
        onRemove={onRemove}
        {...optionalProps}
      />
    </div>
  );
});

// ============================================================================
// Empty State
// ============================================================================

function EmptyState() {
  return (
    <div
      data-testid="proposal-list-empty"
      className="flex flex-col items-center justify-center py-12 px-4 text-center"
    >
      <svg
        width="48"
        height="48"
        viewBox="0 0 48 48"
        fill="none"
        className="mb-4"
        style={{ color: "var(--text-muted)" }}
      >
        <rect
          x="8"
          y="12"
          width="32"
          height="24"
          rx="4"
          stroke="currentColor"
          strokeWidth="2"
          strokeDasharray="4 4"
        />
        <path
          d="M16 20h16M16 26h12M16 32h8"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>
      <p
        className="font-medium mb-1"
        style={{ color: "var(--text-secondary)" }}
      >
        No proposals yet
      </p>
      <p className="text-sm" style={{ color: "var(--text-muted)" }}>
        Chat with the orchestrator to generate task proposals
      </p>
    </div>
  );
}

// ============================================================================
// Toolbar
// ============================================================================

interface ToolbarProps {
  totalCount: number;
  onSortByPriority: () => void;
  onClearAll: () => void;
}

function Toolbar({
  totalCount,
  onSortByPriority,
  onClearAll,
}: ToolbarProps) {
  return (
    <div
      data-testid="proposal-list-toolbar"
      className="flex items-center justify-between mb-3 px-1"
    >
      <div className="flex items-center gap-2">
        <span className="text-sm" style={{ color: "var(--text-muted)" }}>
          of {totalCount}
        </span>
      </div>

      <div className="flex items-center gap-1">
        <button
          data-testid="sort-priority-btn"
          onClick={onSortByPriority}
          aria-label="Sort by priority"
          className="p-1.5 rounded hover:bg-white/10 transition-colors"
          style={{ color: "var(--text-secondary)" }}
          title="Sort by priority"
        >
          <SortIcon />
        </button>

        <button
          data-testid="clear-all-btn"
          onClick={onClearAll}
          aria-label="Clear all"
          className="p-1.5 rounded hover:bg-white/10 transition-colors"
          style={{ color: "var(--text-secondary)" }}
          title="Clear all"
        >
          <ClearIcon />
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ProposalList({
  proposals,
  onEdit,
  onRemove,
  onReorder,
  onSortByPriority,
  onClearAll,
  dependencyCounts,
}: ProposalListProps) {
  // Sort proposals by sortOrder
  const sortedProposals = useMemo(
    () => [...proposals].sort((a, b) => a.sortOrder - b.sortOrder),
    [proposals]
  );

  const proposalIds = useMemo(
    () => sortedProposals.map((p) => p.id),
    [sortedProposals]
  );

  // DnD sensors
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  // Handle drag end
  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;

      if (over && active.id !== over.id) {
        const oldIndex = proposalIds.indexOf(active.id as string);
        const newIndex = proposalIds.indexOf(over.id as string);

        const newOrder = [...proposalIds];
        newOrder.splice(oldIndex, 1);
        newOrder.splice(newIndex, 0, active.id as string);

        onReorder(newOrder);
      }
    },
    [proposalIds, onReorder]
  );

  if (proposals.length === 0) {
    return (
      <div data-testid="proposal-list">
        <EmptyState />
      </div>
    );
  }

  return (
    <div data-testid="proposal-list">
      <Toolbar
        totalCount={proposals.length}
        onSortByPriority={onSortByPriority}
        onClearAll={onClearAll}
      />

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={proposalIds}
          strategy={verticalListSortingStrategy}
        >
          <div
            data-testid="proposal-list-sortable"
            role="list"
            className="space-y-2"
            onClick={(e) => e.stopPropagation()}
          >
            {sortedProposals.map((proposal) => {
              const deps = dependencyCounts?.[proposal.id];
              const depProps = {
                ...(deps?.dependsOn !== undefined && { dependsOnCount: deps.dependsOn }),
                ...(deps?.blocks !== undefined && { blocksCount: deps.blocks }),
              };
              return (
                <SortableProposalCard
                  key={proposal.id}
                  proposal={proposal}
                  onEdit={onEdit}
                  onRemove={onRemove}
                  {...depProps}
                />
              );
            })}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
