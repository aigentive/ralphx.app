/**
 * ConnectionPaths - SVG lines connecting agents through central hub
 *
 * Creates glowing connection paths between agent nodes:
 * - SVG paths from each agent to the central hub
 * - Soft glow effect using SVG filters
 * - Dynamic path generation from agent positions
 *
 * Anti-AI-Slop: Warm orange #ff6b35 accent, no purple/blue gradients
 */

import { useMemo } from "react";

/** Agent position as percentage from center (0-100) */
interface AgentPosition {
  id: string;
  color: string;
  position: { x: number; y: number };
}

interface ConnectionPathsProps {
  /** Array of agent configurations with positions */
  agents: AgentPosition[];
  /** Width of the container in pixels */
  width: number;
  /** Height of the container in pixels */
  height: number;
  /** Optional class name */
  className?: string;
}

/** Center hub position (always at 50%, 50%) */
const HUB_POSITION = { x: 50, y: 50 };

/** Line configuration */
const LINE_WIDTH = 2;
const GLOW_INTENSITY = 6;

export default function ConnectionPaths({
  agents,
  width,
  height,
  className = "",
}: ConnectionPathsProps) {
  // Convert percentage positions to SVG coordinates
  const paths = useMemo(() => {
    const hubX = (HUB_POSITION.x / 100) * width;
    const hubY = (HUB_POSITION.y / 100) * height;

    return agents.map((agent) => {
      const agentX = (agent.position.x / 100) * width;
      const agentY = (agent.position.y / 100) * height;

      // Generate path ID for reference
      const pathId = `path-${agent.id}`;

      // Create a smooth quadratic bezier curve through the hub
      // Control point is offset to create a subtle curve
      const midX = (agentX + hubX) / 2;
      const midY = (agentY + hubY) / 2;

      // Add slight curve by offsetting control point perpendicular to the line
      const dx = hubX - agentX;
      const dy = hubY - agentY;
      const len = Math.sqrt(dx * dx + dy * dy);

      // Normalize and rotate 90 degrees for perpendicular offset
      const perpX = len > 0 ? (-dy / len) * 15 : 0;
      const perpY = len > 0 ? (dx / len) * 15 : 0;

      // Control point with slight curve
      const controlX = midX + perpX;
      const controlY = midY + perpY;

      // SVG path: Move to agent, quadratic curve to hub
      const d = `M ${agentX} ${agentY} Q ${controlX} ${controlY} ${hubX} ${hubY}`;

      return {
        id: pathId,
        d,
        color: agent.color,
        agentId: agent.id,
      };
    });
  }, [agents, width, height]);

  return (
    <svg
      width={width}
      height={height}
      className={`absolute inset-0 pointer-events-none ${className}`}
      style={{ overflow: "visible" }}
    >
      <defs>
        {/* Glow filter for soft edge effect */}
        <filter id="connection-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur in="SourceGraphic" stdDeviation={GLOW_INTENSITY} result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>

        {/* Per-agent gradient for color variation */}
        {paths.map((path) => (
          <linearGradient
            key={`gradient-${path.id}`}
            id={`gradient-${path.id}`}
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0%" stopColor={path.color} stopOpacity={0.6} />
            <stop offset="50%" stopColor="#ff6b35" stopOpacity={0.8} />
            <stop offset="100%" stopColor={path.color} stopOpacity={0.6} />
          </linearGradient>
        ))}
      </defs>

      {/* Render each connection path */}
      {paths.map((path) => (
        <g key={path.id}>
          {/* Glow layer (wider, blurred) */}
          <path
            d={path.d}
            fill="none"
            stroke={path.color}
            strokeWidth={LINE_WIDTH * 4}
            strokeLinecap="round"
            opacity={0.3}
            filter="url(#connection-glow)"
          />

          {/* Main line with gradient */}
          <path
            id={path.id}
            d={path.d}
            fill="none"
            stroke={`url(#gradient-${path.id})`}
            strokeWidth={LINE_WIDTH}
            strokeLinecap="round"
            opacity={0.8}
          />
        </g>
      ))}
    </svg>
  );
}
