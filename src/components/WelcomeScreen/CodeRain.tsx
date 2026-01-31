/**
 * CodeRain - Drifting code fragments background effect
 *
 * Creates a dense field of 40-50 code fragments drifting downward with:
 * - Parallax depth effect (large/near vs small/far)
 * - Varied speeds for depth perception
 * - Occasional orange highlight on random fragments
 *
 * Uses seeded pseudo-random for deterministic rendering (React purity compliance).
 * CSS keyframe animations for performance.
 *
 * Anti-AI-Slop: Warm orange #ff6b35 highlights only, no purple/blue
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

/** Code snippets that suggest AI orchestration */
const CODE_SNIPPETS = [
  "agent.spawn('worker')",
  "await orchestrate()",
  "task.complete()",
  "review.approve()",
  "{ status: 'executing' }",
  "pipeline.next()",
  "commit.push()",
  "qa.validate()",
  "async fn run()",
  "spawn_agent()",
  "task.start()",
  "review.queue()",
  "build.success()",
  "test.pass()",
  "deploy.ready()",
];

/** Fragment count for dense background */
const FRAGMENT_COUNT = 45;

/**
 * Pre-computed fragment data for deterministic rendering
 * Uses prime-based shuffling to ensure nearby fragments have very different timings.
 */
const FRAGMENT_DATA = Array.from({ length: FRAGMENT_COUNT }, (_, i) => {
  // Use different prime multipliers to decouple properties
  const rDepth = seededRandom(i * 7 + 1);
  const rLeft = seededRandom(i * 13 + 100);
  const rSpeed = seededRandom(i * 17 + 200);
  const rStart = seededRandom(i * 23 + 300);
  const rDelay = seededRandom(i * 31 + 400);
  const rText = seededRandom(i * 37 + 500);
  const rDrift = seededRandom(i * 41 + 600);

  // Depth layer: 0 = far (small, slow), 1 = mid, 2 = near (large, fast)
  const depthLayer = rDepth < 0.3 ? 0 : rDepth < 0.7 ? 1 : 2;

  // Size and opacity based on depth (reduced for subtlety)
  const fontSize = depthLayer === 0 ? 10 : depthLayer === 1 ? 12 : 14;
  const opacity = depthLayer === 0 ? 0.08 : depthLayer === 1 ? 0.12 : 0.18;

  // Speed - much more varied within each layer
  const baseSpeed = depthLayer === 0 ? 25 : depthLayer === 1 ? 16 : 10;
  const speedVariation = depthLayer === 0 ? 15 : depthLayer === 1 ? 10 : 6;
  const duration = baseSpeed + rSpeed * speedVariation;

  // Horizontal position - use golden ratio for better distribution
  const goldenRatio = 0.618033988749;
  const left = ((i * goldenRatio * 100) % 100);

  // Horizontal drift amount
  const driftX = (rDrift - 0.5) * 50;

  // Delay - spread across a long period, completely independent of position
  const delay = rDelay * 30; // 0-30 seconds

  // Start offset - varied
  const startOffset = -10 - rStart * 90; // -10% to -100%

  return {
    id: i,
    left: `${left}%`,
    startOffset: `${startOffset}%`,
    delay,
    duration,
    fontSize,
    opacity,
    driftX,
    text: CODE_SNIPPETS[Math.floor(rText * CODE_SNIPPETS.length)],
    isHighlight: rDepth > 0.92,
    depthLayer,
  };
});

interface CodeRainProps {
  className?: string;
}

export default function CodeRain({ className = "" }: CodeRainProps) {
  return (
    <>
      <style>{`
        @keyframes codeDrift {
          0% {
            transform: translateY(var(--start-offset)) translateX(0);
            opacity: 0;
          }
          5% {
            opacity: var(--fragment-opacity);
          }
          95% {
            opacity: var(--fragment-opacity);
          }
          100% {
            transform: translateY(100vh) translateX(var(--drift-x));
            opacity: 0;
          }
        }

        @keyframes highlightPulse {
          0%, 100% {
            text-shadow: 0 0 4px rgba(255, 107, 53, 0.4);
          }
          50% {
            text-shadow: 0 0 12px rgba(255, 107, 53, 0.8), 0 0 20px rgba(255, 107, 53, 0.4);
          }
        }

        .code-fragment {
          position: absolute;
          font-family: "SF Mono", "Fira Code", "Consolas", monospace;
          white-space: nowrap;
          pointer-events: none;
          will-change: transform, opacity;
          animation: codeDrift var(--duration) linear infinite;
          animation-delay: var(--delay);
        }

        .code-fragment.highlight {
          color: #ff6b35;
          animation: codeDrift var(--duration) linear infinite,
                     highlightPulse 2s ease-in-out infinite;
        }
      `}</style>
      <div
        className={`absolute inset-0 overflow-hidden pointer-events-none ${className}`}
        aria-hidden="true"
      >
        {FRAGMENT_DATA.map((f) => (
          <div
            key={f.id}
            className={`code-fragment ${f.isHighlight ? "highlight" : ""}`}
            style={{
              left: f.left,
              top: 0,
              fontSize: f.fontSize,
              color: f.isHighlight ? "#ff6b35" : "rgba(255, 255, 255, 0.25)",
              opacity: 0, // Start hidden, animation will control visibility
              transform: `translateY(${f.startOffset}) translateX(0)`, // Match animation start
              ["--start-offset" as string]: f.startOffset,
              ["--drift-x" as string]: `${f.driftX}px`,
              ["--fragment-opacity" as string]: f.opacity,
              ["--duration" as string]: `${f.duration}s`,
              ["--delay" as string]: `${f.delay}s`,
              zIndex: f.depthLayer,
            }}
          >
            {f.text}
          </div>
        ))}
      </div>
    </>
  );
}
