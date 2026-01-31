/**
 * DataPulse - Particles traveling along connection paths
 *
 * Animates multiple particles per path showing data flow:
 * - 5-8 particles per path traveling simultaneously
 * - Particle trails with fading tail effect
 * - Variable speeds (fast + slow particles)
 * - Bidirectional flow on each path
 * - CSS offset-path animation for performance
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

interface DataPulseProps {
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

/** Number of particles per path */
const PARTICLES_PER_PATH = 6;

/** Particle size range */
const PARTICLE_SIZE_MIN = 3;
const PARTICLE_SIZE_MAX = 6;

/** Speed range in seconds (lower = faster) */
const SPEED_FAST = 2;
const SPEED_SLOW = 4;

/** Seeded random for deterministic rendering */
function seededRandom(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) & 0x7fffffff;
    return state / 0x7fffffff;
  };
}

export default function DataPulse({
  agents,
  width,
  height,
  className = "",
}: DataPulseProps) {
  // Generate particle configurations
  const particles = useMemo(() => {
    const random = seededRandom(42);
    const hubX = (HUB_POSITION.x / 100) * width;
    const hubY = (HUB_POSITION.y / 100) * height;

    const allParticles: Array<{
      id: string;
      pathD: string;
      color: string;
      size: number;
      duration: number;
      delay: number;
      reverse: boolean;
    }> = [];

    agents.forEach((agent) => {
      const agentX = (agent.position.x / 100) * width;
      const agentY = (agent.position.y / 100) * height;

      // Calculate control point for quadratic bezier (matching ConnectionPaths)
      const midX = (agentX + hubX) / 2;
      const midY = (agentY + hubY) / 2;
      const dx = hubX - agentX;
      const dy = hubY - agentY;
      const len = Math.sqrt(dx * dx + dy * dy);
      const perpX = len > 0 ? (-dy / len) * 15 : 0;
      const perpY = len > 0 ? (dx / len) * 15 : 0;
      const controlX = midX + perpX;
      const controlY = midY + perpY;

      // Path from agent to hub
      const pathToHub = `M ${agentX} ${agentY} Q ${controlX} ${controlY} ${hubX} ${hubY}`;
      // Path from hub to agent (reverse)
      const pathFromHub = `M ${hubX} ${hubY} Q ${controlX} ${controlY} ${agentX} ${agentY}`;

      // Create particles for this path
      for (let i = 0; i < PARTICLES_PER_PATH; i++) {
        const reverse = i % 2 === 1; // Alternate direction for bidirectional flow
        const isFast = random() > 0.5;
        const baseDuration = isFast ? SPEED_FAST : SPEED_SLOW;
        const durationVariation = 0.5 + random() * 0.5; // 0.5-1.0 multiplier
        const duration = baseDuration * durationVariation;
        const delay = random() * duration; // Stagger start times
        const size = PARTICLE_SIZE_MIN + random() * (PARTICLE_SIZE_MAX - PARTICLE_SIZE_MIN);

        allParticles.push({
          id: `${agent.id}-${i}`,
          pathD: reverse ? pathFromHub : pathToHub,
          color: agent.color,
          size,
          duration,
          delay,
          reverse,
        });
      }
    });

    return allParticles;
  }, [agents, width, height]);

  // Generate unique path IDs for offset-path
  const pathDefs = useMemo(() => {
    const uniquePaths = new Map<string, string>();
    particles.forEach((p) => {
      const key = `pulse-path-${p.id}`;
      uniquePaths.set(key, p.pathD);
    });
    return Array.from(uniquePaths.entries());
  }, [particles]);

  return (
    <>
      {/* CSS animations for particles */}
      <style>
        {`
          @keyframes travelPath {
            0% {
              offset-distance: 0%;
              opacity: 0;
            }
            5% {
              opacity: 1;
            }
            90% {
              opacity: 0.8;
            }
            100% {
              offset-distance: 100%;
              opacity: 0;
            }
          }

          @keyframes particlePulse {
            0%, 100% {
              transform: scale(1);
              filter: blur(0px);
            }
            50% {
              transform: scale(1.3);
              filter: blur(1px);
            }
          }
        `}
      </style>

      {/* Hidden SVG for path definitions */}
      <svg
        width={0}
        height={0}
        style={{ position: "absolute", visibility: "hidden" }}
        aria-hidden="true"
      >
        <defs>
          {pathDefs.map(([id, d]) => (
            <path key={id} id={id} d={d} />
          ))}
        </defs>
      </svg>

      {/* Particle container */}
      <div
        className={`absolute inset-0 pointer-events-none ${className}`}
        style={{ overflow: "hidden" }}
      >
        {particles.map((particle) => (
            <div
              key={particle.id}
              style={{
                position: "absolute",
                width: particle.size,
                height: particle.size,
                borderRadius: "50%",
                backgroundColor: particle.color,
                boxShadow: `0 0 ${particle.size * 2}px ${particle.color}, 0 0 ${particle.size * 4}px ${particle.color}80`,
                offsetPath: `path("${particle.pathD}")`,
                animation: `travelPath ${particle.duration}s linear infinite, particlePulse ${particle.duration * 0.5}s ease-in-out infinite`,
                animationDelay: `${particle.delay}s, ${particle.delay}s`,
              }}
            />
        ))}

        {/* Trail particles (smaller, delayed copies for trail effect) */}
        {particles.map((particle) => (
          <div
            key={`${particle.id}-trail`}
            style={{
              position: "absolute",
              width: particle.size * 0.6,
              height: particle.size * 0.6,
              borderRadius: "50%",
              backgroundColor: particle.color,
              opacity: 0.4,
              boxShadow: `0 0 ${particle.size}px ${particle.color}60`,
              offsetPath: `path("${particle.pathD}")`,
              animation: `travelPath ${particle.duration}s linear infinite`,
              animationDelay: `${particle.delay + 0.1}s`,
            }}
          />
        ))}
      </div>
    </>
  );
}
