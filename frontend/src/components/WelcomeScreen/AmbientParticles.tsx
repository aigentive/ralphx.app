/**
 * AmbientParticles - Floating dots background effect
 *
 * Creates 30-40 tiny particles drifting randomly with:
 * - Varied sizes (2px to 6px)
 * - Color palette: white, orange accent (#ff6b35), agent colors at low opacity
 * - Smooth random movement with gentle drifting
 *
 * Uses seeded pseudo-random for deterministic rendering (React purity compliance).
 * CSS keyframe animations for performance.
 *
 * Anti-AI-Slop: Uses project accent colors only, no purple gradients
 */

/**
 * Simple seeded pseudo-random number generator (mulberry32)
 * Produces deterministic values for a given seed, avoiding React purity issues
 */
function seededRandom(seed: number): number {
  let t = (seed + 0x6d2b79f5) | 0;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

/** Color palette: white, orange accent, and agent colors at low opacity */
const PARTICLE_COLORS = [
  "rgba(255, 255, 255, 0.4)", // white
  "rgba(255, 255, 255, 0.3)", // white dimmer
  "rgba(255, 107, 53, 0.5)", // orange accent #ff6b35
  "rgba(255, 107, 53, 0.3)", // orange accent dimmer
  "rgba(74, 222, 128, 0.3)", // worker green #4ade80
  "rgba(96, 165, 250, 0.3)", // qa blue #60a5fa
  "rgba(245, 158, 11, 0.3)", // reviewer amber #f59e0b
];

/** Particle count for ambient background */
const PARTICLE_COUNT = 35;

/**
 * Pre-computed particle data for deterministic rendering
 * Uses seeded random to generate visually interesting but stable positions
 */
const PARTICLE_DATA = Array.from({ length: PARTICLE_COUNT }, (_, i) => {
  const r1 = seededRandom(i * 13 + 100);
  const r2 = seededRandom(i * 13 + 101);
  const r3 = seededRandom(i * 13 + 102);
  const r4 = seededRandom(i * 13 + 103);
  const r5 = seededRandom(i * 13 + 104);
  const r6 = seededRandom(i * 13 + 105);
  const r7 = seededRandom(i * 13 + 106);
  const r8 = seededRandom(i * 13 + 107);

  // Size: 2px to 6px
  const size = 2 + r1 * 4;

  // Random drift direction and distance
  const driftXRange = 30 + r7 * 50; // 30-80px horizontal drift
  const driftYRange = 20 + r8 * 40; // 20-60px vertical drift

  // Duration: slower = more ambient feeling (15-35s)
  const duration = 15 + r4 * 20;

  return {
    id: i,
    left: `${r2 * 100}%`,
    top: `${r3 * 100}%`,
    size,
    color: PARTICLE_COLORS[Math.floor(r5 * PARTICLE_COLORS.length)],
    duration,
    delay: r6 * 10,
    driftXRange,
    driftYRange,
    // Direction of drift (alternating for visual variety)
    driftDirection: r7 > 0.5 ? 1 : -1,
  };
});

interface AmbientParticlesProps {
  className?: string;
}

export default function AmbientParticles({
  className = "",
}: AmbientParticlesProps) {
  return (
    <>
      <style>{`
        @keyframes particleDrift {
          0%, 100% {
            transform: translate(0, 0);
            opacity: var(--particle-opacity);
          }
          25% {
            transform: translate(
              calc(var(--drift-x) * var(--direction)),
              calc(var(--drift-y) * -0.5)
            );
            opacity: calc(var(--particle-opacity) * 1.2);
          }
          50% {
            transform: translate(
              calc(var(--drift-x) * var(--direction) * 0.3),
              calc(var(--drift-y) * var(--direction))
            );
            opacity: var(--particle-opacity);
          }
          75% {
            transform: translate(
              calc(var(--drift-x) * var(--direction) * -0.5),
              calc(var(--drift-y) * 0.3)
            );
            opacity: calc(var(--particle-opacity) * 0.8);
          }
        }

        @keyframes particleGlow {
          0%, 100% {
            box-shadow: 0 0 var(--glow-size) var(--particle-color);
          }
          50% {
            box-shadow: 0 0 calc(var(--glow-size) * 2) var(--particle-color);
          }
        }

        .ambient-particle {
          position: absolute;
          border-radius: 50%;
          pointer-events: none;
          will-change: transform, opacity;
          animation:
            particleDrift var(--duration) ease-in-out infinite,
            particleGlow calc(var(--duration) * 0.5) ease-in-out infinite;
          animation-delay: var(--delay);
        }
      `}</style>
      <div
        className={`absolute inset-0 overflow-hidden pointer-events-none ${className}`}
        aria-hidden="true"
      >
        {PARTICLE_DATA.map((p) => (
          <div
            key={p.id}
            className="ambient-particle"
            style={{
              left: p.left,
              top: p.top,
              width: p.size,
              height: p.size,
              backgroundColor: p.color,
              ["--particle-color" as string]: p.color,
              ["--particle-opacity" as string]: 1,
              ["--drift-x" as string]: `${p.driftXRange}px`,
              ["--drift-y" as string]: `${p.driftYRange}px`,
              ["--direction" as string]: p.driftDirection,
              ["--duration" as string]: `${p.duration}s`,
              ["--delay" as string]: `${p.delay}s`,
              ["--glow-size" as string]: `${p.size}px`,
            }}
          />
        ))}
      </div>
    </>
  );
}
