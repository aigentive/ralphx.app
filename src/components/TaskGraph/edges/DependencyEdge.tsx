/**
 * DependencyEdge - Custom edge component for task dependency visualization
 *
 * Renders edges with different styles based on:
 * - Normal: dashed, muted, 1px (default dependencies)
 * - Critical path: solid, accent orange, 2px + glow
 * - Active (executing source): animated dotted
 */

import { memo } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from "@xyflow/react";
import { getEdgeStyleForEdge } from "./edgeStyles";

// ============================================================================
// Types
// ============================================================================

export interface DependencyEdgeData extends Record<string, unknown> {
  /** Whether this edge is on the critical path */
  isCriticalPath?: boolean;
  /** Status of the source (dependency) node */
  sourceStatus?: string;
  /** Optional label for the edge */
  label?: string;
  /** Whether edge crosses plan group boundaries (rendered on top) */
  isCrossPlan?: boolean;
}

// ============================================================================
// Component
// ============================================================================

function DependencyEdgeComponent({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  markerEnd,
  selected,
}: EdgeProps) {
  // Cast data to our expected type
  const edgeData = data as DependencyEdgeData | undefined;

  // Get edge style based on properties
  const isCriticalPath = edgeData?.isCriticalPath ?? false;
  const sourceStatus = edgeData?.sourceStatus;
  const edgeStyle = getEdgeStyleForEdge(isCriticalPath, sourceStatus);

  // Compute bezier path
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  return (
    <>
      {/* Shadow/glow layer for critical path edges */}
      {edgeStyle.filter && (
        <BaseEdge
          id={`${id}-glow`}
          path={edgePath}
          style={{
            stroke: edgeStyle.stroke,
            strokeWidth: edgeStyle.strokeWidth + 2,
            strokeOpacity: 0.3,
            filter: edgeStyle.filter,
          }}
        />
      )}

      {/* Main edge path */}
      <BaseEdge
        id={id}
        path={edgePath}
        {...(markerEnd ? { markerEnd } : {})}
        style={{
          stroke: edgeStyle.stroke,
          strokeWidth: selected ? edgeStyle.strokeWidth + 0.5 : edgeStyle.strokeWidth,
          strokeDasharray: edgeStyle.strokeDasharray,
          transition: "stroke-width 0.15s ease",
        }}
        className={edgeStyle.animated ? "react-flow__edge-path-animated" : ""}
      />

      {/* Optional label */}
      {edgeData?.label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "all",
            }}
            className="nodrag nopan px-1.5 py-0.5 rounded text-[10px] bg-bg-surface/90 border border-border-subtle text-text-muted"
          >
            {edgeData.label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

/**
 * Custom edge component for task dependencies
 *
 * Styles:
 * - Normal dependencies: dashed gray 1px
 * - Critical path: solid orange 2px with glow
 * - Active (from executing node): animated dotted orange
 *
 * @example
 * ```tsx
 * const edgeTypes = { dependency: DependencyEdge };
 *
 * // Edge with critical path styling
 * const edges = [{
 *   id: 'e1',
 *   source: 'task-1',
 *   target: 'task-2',
 *   type: 'dependency',
 *   data: { isCriticalPath: true }
 * }];
 *
 * <ReactFlow edgeTypes={edgeTypes} edges={edges} />
 * ```
 */
export const DependencyEdge = memo(DependencyEdgeComponent);
