/**
 * AgentConstellation - Main orchestrator for Agent Network visualization
 *
 * Composes all visual elements for the welcome screen "constellation":
 * - CodeRain: Dense background code fragments drifting
 * - AmbientParticles: Floating dots throughout scene
 * - CentralHub: Pulsing core with ripple rings
 * - ConnectionPaths: SVG lines with glow connecting agents to hub
 * - DataPulse: Particles traveling along connection paths
 * - AgentNode: 4 agent nodes (Orchestrator, Worker, QA, Reviewer)
 *
 * Features:
 * - Staggered node entrance animation (fly in from edges)
 * - Mouse parallax effect on entire scene
 * - Proper layering: background → connections → hub → nodes → particles
 *
 * Anti-AI-Slop: Warm orange #ff6b35 only, no purple/blue gradients
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { motion, useMotionValue, useSpring } from "framer-motion";
import { Brain, Code2, Eye, ShieldCheck } from "lucide-react";

import AgentNode, { type AgentConfig } from "./AgentNode";
import AmbientParticles from "./AmbientParticles";
import CentralHub from "./CentralHub";
import CodeRain from "./CodeRain";
import ConnectionPaths from "./ConnectionPaths";
import DataPulse from "./DataPulse";

/**
 * AGENTS configuration array
 * Positions are percentages from top-left (0-100)
 */
const AGENTS: AgentConfig[] = [
  {
    id: "orchestrator",
    name: "Orchestrator",
    role: "Plans & coordinates",
    icon: Brain,
    color: "#ff6b35", // Warm orange accent
    position: { x: 25, y: 70 },
  },
  {
    id: "worker",
    name: "Worker",
    role: "Writes code",
    icon: Code2,
    color: "#4ade80", // Green
    position: { x: 25, y: 30 },
  },
  {
    id: "qa",
    name: "QA Refiner",
    role: "Validates quality",
    icon: ShieldCheck,
    color: "#60a5fa", // Blue
    position: { x: 75, y: 30 },
  },
  {
    id: "reviewer",
    name: "Reviewer",
    role: "Reviews changes",
    icon: Eye,
    color: "#f59e0b", // Amber
    position: { x: 75, y: 70 },
  },
];

/** Hub size in pixels */
const HUB_SIZE = 100;

/** Agent node size in pixels */
const NODE_SIZE = 80;

/** Parallax intensity (pixels of movement) */
const PARALLAX_INTENSITY = 20;

interface AgentConstellationProps {
  className?: string;
}

export default function AgentConstellation({
  className = "",
}: AgentConstellationProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });

  // Mouse position for parallax (raw values)
  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);

  // Smoothed parallax values using spring physics
  const parallaxX = useSpring(mouseX, { stiffness: 50, damping: 20 });
  const parallaxY = useSpring(mouseY, { stiffness: 50, damping: 20 });

  // Update dimensions on mount and resize
  useEffect(() => {
    const updateDimensions = () => {
      if (containerRef.current) {
        const { width, height } = containerRef.current.getBoundingClientRect();
        setDimensions({ width, height });
      }
    };

    updateDimensions();
    window.addEventListener("resize", updateDimensions);
    return () => window.removeEventListener("resize", updateDimensions);
  }, []);

  // Handle mouse movement for parallax effect
  const handleMouseMove = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (!containerRef.current) return;

      const { left, top, width, height } =
        containerRef.current.getBoundingClientRect();

      // Calculate mouse position relative to center (range: -0.5 to 0.5)
      const relativeX = (event.clientX - left) / width - 0.5;
      const relativeY = (event.clientY - top) / height - 0.5;

      // Apply parallax intensity
      mouseX.set(relativeX * PARALLAX_INTENSITY);
      mouseY.set(relativeY * PARALLAX_INTENSITY);
    },
    [mouseX, mouseY]
  );

  // Reset parallax when mouse leaves
  const handleMouseLeave = useCallback(() => {
    mouseX.set(0);
    mouseY.set(0);
  }, [mouseX, mouseY]);

  // Convert agent positions from percentage to pixels for absolute positioning
  const getAgentStyle = (agent: AgentConfig) => ({
    left: `${agent.position.x}%`,
    top: `${agent.position.y}%`,
    transform: "translate(-50%, -50%)",
  });

  return (
    <div
      ref={containerRef}
      className={`relative w-full h-full overflow-hidden ${className}`}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {/* Layer 1: Code Rain Background */}
      <CodeRain className="z-0" />

      {/* Layer 2: Ambient Particles (floats above code rain) */}
      <AmbientParticles className="z-10" />

      {/* Layer 3-6: Parallax-affected content */}
      <motion.div
        className="absolute inset-0"
        style={{
          x: parallaxX,
          y: parallaxY,
        }}
      >
        {/* Layer 3: Connection Paths (SVG lines) */}
        {dimensions.width > 0 && dimensions.height > 0 && (
          <ConnectionPaths
            agents={AGENTS}
            width={dimensions.width}
            height={dimensions.height}
            className="z-20"
          />
        )}

        {/* Layer 4: Data Pulses (particles on paths) */}
        {dimensions.width > 0 && dimensions.height > 0 && (
          <DataPulse
            agents={AGENTS}
            width={dimensions.width}
            height={dimensions.height}
            className="z-30"
          />
        )}

        {/* Layer 5: Central Hub (positioned at 50%, 50%) */}
        <div
          className="absolute z-40"
          style={{
            left: "50%",
            top: "50%",
            transform: "translate(-50%, -50%)",
          }}
        >
          <CentralHub size={HUB_SIZE} />
        </div>

        {/* Layer 6: Agent Nodes */}
        {AGENTS.map((agent, index) => (
          <div
            key={agent.id}
            className="absolute z-50"
            style={getAgentStyle(agent)}
          >
            <AgentNode agent={agent} size={NODE_SIZE} index={index} />
          </div>
        ))}
      </motion.div>
    </div>
  );
}
