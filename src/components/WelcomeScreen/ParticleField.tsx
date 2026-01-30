/**
 * ParticleField - CSS-only particle effect suggesting AI orchestration
 *
 * Creates a field of floating particles that drift slowly across the screen,
 * giving the impression of neural network activity or AI coordination.
 *
 * Uses seeded pseudo-random for deterministic rendering (React purity compliance).
 * Limited to 25 particles for 60fps performance.
 *
 * Anti-AI-Slop: Warm orange #ff6b35 and white particles, no purple/blue
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

/** Particle count - limited for performance (60fps target) */
const PARTICLE_COUNT = 25;

/**
 * Pre-computed particle data for deterministic rendering
 * Using seeded random to generate visually interesting but stable positions
 */
const PARTICLE_DATA = Array.from({ length: PARTICLE_COUNT }, (_, i) => {
  const r1 = seededRandom(i * 7 + 1);
  const r2 = seededRandom(i * 7 + 2);
  const r3 = seededRandom(i * 7 + 3);
  const r4 = seededRandom(i * 7 + 4);
  const r5 = seededRandom(i * 7 + 5);
  const r6 = seededRandom(i * 7 + 6);
  const r7 = seededRandom(i * 7 + 7);
  const r8 = seededRandom(i * 7 + 8);
  return {
    id: i,
    left: `${r1 * 100}%`,
    top: `${r2 * 100}%`,
    delay: `${r3 * 8}s`,
    duration: `${8 + r4 * 6}s`,
    size: r5 > 0.7 ? 3 : 2,
    isOrange: r6 > 0.6,
    driftX: `${(r7 - 0.5) * 100}px`,
    driftY: `${(r8 - 0.5) * 100}px`,
  };
});

interface ParticleFieldProps {
  className?: string;
}

export default function ParticleField({ className = "" }: ParticleFieldProps) {
  return (
    <div className={`absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {PARTICLE_DATA.map((p) => (
        <div
          key={p.id}
          className="absolute rounded-full particle"
          style={{
            left: p.left,
            top: p.top,
            width: p.size,
            height: p.size,
            backgroundColor: p.isOrange
              ? "var(--accent-primary)"
              : "var(--text-secondary)",
            opacity: 0,
            animationDuration: p.duration,
            animationDelay: p.delay,
            // CSS custom properties for drift direction
            ["--drift-x" as string]: p.driftX,
            ["--drift-y" as string]: p.driftY,
          }}
        />
      ))}
    </div>
  );
}
