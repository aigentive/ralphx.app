/**
 * AgentNode - Individual agent node with glow and hover effects
 *
 * Displays an agent node in the constellation with:
 * - Icon + label display using Lucide icons
 * - Breathing glow animation (scale + box-shadow pulse)
 * - Dramatic hover effect (scale 1.25 + intense glow)
 * - Spring physics on hover transitions
 *
 * Uses Framer Motion for smooth animations.
 *
 * Anti-AI-Slop: Each agent has its own color, warm orange #ff6b35 for orchestrator
 */

import { motion } from "framer-motion";
import type { LucideIcon } from "lucide-react";

export interface AgentConfig {
  id: string;
  name: string;
  role: string;
  icon: LucideIcon;
  color: string;
  position: { x: number; y: number };
}

interface AgentNodeProps {
  agent: AgentConfig;
  className?: string;
  /** Size of the node in pixels */
  size?: number;
  /** Index for staggered entrance animation */
  index?: number;
}

export default function AgentNode({
  agent,
  className = "",
  size = 80,
  index = 0,
}: AgentNodeProps) {
  const { name, role, icon: Icon, color } = agent;
  const iconSize = size * 0.4;

  // Create rgba version of the color for shadows
  const colorToRgba = (hex: string, alpha: number) => {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  };

  return (
    <motion.div
      className={`relative flex flex-col items-center ${className}`}
      // Entrance animation - fly in with spring physics
      initial={{ opacity: 0, scale: 0 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{
        type: "spring",
        stiffness: 100,
        damping: 12,
        delay: index * 0.15,
      }}
    >
      {/* Node container with hover effects */}
      <motion.div
        className="relative flex items-center justify-center rounded-full cursor-pointer"
        style={{
          width: size,
          height: size,
          background: `radial-gradient(circle, ${colorToRgba(color, 0.2)} 0%, ${colorToRgba(color, 0.05)} 70%, transparent 100%)`,
          boxShadow: `0 0 20px ${colorToRgba(color, 0.3)}, 0 0 40px ${colorToRgba(color, 0.15)}`,
        }}
        // Hover effect with spring physics
        whileHover={{
          scale: 1.25,
          boxShadow: `0 0 50px ${colorToRgba(color, 0.7)}, 0 0 80px ${colorToRgba(color, 0.4)}, 0 0 120px ${colorToRgba(color, 0.2)}`,
        }}
        whileTap={{ scale: 1.15 }}
        transition={{
          type: "spring",
          stiffness: 400,
          damping: 15,
        }}
      >
        {/* Breathing glow ring - pulsing scale animation */}
        <motion.div
          className="absolute inset-0 rounded-full"
          style={{
            border: `2px solid ${colorToRgba(color, 0.4)}`,
            boxShadow: `0 0 20px ${colorToRgba(color, 0.3)}`,
          }}
          animate={{
            scale: [1, 1.08, 1],
            boxShadow: [
              `0 0 20px ${colorToRgba(color, 0.3)}`,
              `0 0 35px ${colorToRgba(color, 0.5)}`,
              `0 0 20px ${colorToRgba(color, 0.3)}`,
            ],
          }}
          transition={{
            duration: 2.5,
            repeat: Infinity,
            ease: "easeInOut",
            delay: index * 0.3,
          }}
        />

        {/* Inner glow layer */}
        <motion.div
          className="absolute rounded-full"
          style={{
            width: size * 0.7,
            height: size * 0.7,
            background: `radial-gradient(circle, ${colorToRgba(color, 0.4)} 0%, transparent 70%)`,
          }}
          animate={{
            opacity: [0.5, 0.8, 0.5],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: "easeInOut",
          }}
        />

        {/* Icon container */}
        <motion.div
          className="relative z-10 flex items-center justify-center rounded-full"
          style={{
            width: size * 0.55,
            height: size * 0.55,
            background: `linear-gradient(135deg, ${colorToRgba(color, 0.3)} 0%, ${colorToRgba(color, 0.1)} 100%)`,
            backdropFilter: "blur(8px)",
            border: `1px solid ${colorToRgba(color, 0.5)}`,
          }}
        >
          <Icon
            size={iconSize}
            color={color}
            strokeWidth={1.5}
            style={{
              filter: `drop-shadow(0 0 8px ${colorToRgba(color, 0.6)})`,
            }}
          />
        </motion.div>
      </motion.div>

      {/* Label - name and role */}
      <motion.div
        className="mt-3 text-center"
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{
          delay: index * 0.15 + 0.3,
          duration: 0.4,
        }}
      >
        <div
          className="text-sm font-semibold"
          style={{
            color,
            textShadow: `0 0 10px ${colorToRgba(color, 0.5)}`,
          }}
        >
          {name}
        </div>
        <div className="text-xs text-white/50 mt-0.5">{role}</div>
      </motion.div>
    </motion.div>
  );
}
