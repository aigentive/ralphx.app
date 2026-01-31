/**
 * CentralHub - Pulsing core with ripple rings
 *
 * Creates a glowing command center visual in the center with:
 * - Pulsing warm orange core (#ff6b35)
 * - Concentric animated rings rippling outward (sonar effect)
 * - Multiple ripple layers for depth
 *
 * Uses Framer Motion for scale + opacity animations.
 *
 * Anti-AI-Slop: Warm orange #ff6b35 only, no purple/blue
 */

import { motion } from "framer-motion";

/** Number of ripple rings */
const RIPPLE_COUNT = 3;

/** Ripple ring configuration */
const RIPPLE_DATA = Array.from({ length: RIPPLE_COUNT }, (_, i) => ({
  id: i,
  delay: i * 0.7, // Stagger ripples
  duration: 2.5,
}));

interface CentralHubProps {
  className?: string;
  /** Size of the hub in pixels */
  size?: number;
}

export default function CentralHub({ className = "", size = 80 }: CentralHubProps) {
  const coreSize = size * 0.5; // Core is 50% of total hub size

  return (
    <div
      className={`relative flex items-center justify-center ${className}`}
      style={{ width: size, height: size }}
    >
      {/* Ripple rings - emanating outward */}
      {RIPPLE_DATA.map((ripple) => (
        <motion.div
          key={ripple.id}
          className="absolute rounded-full border"
          style={{
            width: coreSize,
            height: coreSize,
            borderColor: "rgba(255, 107, 53, 0.4)",
            borderWidth: 2,
          }}
          initial={{ scale: 1, opacity: 0.6 }}
          animate={{
            scale: [1, 2.5, 3],
            opacity: [0.6, 0.3, 0],
          }}
          transition={{
            duration: ripple.duration,
            repeat: Infinity,
            delay: ripple.delay,
            ease: "easeOut",
          }}
        />
      ))}

      {/* Outer glow layer */}
      <motion.div
        className="absolute rounded-full"
        style={{
          width: coreSize * 1.5,
          height: coreSize * 1.5,
          background:
            "radial-gradient(circle, rgba(255, 107, 53, 0.3) 0%, rgba(255, 107, 53, 0) 70%)",
        }}
        animate={{
          scale: [1, 1.2, 1],
          opacity: [0.5, 0.8, 0.5],
        }}
        transition={{
          duration: 2,
          repeat: Infinity,
          ease: "easeInOut",
        }}
      />

      {/* Pulsing core */}
      <motion.div
        className="absolute rounded-full"
        style={{
          width: coreSize,
          height: coreSize,
          background:
            "radial-gradient(circle, #ff6b35 0%, rgba(255, 107, 53, 0.8) 50%, rgba(255, 107, 53, 0.4) 100%)",
          boxShadow: "0 0 30px rgba(255, 107, 53, 0.6)",
        }}
        animate={{
          scale: [1, 1.1, 1],
          boxShadow: [
            "0 0 30px rgba(255, 107, 53, 0.6)",
            "0 0 50px rgba(255, 107, 53, 0.8)",
            "0 0 30px rgba(255, 107, 53, 0.6)",
          ],
        }}
        transition={{
          duration: 1.5,
          repeat: Infinity,
          ease: "easeInOut",
        }}
      />

      {/* Inner bright spot */}
      <motion.div
        className="absolute rounded-full"
        style={{
          width: coreSize * 0.4,
          height: coreSize * 0.4,
          background:
            "radial-gradient(circle, rgba(255, 255, 255, 0.9) 0%, rgba(255, 200, 180, 0.6) 50%, transparent 100%)",
        }}
        animate={{
          scale: [1, 1.15, 1],
          opacity: [0.8, 1, 0.8],
        }}
        transition={{
          duration: 1.5,
          repeat: Infinity,
          ease: "easeInOut",
        }}
      />
    </div>
  );
}
