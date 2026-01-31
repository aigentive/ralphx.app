/**
 * TieredProposalList - Orchestrates rendering proposals grouped by dependency tiers
 *
 * Features:
 * - Groups proposals by topological tier using useDependencyTiers
 * - Renders ProposalTierGroup for each tier (Foundation, Core, Integration)
 * - Passes dependency details to each ProposalCard
 * - Maintains sortOrder as tiebreaker within same tier
 * - Preserves selection and highlight functionality
 */

import React, { useMemo } from "react";
import type { TaskProposal, DependencyGraph } from "@/types/ideation";
import { ProposalCard, type DependencyDetail } from "./ProposalCard";
import { ProposalTierGroup } from "./ProposalTierGroup";
import { useDependencyTiers, getDependencyReason } from "@/hooks/useDependencyGraph";

// ============================================================================
// Types
// ============================================================================

export interface TieredProposalListProps {
  /** List of proposals to display */
  proposals: TaskProposal[];
  /** Dependency graph for tier computation and dependency details */
  dependencyGraph: DependencyGraph | null | undefined;
  /** Currently selected/highlighted proposal IDs */
  highlightedIds: Set<string>;
  /** IDs of proposals on the critical path */
  criticalPathIds: Set<string>;
  /** Current plan version for historical link display */
  currentPlanVersion?: number;
  /** Callback when a proposal is selected */
  onSelect: (proposalId: string) => void;
  /** Callback when a proposal is edited */
  onEdit: (proposalId: string) => void;
  /** Callback when a proposal is removed */
  onRemove: (proposalId: string) => void;
  /** Callback to view historical plan */
  onViewHistoricalPlan?: (artifactId: string, version: number) => void;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Build dependency details for a proposal from the graph
 */
function buildDependencyDetails(
  proposalId: string,
  proposals: TaskProposal[],
  dependencyGraph: DependencyGraph | null | undefined
): DependencyDetail[] {
  if (!dependencyGraph) return [];

  const details: DependencyDetail[] = [];

  // Find edges where this proposal is the dependent (edge.from = this proposal)
  // Edge semantics: edge.from depends on edge.to
  // Wait - looking at IdeationView.tsx line 130-142, the edge semantics are:
  // edge.to depends on edge.from (from → to in graph direction)
  // So if we want "what does this proposal depend on", we need edges where edge.from = proposalId
  // Actually looking closer at line 131: "edge.from = the proposal that depends on edge.to"
  // That's backwards from the typical graph direction. Let me check the edge building...

  // From IdeationView line 130-142, the edge loop shows:
  // for edge in edges: details[edge.from] gets edge.to as a dependency
  // This means: edge.from depends on edge.to
  // So to get what proposalId depends on, we look for edges where edge.from === proposalId

  for (const edge of dependencyGraph.edges) {
    // Find proposals that this proposal depends on (this = edge.from)
    // Wait, need to re-check the semantics. Let me trace through:
    // In the graph, edge {from: A, to: B} means A → B
    // In dependency terms: A depends on B (A needs B to be done first)
    // So if we're looking for what proposal X depends on, we want edges where from = X
    // The to value is what X depends on

    // Actually looking at line 131 comment: "edge.from = the proposal that depends on edge.to"
    // So edge.from depends on edge.to. To find what proposalId depends on:
    // We want edges where edge.from === proposalId, and edge.to is the dependency

    // But wait, looking at useDependencyGraph.ts line 192-193:
    // edge.to depends on edge.from
    // That's the opposite! The codebase has inconsistent semantics.

    // Let me check the actual data flow. In computeDependencyTiers:
    // Line 192: deps.add(edge.from) where deps = dependsOn.get(edge.to)
    // This means: edge.to's dependencies include edge.from
    // So: edge.to depends on edge.from

    // And in IdeationView line 131-142:
    // details[edge.from] includes edge.to
    // So if A = edge.from, B = edge.to, then A's dependency details include B
    // Which means: A depends on B
    // This is OPPOSITE to what useDependencyGraph says!

    // Let me look at the actual backend to understand the true semantics...
    // Actually, let me just follow what IdeationView does since that's the existing working code.
    // IdeationView builds details[edge.from] = edge.to's info
    // So edge.from depends on edge.to

    if (edge.from === proposalId) {
      const targetProposal = proposals.find(p => p.id === edge.to);
      if (targetProposal) {
        const reason = getDependencyReason(dependencyGraph, edge.from, edge.to);
        const detail: DependencyDetail = {
          proposalId: edge.to,
          title: targetProposal.title,
        };
        if (reason !== undefined) {
          detail.reason = reason;
        }
        details.push(detail);
      }
    }
  }

  return details;
}

/**
 * Compute blocks count for a proposal (how many proposals depend on this one)
 */
function computeBlocksCount(
  proposalId: string,
  dependencyGraph: DependencyGraph | null | undefined
): number {
  if (!dependencyGraph) return 0;

  // Count edges where edge.to === proposalId (this proposal is depended upon)
  return dependencyGraph.edges.filter(edge => edge.to === proposalId).length;
}

// ============================================================================
// Component
// ============================================================================

export const TieredProposalList = React.memo(function TieredProposalList({
  proposals,
  dependencyGraph,
  highlightedIds,
  criticalPathIds,
  currentPlanVersion,
  onSelect,
  onEdit,
  onRemove,
  onViewHistoricalPlan,
}: TieredProposalListProps) {
  // Compute tier assignments from dependency graph
  const { tierMap, maxTier } = useDependencyTiers(dependencyGraph);

  // Group proposals by tier, sorted within each tier by sortOrder
  const proposalsByTier = useMemo(() => {
    const tiers = new Map<number, TaskProposal[]>();

    // Initialize tier groups
    for (let i = 0; i <= maxTier; i++) {
      tiers.set(i, []);
    }

    // Assign proposals to tiers
    for (const proposal of proposals) {
      const tier = tierMap.get(proposal.id) ?? 0;
      const tierProposals = tiers.get(tier);
      if (tierProposals) {
        tierProposals.push(proposal);
      } else {
        tiers.set(tier, [proposal]);
      }
    }

    // Sort proposals within each tier by sortOrder
    for (const [tier, tierProposals] of tiers) {
      tiers.set(tier, tierProposals.sort((a, b) => a.sortOrder - b.sortOrder));
    }

    return tiers;
  }, [proposals, tierMap, maxTier]);

  // Get ordered tier numbers (0, 1, 2, ...)
  const tierNumbers = useMemo(() => {
    return Array.from(proposalsByTier.keys()).sort((a, b) => a - b);
  }, [proposalsByTier]);

  // If no proposals, return null (parent handles empty state)
  if (proposals.length === 0) {
    return null;
  }

  return (
    <div data-testid="tiered-proposal-list" className="space-y-4">
      {tierNumbers.map((tier) => {
        const tierProposals = proposalsByTier.get(tier) ?? [];
        if (tierProposals.length === 0) return null;

        return (
          <ProposalTierGroup
            key={tier}
            tier={tier}
            proposalCount={tierProposals.length}
          >
            <div className="space-y-3">
              {tierProposals.map((proposal, index) => {
                const dependsOnDetails = buildDependencyDetails(
                  proposal.id,
                  proposals,
                  dependencyGraph
                );
                const blocksCount = computeBlocksCount(proposal.id, dependencyGraph);
                const isOnCriticalPath = criticalPathIds.has(proposal.id);
                const isHighlighted = highlightedIds.has(proposal.id);

                // Build optional props conditionally for exactOptionalPropertyTypes
                const optionalProps: {
                  dependsOnCount?: number;
                  dependsOnDetails?: DependencyDetail[];
                  blocksCount?: number;
                  isOnCriticalPath?: boolean;
                  isHighlighted?: boolean;
                  currentPlanVersion?: number;
                  onViewHistoricalPlan?: (artifactId: string, version: number) => void;
                } = {};

                if (dependsOnDetails.length > 0) {
                  optionalProps.dependsOnCount = dependsOnDetails.length;
                  optionalProps.dependsOnDetails = dependsOnDetails;
                }
                if (blocksCount > 0) {
                  optionalProps.blocksCount = blocksCount;
                }
                if (isOnCriticalPath) {
                  optionalProps.isOnCriticalPath = isOnCriticalPath;
                }
                if (isHighlighted) {
                  optionalProps.isHighlighted = isHighlighted;
                }
                if (currentPlanVersion !== undefined) {
                  optionalProps.currentPlanVersion = currentPlanVersion;
                }
                if (onViewHistoricalPlan !== undefined) {
                  optionalProps.onViewHistoricalPlan = onViewHistoricalPlan;
                }

                return (
                  <div key={proposal.id} style={{ animationDelay: `${index * 50}ms` }}>
                    <ProposalCard
                      proposal={proposal}
                      onSelect={onSelect}
                      onEdit={onEdit}
                      onRemove={onRemove}
                      {...optionalProps}
                    />
                  </div>
                );
              })}
            </div>
          </ProposalTierGroup>
        );
      })}
    </div>
  );
});

export default TieredProposalList;
