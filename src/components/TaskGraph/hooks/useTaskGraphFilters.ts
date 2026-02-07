/**
 * useTaskGraphFilters - State management for task graph filtering and grouping
 *
 * Manages:
 * - Filter state (status, plan, show completed)
 * - Grouping state (plan, tier, uncategorized)
 * - Layout direction (TB ↔ LR)
 * - Functions to filter nodes/edges based on selections
 *
 * @see specs/plans/task_graph_view.md section "Task E.4"
 */

import { useState, useCallback, useMemo } from "react";
import type { Node, Edge } from "@xyflow/react";
import type {
  GraphFilters,
  LayoutDirection,
  GroupingState,
} from "../controls/GraphControls";
import {
  DEFAULT_GRAPH_FILTERS,
  DEFAULT_LAYOUT_DIRECTION,
  DEFAULT_GROUPING,
} from "../controls/GraphControls";
import type { TaskGraphNode, TaskGraphEdge, PlanGroupInfo } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import { isGroupNodeId } from "../groups/groupTypes";

// ============================================================================
// Types
// ============================================================================


/**
 * Result of applying filters to graph data
 */
export interface FilteredGraphData {
  /** Filtered nodes (tasks that pass all filters) */
  nodes: TaskGraphNode[];
  /** Filtered edges (edges where both source and target are visible) */
  edges: TaskGraphEdge[];
  /** Plan groups with filtered task lists */
  planGroups: PlanGroupInfo[];
}

/**
 * Hook return type
 */
export interface UseTaskGraphFiltersReturn {
  /** Current filter state */
  filters: GraphFilters;
  /** Update filters */
  setFilters: (filters: GraphFilters) => void;
  /** Update a single filter property */
  updateFilter: <K extends keyof GraphFilters>(key: K, value: GraphFilters[K]) => void;
  /** Reset filters to defaults */
  resetFilters: () => void;

  /** Current layout direction */
  layoutDirection: LayoutDirection;
  /** Update layout direction */
  setLayoutDirection: (direction: LayoutDirection) => void;
  /** Toggle between TB and LR */
  toggleLayoutDirection: () => void;

  /** Current grouping option */
  grouping: GroupingState;
  /** Update grouping option */
  setGrouping: (grouping: GroupingState) => void;

  /** Check if a node passes all filters */
  nodePassesFilters: (node: TaskGraphNode) => boolean;

  /** Apply filters to graph data */
  applyFilters: (
    nodes: TaskGraphNode[],
    edges: TaskGraphEdge[],
    planGroups: PlanGroupInfo[]
  ) => FilteredGraphData;

  /** Check if any filters are active */
  hasActiveFilters: boolean;

  /** Count of active status filters */
  activeStatusCount: number;

  /** Count of active plan filters */
  activePlanCount: number;
}

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook for managing task graph filter and grouping state
 *
 * @example
 * ```tsx
 * const {
 *   filters,
 *   setFilters,
 *   layoutDirection,
 *   toggleLayoutDirection,
 *   grouping,
 *   setGrouping,
 *   applyFilters,
 * } = useTaskGraphFilters();
 *
 * // Apply filters to raw graph data
 * const filtered = applyFilters(graphData.nodes, graphData.edges, graphData.planGroups);
 * ```
 */
export function useTaskGraphFilters(): UseTaskGraphFiltersReturn {
  // Filter state
  const [filters, setFiltersState] = useState<GraphFilters>(DEFAULT_GRAPH_FILTERS);

  // Layout direction state
  const [layoutDirection, setLayoutDirection] = useState<LayoutDirection>(
    DEFAULT_LAYOUT_DIRECTION
  );

  // Grouping state
  const [grouping, setGrouping] = useState<GroupingState>(DEFAULT_GROUPING);

  // ============================================================================
  // Filter State Management
  // ============================================================================

  const setFilters = useCallback((newFilters: GraphFilters) => {
    setFiltersState(newFilters);
  }, []);

  const updateFilter = useCallback(
    <K extends keyof GraphFilters>(key: K, value: GraphFilters[K]) => {
      setFiltersState((prev) => ({ ...prev, [key]: value }));
    },
    []
  );

  const resetFilters = useCallback(() => {
    setFiltersState(DEFAULT_GRAPH_FILTERS);
  }, []);

  // ============================================================================
  // Layout Direction Management
  // ============================================================================

  const toggleLayoutDirection = useCallback(() => {
    setLayoutDirection((prev) => (prev === "TB" ? "LR" : "TB"));
  }, []);

  // ============================================================================
  // Filter Logic
  // ============================================================================

  /**
   * Check if a node passes all active filters
   */
  const nodePassesFilters = useCallback(
    (node: TaskGraphNode): boolean => {
      const status = node.internalStatus as InternalStatus;

      // Check status filter (empty = show all)
      if (filters.statuses.length > 0 && !filters.statuses.includes(status)) {
        return false;
      }

      // Check plan filter (empty = show all)
      if (filters.planIds.length > 0) {
        // Tasks without a plan are shown only if no plan filter is active
        if (!node.planArtifactId) {
          return false;
        }
        if (!filters.planIds.includes(node.planArtifactId)) {
          return false;
        }
      }

      return true;
    },
    [filters]
  );

  /**
   * Apply all filters to graph data
   */
  const applyFilters = useCallback(
    (
      nodes: TaskGraphNode[],
      edges: TaskGraphEdge[],
      planGroups: PlanGroupInfo[]
    ): FilteredGraphData => {
      // Filter nodes
      const filteredNodes = nodes.filter(nodePassesFilters);

      // Build set of visible node IDs for edge filtering
      const visibleNodeIds = new Set(filteredNodes.map((n) => n.taskId));

      // Filter edges - only keep edges where both source and target are visible
      const filteredEdges = edges.filter(
        (edge) => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)
      );

      // Update plan groups with filtered task lists
      const filteredPlanGroups = planGroups.map((group) => ({
        ...group,
        taskIds: group.taskIds.filter((taskId) => visibleNodeIds.has(taskId)),
      }));

      return {
        nodes: filteredNodes,
        edges: filteredEdges,
        planGroups: filteredPlanGroups,
      };
    },
    [nodePassesFilters]
  );

  // ============================================================================
  // Computed Values
  // ============================================================================

  const hasActiveFilters = useMemo(() => {
    return (
      filters.statuses.length > 0 ||
      filters.planIds.length > 0
    );
  }, [filters]);

  const activeStatusCount = filters.statuses.length;
  const activePlanCount = filters.planIds.length;

  return {
    // Filter state
    filters,
    setFilters,
    updateFilter,
    resetFilters,

    // Layout direction
    layoutDirection,
    setLayoutDirection,
    toggleLayoutDirection,

    // Grouping
    grouping,
    setGrouping,

    // Filter functions
    nodePassesFilters,
    applyFilters,

    // Computed values
    hasActiveFilters,
    activeStatusCount,
    activePlanCount,
  };
}

// ============================================================================
// Utility Functions for React Flow Integration
// ============================================================================

/**
 * Filter React Flow nodes based on task graph filters
 *
 * @param flowNodes - React Flow nodes with TaskNodeData
 * @param filters - Current filter state
 * @returns Filtered nodes
 */
export function filterFlowNodes<T extends Node>(
  flowNodes: T[],
  filters: GraphFilters
): T[] {
  return flowNodes.filter((node) => {
    // Skip group nodes (they have different data structure)
    if (node.type === "planGroup" || node.type === "tierGroup" || isGroupNodeId(node.id)) {
      return true;
    }

    const data = node.data as { internalStatus?: string; planArtifactId?: string | null } | undefined;
    if (!data) return true;

    const status = data.internalStatus as InternalStatus | undefined;
    if (!status) return true;

    // Check status filter (empty = show all)
    if (filters.statuses.length > 0 && !filters.statuses.includes(status)) {
      return false;
    }

    // Check plan filter (empty = show all)
    if (filters.planIds.length > 0) {
      const planArtifactId = data.planArtifactId;
      if (!planArtifactId) {
        return false;
      }
      if (!filters.planIds.includes(planArtifactId)) {
        return false;
      }
    }

    return true;
  });
}

/**
 * Filter React Flow edges based on visible node IDs
 *
 * @param flowEdges - React Flow edges
 * @param visibleNodeIds - Set of visible node IDs
 * @returns Filtered edges
 */
export function filterFlowEdges<T extends Edge>(
  flowEdges: T[],
  visibleNodeIds: Set<string>
): T[] {
  return flowEdges.filter(
    (edge) => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)
  );
}
