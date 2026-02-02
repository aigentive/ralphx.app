/**
 * groupUtils.ts - Bounding box calculations for plan groups
 *
 * Utilities for calculating the bounding rectangle that encloses
 * all task nodes belonging to a plan group.
 */

import type { Node } from "@xyflow/react";

// ============================================================================
// Types
// ============================================================================

export interface BoundingBox {
  /** Minimum X coordinate */
  minX: number;
  /** Minimum Y coordinate */
  minY: number;
  /** Maximum X coordinate */
  maxX: number;
  /** Maximum Y coordinate */
  maxY: number;
  /** Width of bounding box */
  width: number;
  /** Height of bounding box */
  height: number;
}

export interface GroupBoundingBox extends BoundingBox {
  /** Plan artifact ID this box belongs to */
  planArtifactId: string;
  /** Task IDs contained in this group */
  taskIds: string[];
}

// ============================================================================
// Constants
// ============================================================================

/** Padding around grouped tasks inside the group region */
export const GROUP_PADDING = 12;

/** Extra space at top for the header */
export const HEADER_HEIGHT = 48;

/** Minimum group dimensions */
export const MIN_GROUP_WIDTH = 320;
export const MIN_GROUP_HEIGHT = 100;

// ============================================================================
// Functions
// ============================================================================

/**
 * Calculate the bounding box for a set of nodes
 *
 * @param nodes - React Flow nodes to calculate bounds for
 * @param nodeWidth - Width of each node (default: 180)
 * @param nodeHeight - Height of each node (default: 50)
 * @returns Bounding box or null if no nodes
 */
export function calculateBoundingBox(
  nodes: Node[],
  nodeWidth = 180,
  nodeHeight = 50
): BoundingBox | null {
  if (nodes.length === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of nodes) {
    const x = node.position.x;
    const y = node.position.y;

    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + nodeWidth);
    maxY = Math.max(maxY, y + nodeHeight);
  }

  return {
    minX,
    minY,
    maxX,
    maxY,
    width: maxX - minX,
    height: maxY - minY,
  };
}

/**
 * Calculate bounding boxes for multiple plan groups
 *
 * @param allNodes - All React Flow nodes
 * @param planGroups - Map of planArtifactId -> taskIds
 * @param nodeWidth - Width of each node (default: 180)
 * @param nodeHeight - Height of each node (default: 50)
 * @returns Array of bounding boxes for each plan group
 */
export function calculateGroupBoundingBoxes(
  allNodes: Node[],
  planGroups: Map<string, string[]>,
  nodeWidth = 180,
  nodeHeight = 50
): GroupBoundingBox[] {
  const results: GroupBoundingBox[] = [];

  // Create a lookup of node id -> node
  const nodeMap = new Map<string, Node>();
  for (const node of allNodes) {
    nodeMap.set(node.id, node);
  }

  for (const [planArtifactId, taskIds] of planGroups) {
    // Get nodes for this group
    const groupNodes: Node[] = [];
    for (const taskId of taskIds) {
      const node = nodeMap.get(taskId);
      if (node) {
        groupNodes.push(node);
      }
    }

    // Calculate bounding box
    const bbox = calculateBoundingBox(groupNodes, nodeWidth, nodeHeight);
    if (bbox) {
      results.push({
        ...bbox,
        planArtifactId,
        taskIds,
      });
    }
  }

  return results;
}

/**
 * Expand a bounding box with padding for the group container
 *
 * @param bbox - Original bounding box
 * @param padding - Padding to add on all sides
 * @param headerHeight - Extra height for the header
 * @returns Expanded bounding box
 */
export function expandBoundingBox(
  bbox: BoundingBox,
  padding: number = GROUP_PADDING,
  headerHeight: number = HEADER_HEIGHT
): BoundingBox {
  let minX = bbox.minX - padding;
  const minY = bbox.minY - padding - headerHeight;
  let maxX = bbox.maxX + padding;
  const maxY = bbox.maxY + padding;

  const naturalWidth = maxX - minX;
  const width = Math.max(naturalWidth, MIN_GROUP_WIDTH);

  // Center content horizontally if min width is enforced
  if (width > naturalWidth) {
    const extraWidth = width - naturalWidth;
    minX -= extraWidth / 2;
    maxX += extraWidth / 2;
  }

  return {
    minX,
    minY,
    maxX,
    maxY,
    width,
    height: Math.max(maxY - minY, MIN_GROUP_HEIGHT),
  };
}

/**
 * Convert a bounding box to a React Flow group node position and dimensions
 *
 * @param bbox - Bounding box (already expanded with padding)
 * @returns Position and dimensions for React Flow node
 */
export function boundingBoxToGroupNode(bbox: BoundingBox): {
  position: { x: number; y: number };
  width: number;
  height: number;
} {
  return {
    position: { x: bbox.minX, y: bbox.minY },
    width: bbox.width,
    height: bbox.height,
  };
}
