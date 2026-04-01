/**
 * useDependencyGraphComplete - Hook to validate dependency graph completeness
 *
 * Checks if the dependency graph is ready for plan acceptance:
 * - All proposals have valid tier assignments
 * - No dangling dependencies (references to non-existent proposals)
 * - No cycles in the dependency graph
 */

import { useMemo } from "react";
import type { TaskProposal } from "@/types/ideation";
import type { DependencyGraphResponse } from "@/api/ideation.types";
import { computeDependencyTiers } from "./useDependencyGraph";

/**
 * Result of dependency graph validation
 */
export interface DependencyGraphValidation {
  /** Whether the graph is complete and ready for plan acceptance */
  isComplete: boolean;
  /** Whether there are cycles in the dependency graph */
  hasCycles: boolean;
  /** Whether there are dangling dependencies (references to non-existent proposals) */
  hasDanglingDependencies: boolean;
  /** Whether all proposals have valid tier assignments */
  allProposalsHaveTiers: boolean;
  /** List of proposal IDs that have no tier assignment */
  untieredProposalIds: string[];
  /** List of edges that reference non-existent proposals */
  danglingEdges: Array<{ from: string; to: string }>;
  /** Human-readable validation message */
  message: string;
}

/**
 * Validate the dependency graph for completeness.
 *
 * @param proposals - List of proposals in the session
 * @param graph - The dependency graph (can be null/undefined)
 * @returns Validation result with detailed information
 */
export function validateDependencyGraph(
  proposals: TaskProposal[],
  graph: DependencyGraphResponse | null | undefined
): DependencyGraphValidation {
  // If no proposals, the graph is trivially complete
  if (proposals.length === 0) {
    return {
      isComplete: true,
      hasCycles: false,
      hasDanglingDependencies: false,
      allProposalsHaveTiers: true,
      untieredProposalIds: [],
      danglingEdges: [],
      message: "No proposals to validate",
    };
  }

  // Build set of valid proposal IDs
  const validProposalIds = new Set(proposals.map((p) => p.id));

  // Check for cycles
  const hasCycles = graph?.hasCycles ?? false;

  // Check for dangling dependencies (edges that reference non-existent proposals)
  const danglingEdges: Array<{ from: string; to: string }> = [];
  if (graph?.edges) {
    for (const edge of graph.edges) {
      if (!validProposalIds.has(edge.from) || !validProposalIds.has(edge.to)) {
        danglingEdges.push({ from: edge.from, to: edge.to });
      }
    }
  }
  const hasDanglingDependencies = danglingEdges.length > 0;

  // Compute tiers and check if all proposals have tier assignments
  const { tierMap } = computeDependencyTiers(graph);
  const untieredProposalIds: string[] = [];

  for (const proposal of proposals) {
    // A proposal has a tier if it's in the tier map OR if there's no graph
    // (when there's no graph, proposals are implicitly tier 0)
    if (graph && graph.nodes.length > 0) {
      if (!tierMap.has(proposal.id)) {
        untieredProposalIds.push(proposal.id);
      }
    }
  }
  const allProposalsHaveTiers = untieredProposalIds.length === 0;

  // Determine overall completeness
  const isComplete = !hasCycles && !hasDanglingDependencies && allProposalsHaveTiers;

  // Build human-readable message
  let message = "";
  if (isComplete) {
    message = "Dependency graph is complete";
  } else {
    const issues: string[] = [];
    if (hasCycles) {
      issues.push("circular dependencies detected");
    }
    if (hasDanglingDependencies) {
      issues.push(`${danglingEdges.length} invalid dependency reference(s)`);
    }
    if (!allProposalsHaveTiers) {
      issues.push(`${untieredProposalIds.length} proposal(s) without tier assignment`);
    }
    message = `Cannot accept plan: ${issues.join(", ")}`;
  }

  return {
    isComplete,
    hasCycles,
    hasDanglingDependencies,
    allProposalsHaveTiers,
    untieredProposalIds,
    danglingEdges,
    message,
  };
}

/**
 * Hook to check if the dependency graph is complete and ready for plan acceptance.
 *
 * A complete graph:
 * 1. Has no cycles (cycles prevent proper execution ordering)
 * 2. Has no dangling dependencies (all referenced proposals exist)
 * 3. All proposals have valid tier assignments
 *
 * @param proposals - List of proposals in the session
 * @param graph - The dependency graph response (from useDependencyGraph)
 * @returns boolean - true if graph is complete, false otherwise
 *
 * @example
 * ```tsx
 * const { data: graph } = useDependencyGraph(sessionId);
 * const isGraphComplete = useDependencyGraphComplete(proposals, graph);
 *
 * <Button disabled={!isGraphComplete}>
 *   Accept Plan
 * </Button>
 * ```
 */
export function useDependencyGraphComplete(
  proposals: TaskProposal[],
  graph: DependencyGraphResponse | null | undefined
): boolean {
  return useMemo(() => {
    const validation = validateDependencyGraph(proposals, graph);
    return validation.isComplete;
  }, [proposals, graph]);
}

/**
 * Hook to get detailed validation information about the dependency graph.
 *
 * Use this when you need to show why the graph is incomplete.
 *
 * @param proposals - List of proposals in the session
 * @param graph - The dependency graph response (from useDependencyGraph)
 * @returns DependencyGraphValidation with detailed validation results
 *
 * @example
 * ```tsx
 * const { data: graph } = useDependencyGraph(sessionId);
 * const validation = useDependencyGraphValidation(proposals, graph);
 *
 * {!validation.isComplete && (
 *   <Tooltip content={validation.message}>
 *     <AlertCircle className="text-warning" />
 *   </Tooltip>
 * )}
 * ```
 */
export function useDependencyGraphValidation(
  proposals: TaskProposal[],
  graph: DependencyGraphResponse | null | undefined
): DependencyGraphValidation {
  return useMemo(() => validateDependencyGraph(proposals, graph), [proposals, graph]);
}
