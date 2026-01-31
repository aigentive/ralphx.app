/**
 * AgentConstellation - Main orchestrator for Agent Network visualization
 *
 * Displays all 13 RalphX agents orbiting around a central hub:
 * - Inner orbit (4): Orchestrator, Worker, Reviewer, QA
 * - Middle orbit (4): Supervisor, Researcher, Ideation, QA Prep
 * - Outer orbit (5): Chat agents, Namer, Dependencies
 *
 * Features:
 * - True orbital physics with smooth animation
 * - Responsive - orbits scale with viewport while staying in bounds
 * - Connection lines from agents to central hub
 * - Mouse parallax effect on entire scene
 *
 * Anti-AI-Slop: Warm orange #ff6b35 only, no purple/blue gradients
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { motion, useMotionValue, useSpring } from "framer-motion";

import AmbientParticles from "./AmbientParticles";
import CentralHub from "./CentralHub";
import CodeRain from "./CodeRain";
import OrbitingAgentNode from "./OrbitingAgentNode";
import { ORBITING_AGENTS, calculateOrbitalRadius } from "./agentConfig";

/** Hub size in pixels */
const HUB_SIZE = 80;

/** Parallax intensity (pixels of movement) */
const PARALLAX_INTENSITY = 15;

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

  const centerX = dimensions.width / 2;
  const centerY = dimensions.height / 2;

  // Calculate orbital radii for the orbit path circles
  const innerRadius = calculateOrbitalRadius("inner", dimensions.width, dimensions.height);
  const middleRadius = calculateOrbitalRadius("middle", dimensions.width, dimensions.height);
  const outerRadius = calculateOrbitalRadius("outer", dimensions.width, dimensions.height);

  return (
    <div
      ref={containerRef}
      className={`relative w-full h-full overflow-hidden ${className}`}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {/* Layer 1: Code Rain Background */}
      <CodeRain className="z-0" />

      {/* Layer 2: Ambient Particles */}
      <AmbientParticles className="z-10" />

      {/* Layer 3+: Parallax-affected content */}
      <motion.div
        className="absolute inset-0"
        style={{
          x: parallaxX,
          y: parallaxY,
        }}
      >
        {/* Orbital path circles */}
        {dimensions.width > 0 && dimensions.height > 0 && (
          <svg
            className="absolute inset-0 z-20 pointer-events-none"
            width={dimensions.width}
            height={dimensions.height}
          >
            <defs>
              {/* Glow filter for orbital paths */}
              <filter id="orbitGlow" x="-50%" y="-50%" width="200%" height="200%">
                <feGaussianBlur stdDeviation="1" result="coloredBlur" />
                <feMerge>
                  <feMergeNode in="coloredBlur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
            </defs>

            {/* Inner orbital path */}
            <circle
              cx={centerX}
              cy={centerY}
              r={innerRadius}
              fill="none"
              stroke="rgba(255, 107, 53, 0.04)"
              strokeWidth="1"
            />
            {/* Middle orbital path */}
            <circle
              cx={centerX}
              cy={centerY}
              r={middleRadius}
              fill="none"
              stroke="rgba(255, 107, 53, 0.03)"
              strokeWidth="1"
            />
            {/* Outer orbital path */}
            <circle
              cx={centerX}
              cy={centerY}
              r={outerRadius}
              fill="none"
              stroke="rgba(255, 107, 53, 0.02)"
              strokeWidth="1"
            />
          </svg>
        )}

        {/* Central Hub */}
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

        {/* Orbiting Agent Nodes */}
        {dimensions.width > 0 &&
          dimensions.height > 0 &&
          ORBITING_AGENTS.map((agent, index) => (
            <OrbitingAgentNode
              key={agent.id}
              agent={agent}
              viewportWidth={dimensions.width}
              viewportHeight={dimensions.height}
              centerX={centerX}
              centerY={centerY}
              size={agent.tier === "inner" ? 50 : agent.tier === "middle" ? 42 : 36}
              index={index}
            />
          ))}
      </motion.div>
    </div>
  );
}
