/**
 * edgeStyles.ts - Edge styling for Task Graph dependency edges
 *
 * Defines styles for different edge types:
 * - Normal: dashed, muted, 1px
 * - Critical path: solid, accent orange, 2px + glow
 * - Active (executing source): animated dotted
 */

// ============================================================================
// Types
// ============================================================================

export interface EdgeStyle {
  /** Stroke color */
  stroke: string;
  /** Stroke width in pixels */
  strokeWidth: number;
  /** Optional stroke dash array for dashed/dotted lines */
  strokeDasharray?: string;
  /** Whether edge should be animated */
  animated: boolean;
  /** Optional filter for glow effect */
  filter?: string;
}

export type EdgeType = "normal" | "critical" | "active";

// ============================================================================
// Color Definitions
// ============================================================================

/** Muted gray for normal dependency edges */
const NORMAL_STROKE = "hsl(220 10% 40%)";

/** Accent orange for critical path - matches EXECUTING_COLORS from nodeStyles */
const CRITICAL_STROKE = "hsl(14 100% 55%)";

/** Animated dotted style for edges from executing nodes */
const ACTIVE_STROKE = "hsl(14 100% 55%)";

// ============================================================================
// Edge Styles
// ============================================================================

/**
 * Style for normal dependency edges
 * Dashed, muted gray, 1px
 */
export const NORMAL_EDGE_STYLE: EdgeStyle = {
  stroke: NORMAL_STROKE,
  strokeWidth: 1,
  strokeDasharray: "5 5",
  animated: false,
};

/**
 * Style for critical path edges
 * Solid, accent orange, 2px with glow
 */
export const CRITICAL_EDGE_STYLE: EdgeStyle = {
  stroke: CRITICAL_STROKE,
  strokeWidth: 2,
  animated: false,
  filter: "drop-shadow(0 0 4px hsla(14 100% 55% / 0.5))",
};

/**
 * Style for active edges (from executing nodes)
 * Animated dotted, accent orange, 1.5px
 */
export const ACTIVE_EDGE_STYLE: EdgeStyle = {
  stroke: ACTIVE_STROKE,
  strokeWidth: 1.5,
  strokeDasharray: "3 3",
  animated: true,
};

// ============================================================================
// Style Getters
// ============================================================================

/**
 * Get the edge type based on properties
 *
 * @param isCriticalPath - Whether edge is on the critical path
 * @param sourceStatus - Status of the source node (optional)
 * @returns EdgeType
 */
export function getEdgeType(
  isCriticalPath: boolean,
  sourceStatus?: string
): EdgeType {
  // Active takes priority - edge from executing node
  if (sourceStatus === "executing" || sourceStatus === "re_executing") {
    return "active";
  }

  // Critical path
  if (isCriticalPath) {
    return "critical";
  }

  // Default normal
  return "normal";
}

/**
 * Get the complete edge style for a given type
 */
export function getEdgeStyle(edgeType: EdgeType): EdgeStyle {
  switch (edgeType) {
    case "critical":
      return CRITICAL_EDGE_STYLE;
    case "active":
      return ACTIVE_EDGE_STYLE;
    case "normal":
    default:
      return NORMAL_EDGE_STYLE;
  }
}

/**
 * Get edge style based on edge properties
 * Convenience function combining type detection and style lookup
 *
 * @param isCriticalPath - Whether edge is on the critical path
 * @param sourceStatus - Status of the source node (optional)
 * @returns EdgeStyle
 */
export function getEdgeStyleForEdge(
  isCriticalPath: boolean,
  sourceStatus?: string
): EdgeStyle {
  const edgeType = getEdgeType(isCriticalPath, sourceStatus);
  return getEdgeStyle(edgeType);
}
