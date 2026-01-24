/**
 * DependencyVisualization - Simple visualization of proposal dependencies
 *
 * Features:
 * - Lines connecting dependent proposals (SVG)
 * - Critical path highlighting
 * - Cycle warning indicators
 * - Compact mode for ApplyModal
 * - Vertical or horizontal layout
 */

import { useMemo } from "react";
import type { DependencyGraph } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

interface DependencyVisualizationProps {
  graph: DependencyGraph;
  compact?: boolean;
  direction?: "vertical" | "horizontal";
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Check if an edge is on the critical path
 */
function isEdgeOnCriticalPath(
  from: string,
  to: string,
  criticalPath: string[]
): boolean {
  const fromIndex = criticalPath.indexOf(from);
  const toIndex = criticalPath.indexOf(to);
  // Edge is on critical path if both nodes are adjacent in the path
  return fromIndex !== -1 && toIndex !== -1 && toIndex === fromIndex + 1;
}

/**
 * Check if a node is in any cycle
 */
function isNodeInCycle(
  nodeId: string,
  cycles: string[][] | null
): boolean {
  if (!cycles) return false;
  return cycles.some((cycle) => cycle.includes(nodeId));
}

// ============================================================================
// Component
// ============================================================================

export function DependencyVisualization({
  graph,
  compact = false,
  direction = "vertical",
}: DependencyVisualizationProps) {
  const { nodes, edges, criticalPath, hasCycles, cycles } = graph;

  // Calculate node positions for SVG edges
  const nodePositions = useMemo(() => {
    const positions = new Map<string, { x: number; y: number }>();
    const isHorizontal = direction === "horizontal";
    const spacing = compact ? 60 : 80;
    const nodeSize = compact ? 100 : 150;

    nodes.forEach((node, index) => {
      positions.set(node.proposalId, {
        x: isHorizontal ? index * (nodeSize + spacing) + nodeSize / 2 : nodeSize / 2,
        y: isHorizontal ? 30 : index * spacing + 20,
      });
    });
    return positions;
  }, [nodes, compact, direction]);

  // Empty state
  if (nodes.length === 0) {
    return (
      <div
        data-testid="dependency-visualization"
        data-compact={compact ? "true" : "false"}
        className="flex items-center justify-center p-4"
        style={{ backgroundColor: "var(--bg-surface)" }}
        aria-label="Dependency Graph"
      >
        <p
          className="text-sm italic"
          style={{ color: "var(--text-muted)" }}
        >
          No dependencies to display
        </p>
      </div>
    );
  }

  return (
    <div
      data-testid="dependency-visualization"
      data-compact={compact ? "true" : "false"}
      className="relative p-4"
      style={{ backgroundColor: "var(--bg-surface)" }}
      aria-label="Dependency Graph"
    >
      {/* Cycle Warning */}
      {hasCycles && (
        <div
          data-testid="cycle-warning"
          role="alert"
          className="flex items-center gap-2 mb-3 px-3 py-2 rounded text-sm"
          style={{
            backgroundColor: "rgba(239, 68, 68, 0.1)",
            color: "var(--status-error)",
          }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 1L15 14H1L8 1Z" stroke="currentColor" strokeWidth="1.5" fill="none" />
            <line x1="8" y1="5" x2="8" y2="9" stroke="currentColor" strokeWidth="1.5" />
            <circle cx="8" cy="11" r="1" fill="currentColor" />
          </svg>
          <span>Circular dependency detected</span>
        </div>
      )}

      {/* Critical Path Indicator */}
      {criticalPath.length > 0 && (
        <div
          data-testid="critical-path-indicator"
          className="flex items-center gap-2 mb-3 text-xs"
          style={{ color: "var(--text-secondary)" }}
        >
          <span
            className="w-3 h-0.5 rounded"
            style={{ backgroundColor: "var(--accent-primary)" }}
          />
          <span>Critical path</span>
        </div>
      )}

      {/* SVG Edges Layer */}
      {edges.length > 0 && (
        <svg
          data-testid="dependency-edges"
          role="img"
          aria-label="Dependency connections"
          className="absolute inset-0 pointer-events-none"
          style={{ width: "100%", height: "100%" }}
        >
          {edges.map((edge) => {
            const fromPos = nodePositions.get(edge.from);
            const toPos = nodePositions.get(edge.to);
            if (!fromPos || !toPos) return null;

            const isCritical = isEdgeOnCriticalPath(edge.from, edge.to, criticalPath);
            const strokeColor = isCritical ? "var(--accent-primary)" : "var(--border-subtle)";

            return (
              <line
                key={`${edge.from}-${edge.to}`}
                data-testid="dependency-edge"
                data-from={edge.from}
                data-to={edge.to}
                data-critical-path={isCritical ? "true" : "false"}
                x1={fromPos.x + 16}
                y1={fromPos.y + 16}
                x2={toPos.x + 16}
                y2={toPos.y}
                stroke={strokeColor}
                strokeWidth={isCritical ? 2 : 1}
                strokeDasharray={isCritical ? undefined : "4 2"}
              />
            );
          })}
        </svg>
      )}

      {/* Nodes Container */}
      <div
        data-testid="nodes-container"
        className={`relative flex gap-3 ${direction === "horizontal" ? "flex-row" : "flex-col"}`}
      >
        {nodes.map((node) => {
          const isOnCriticalPath = criticalPath.includes(node.proposalId);
          const isInCycle = isNodeInCycle(node.proposalId, cycles);

          return (
            <div
              key={node.proposalId}
              data-testid="dependency-node"
              data-in-degree={node.inDegree.toString()}
              data-out-degree={node.outDegree.toString()}
              data-critical-path={isOnCriticalPath ? "true" : "false"}
              data-in-cycle={isInCycle ? "true" : "false"}
              aria-label={`${node.title}, ${node.inDegree} dependencies, blocks ${node.outDegree}`}
              className={`
                flex items-center gap-2 px-3 py-2 rounded border transition-colors
                ${compact ? "compact-node truncate text-xs" : "text-sm"}
              `}
              style={{
                backgroundColor: "var(--bg-elevated)",
                borderColor: isOnCriticalPath
                  ? "var(--accent-primary)"
                  : isInCycle
                  ? "var(--status-error)"
                  : "var(--border-subtle)",
                color: "var(--text-primary)",
                borderWidth: isOnCriticalPath || isInCycle ? 2 : 1,
              }}
            >
              {/* Node content */}
              <span className={compact ? "truncate max-w-[120px]" : ""}>
                {node.title}
              </span>

              {/* Degree info - only in non-compact mode */}
              {!compact && (
                <span
                  data-testid="degree-info"
                  className="text-xs ml-auto"
                  style={{ color: "var(--text-muted)" }}
                >
                  {node.inDegree > 0 && `←${node.inDegree}`}
                  {node.inDegree > 0 && node.outDegree > 0 && " "}
                  {node.outDegree > 0 && `→${node.outDegree}`}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
