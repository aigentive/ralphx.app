/**
 * DependencyEdge - Custom edge component for task dependency visualization
 *
 * Renders edges with different styles based on:
 * - Normal: dashed, muted, 1px (default dependencies)
 * - Critical path: solid, accent orange, 2px + glow
 * - Active (executing source): animated dotted
 *
 * Features:
 * - Arrow markers pointing to target
 * - Center dot with tooltip showing relationship
 */

import { memo } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from "@xyflow/react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { getEdgeStyleForEdge, getEdgeType, MARKER_IDS } from "./edgeStyles";

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
  /** Source task title for tooltip */
  sourceLabel?: string;
  /** Target task title for tooltip */
  targetLabel?: string;
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
  selected,
}: EdgeProps) {
  // Cast data to our expected type
  const edgeData = data as DependencyEdgeData | undefined;

  // Get edge style based on properties
  const isCriticalPath = edgeData?.isCriticalPath ?? false;
  const sourceStatus = edgeData?.sourceStatus;
  const edgeStyle = getEdgeStyleForEdge(isCriticalPath, sourceStatus);
  const edgeType = getEdgeType(isCriticalPath, sourceStatus);

  // Get marker ID based on edge type
  const markerId = `url(#${MARKER_IDS[edgeType]})`;

  // Compute bezier path
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  // Determine if this edge has tooltip content
  const sourceLabel = edgeData?.sourceLabel;
  const targetLabel = edgeData?.targetLabel;
  const hasTooltip = sourceLabel && targetLabel;

  // Determine dot color based on edge type
  const isAccentEdge = edgeType === "critical" || edgeType === "active";

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

      {/* Main edge path with arrow marker */}
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerId}
        style={{
          stroke: edgeStyle.stroke,
          strokeWidth: selected ? edgeStyle.strokeWidth + 0.5 : edgeStyle.strokeWidth,
          strokeDasharray: edgeStyle.strokeDasharray,
          transition: "stroke-width 0.15s ease",
        }}
        className={edgeStyle.animated ? "react-flow__edge-path-animated" : ""}
      />

      {/* Center dot with tooltip showing relationship */}
      <EdgeLabelRenderer>
        {hasTooltip ? (
          <TooltipProvider delayDuration={200}>
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  style={{
                    position: "absolute",
                    transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
                    pointerEvents: "all",
                  }}
                  className={cn(
                    "w-1.5 h-1.5 rounded-full cursor-help transition-transform hover:scale-150",
                    isAccentEdge ? "bg-[hsl(14_100%_55%)]" : "bg-[hsl(220_10%_40%)]"
                  )}
                />
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs max-w-[250px]">
                <span className="text-[hsl(var(--text-primary))]">{sourceLabel}</span>
                <span className="text-[hsl(var(--text-muted))] mx-1">blocks</span>
                <span className="text-[hsl(var(--text-primary))]">{targetLabel}</span>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        ) : (
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "none",
            }}
            className={cn(
              "w-1.5 h-1.5 rounded-full",
              isAccentEdge ? "bg-[hsl(14_100%_55%)]" : "bg-[hsl(220_10%_40%)]"
            )}
          />
        )}
      </EdgeLabelRenderer>

      {/* Optional label (legacy support) */}
      {edgeData?.label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY + 16}px)`,
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
