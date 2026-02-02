/**
 * GraphMiniMap - Custom MiniMap with status-based node coloring
 *
 * Uses React Flow's MiniMap with a custom nodeColor function that
 * colors nodes by their task status using colors from nodeStyles.ts.
 *
 * Per spec: Phase E.5 of Task Graph View implementation
 */

import { memo } from "react";
import { MiniMap, type MiniMapProps, type Node } from "@xyflow/react";
import { getStatusBorderColor } from "../nodes/nodeStyles";

// ============================================================================
// Types
// ============================================================================

export interface GraphMiniMapProps {
  /** Additional className for the MiniMap container */
  className?: string;
  /** Whether to show the MiniMap (default: true) */
  visible?: boolean;
  /** Position style override */
  style?: React.CSSProperties;
}

// ============================================================================
// Node Color Function
// ============================================================================

/**
 * Gets the color for a node in the MiniMap based on its internal status.
 * Falls back to 'backlog' color for group nodes or nodes without status.
 */
function getNodeColor(node: Node): string {
  // Skip group nodes (they start with "group-")
  if (node.id.startsWith("group-")) {
    // Return a subtle color for group nodes
    return "hsla(220 10% 30% / 0.3)";
  }

  // Extract internal status from node data
  const data = node.data as { internalStatus?: string } | undefined;
  const status = data?.internalStatus ?? "backlog";

  return getStatusBorderColor(status);
}

// ============================================================================
// Default Styles
// ============================================================================

const DEFAULT_MASK_COLOR = "hsla(220 10% 5% / 0.8)";

const DEFAULT_STYLE: React.CSSProperties = {
  background: "hsla(220 10% 12% / 0.9)",
  border: "1px solid hsla(220 20% 100% / 0.08)",
  borderRadius: 8,
};

// ============================================================================
// Component
// ============================================================================

/**
 * Custom MiniMap component that colors nodes by their task status.
 *
 * Uses the shared nodeStyles.ts color mapping to ensure consistency
 * between the main graph and the minimap.
 */
function GraphMiniMapComponent({
  className,
  visible = true,
  style,
}: GraphMiniMapProps) {
  if (!visible) {
    return null;
  }

  const miniMapProps: Partial<MiniMapProps> = {
    nodeColor: getNodeColor,
    maskColor: DEFAULT_MASK_COLOR,
    style: {
      ...DEFAULT_STYLE,
      ...style,
    },
    className,
    "aria-label": "Graph minimap",
  };

  return <MiniMap {...miniMapProps} />;
}

/**
 * Memoized GraphMiniMap component
 */
export const GraphMiniMap = memo(GraphMiniMapComponent);
