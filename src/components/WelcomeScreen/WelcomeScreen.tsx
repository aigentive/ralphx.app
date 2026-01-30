/**
 * WelcomeScreen - Impressive animated welcome screen for first-run experience
 *
 * "Terminal Symphony" design: sophisticated dark terminal environment with
 * floating code fragments, particle effects, and warm orange accents.
 *
 * Anti-AI-Slop: No purple/blue gradients, warm orange #ff6b35, SF Pro typography
 */

import { Sparkles } from "lucide-react";

interface WelcomeScreenProps {
  onCreateProject: () => void;
}

/**
 * TerminalCanvas - Visual terminal element with floating code fragments
 * Placeholder structure for Task 2 implementation
 */
function TerminalCanvas() {
  return (
    <div
      className="relative w-full max-w-2xl mx-auto rounded-xl overflow-hidden"
      style={{
        backgroundColor: "var(--bg-surface)",
        border: "1px solid var(--border-subtle)",
        boxShadow: "var(--shadow-lg)",
      }}
    >
      {/* Terminal header with traffic lights */}
      <div
        className="flex items-center gap-2 px-4 py-3"
        style={{
          backgroundColor: "var(--bg-elevated)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-[#ff5f57]" />
          <div className="w-3 h-3 rounded-full bg-[#ffbd2e]" />
          <div className="w-3 h-3 rounded-full bg-[#28c840]" />
        </div>
        <span
          className="ml-2 text-xs font-medium"
          style={{
            color: "var(--text-muted)",
            fontFamily: "var(--font-mono)",
          }}
        >
          ralphx ~ orchestrator
        </span>
      </div>

      {/* Terminal body with typing animation */}
      <div
        className="p-6 min-h-[200px] relative"
        style={{ fontFamily: "var(--font-mono)" }}
      >
        {/* Typing line with cursor */}
        <div className="flex items-center gap-2 mb-4">
          <span style={{ color: "#28c840" }}>$</span>
          <span style={{ color: "var(--text-primary)" }}>
            ralphx init --agent orchestrator
          </span>
          <span
            className="inline-block w-2 h-5 ml-1"
            style={{
              backgroundColor: "var(--accent-primary)",
              animation: "terminalBlink 1s step-end infinite",
            }}
          />
        </div>

        {/* Code output lines */}
        <div
          className="space-y-2 text-sm"
          style={{ color: "var(--text-secondary)" }}
        >
          <p>
            <span style={{ color: "var(--text-muted)" }}>// </span>
            <span style={{ color: "#6a9955" }}>
              Initializing autonomous development environment...
            </span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>agent</span>
            <span style={{ color: "var(--text-muted)" }}>.</span>
            <span style={{ color: "#dcdcaa" }}>spawn</span>
            <span style={{ color: "var(--text-muted)" }}>(</span>
            <span style={{ color: "#ce9178" }}>'worker'</span>
            <span style={{ color: "var(--text-muted)" }}>)</span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>agent</span>
            <span style={{ color: "var(--text-muted)" }}>.</span>
            <span style={{ color: "#dcdcaa" }}>spawn</span>
            <span style={{ color: "var(--text-muted)" }}>(</span>
            <span style={{ color: "#ce9178" }}>'reviewer'</span>
            <span style={{ color: "var(--text-muted)" }}>)</span>
          </p>
          <p>
            <span style={{ color: "var(--accent-primary)" }}>await</span>
            <span style={{ color: "var(--text-primary)" }}> </span>
            <span style={{ color: "#dcdcaa" }}>orchestrate</span>
            <span style={{ color: "var(--text-muted)" }}>()</span>
          </p>
        </div>

        {/* Floating code fragments (Task 2 will add more) */}
        <div
          className="absolute top-4 right-4 text-xs px-2 py-1 rounded opacity-60"
          style={{
            backgroundColor: "rgba(255, 107, 53, 0.1)",
            color: "var(--accent-primary)",
            animation: "codeFloat 4s ease-in-out infinite",
          }}
        >
          {'{ status: "ready" }'}
        </div>
      </div>
    </div>
  );
}

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

/**
 * Pre-computed particle data for deterministic rendering
 * Using seeded random to generate visually interesting but stable positions
 */
const PARTICLE_DATA = Array.from({ length: 25 }, (_, i) => {
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

/**
 * ParticleField - CSS-only particle effect suggesting AI orchestration
 * Placeholder structure for Task 2 implementation
 */
function ParticleField() {
  // Use pre-computed particle data for React purity compliance
  const particles = PARTICLE_DATA;

  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none">
      {particles.map((p) => (
        <div
          key={p.id}
          className="absolute rounded-full"
          style={{
            left: p.left,
            top: p.top,
            width: p.size,
            height: p.size,
            backgroundColor: p.isOrange
              ? "var(--accent-primary)"
              : "var(--text-secondary)",
            opacity: 0,
            animation: `particleDrift ${p.duration} ease-in-out infinite`,
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

export default function WelcomeScreen({ onCreateProject }: WelcomeScreenProps) {
  return (
    <div
      className="flex-1 flex flex-col items-center justify-center relative overflow-hidden"
      style={{ backgroundColor: "var(--bg-base)" }}
      data-testid="welcome-screen"
    >
      {/* Particle field background */}
      <ParticleField />

      {/* Gradient overlay for depth */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background:
            "radial-gradient(ellipse at center, transparent 0%, var(--bg-base) 70%)",
        }}
      />

      {/* Content container */}
      <div className="relative z-10 flex flex-col items-center px-8 max-w-4xl w-full">
        {/* Hero section */}
        <div
          className="text-center mb-12"
          style={{ animation: "fadeSlideIn 0.6s ease-out forwards" }}
        >
          {/* RalphX title with subtle glow */}
          <h1
            className="text-6xl font-bold tracking-tight mb-4"
            style={{
              fontFamily: "var(--font-display)",
              color: "var(--text-primary)",
              textShadow: "0 0 40px rgba(255, 107, 53, 0.15)",
            }}
          >
            Ralph
            <span style={{ color: "var(--accent-primary)" }}>X</span>
          </h1>

          {/* Tagline */}
          <p
            className="text-xl font-light"
            style={{
              fontFamily: "var(--font-body)",
              color: "var(--text-secondary)",
              letterSpacing: "var(--tracking-wide)",
            }}
          >
            Autonomous AI Development, Orchestrated
          </p>
        </div>

        {/* Terminal canvas visual */}
        <div
          className="w-full mb-12"
          style={{
            animation: "fadeSlideIn 0.6s ease-out 0.15s forwards",
            opacity: 0,
          }}
        >
          <TerminalCanvas />
        </div>

        {/* CTA section */}
        <div
          className="flex flex-col items-center gap-4"
          style={{
            animation: "fadeSlideIn 0.6s ease-out 0.3s forwards",
            opacity: 0,
          }}
        >
          {/* Primary CTA button with glow */}
          <button
            onClick={onCreateProject}
            className="group flex items-center gap-3 px-8 py-4 rounded-xl text-lg font-semibold transition-all duration-300 hover:scale-[1.02] active:scale-[0.98]"
            style={{
              backgroundColor: "var(--accent-primary)",
              color: "#fff",
              fontFamily: "var(--font-body)",
              boxShadow:
                "0 0 20px rgba(255, 107, 53, 0.3), 0 0 40px rgba(255, 107, 53, 0.1)",
              animation: "glowPulse 3s ease-in-out infinite",
            }}
            data-testid="create-first-project-button"
          >
            <Sparkles className="w-5 h-5 transition-transform group-hover:rotate-12" />
            Create Your First Project
          </button>

          {/* Keyboard shortcut hints */}
          <p
            className="text-sm"
            style={{
              color: "var(--text-muted)",
              fontFamily: "var(--font-body)",
            }}
          >
            Press{" "}
            <kbd
              className="px-2 py-0.5 rounded text-xs font-medium"
              style={{
                backgroundColor: "var(--bg-elevated)",
                color: "var(--text-secondary)",
                border: "1px solid var(--border-default)",
              }}
            >
              ⌘N
            </kbd>{" "}
            to create a project
          </p>
        </div>
      </div>

      {/* CSS animations */}
      <style>{`
        /* Cursor blink animation */
        @keyframes terminalBlink {
          0%, 100% { opacity: 1; }
          50% { opacity: 0; }
        }

        /* Staggered fade-in animation for content sections */
        @keyframes fadeSlideIn {
          from {
            opacity: 0;
            transform: translateY(20px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        /* Floating code fragment animation */
        @keyframes codeFloat {
          0%, 100% {
            transform: translateY(0) rotate(0deg);
            opacity: 0.6;
          }
          50% {
            transform: translateY(-10px) rotate(2deg);
            opacity: 0.8;
          }
        }

        /* Particle drift animation */
        @keyframes particleDrift {
          0% {
            transform: translate(0, 0);
            opacity: 0;
          }
          10% {
            opacity: 0.6;
          }
          90% {
            opacity: 0.6;
          }
          100% {
            transform: translate(var(--drift-x), var(--drift-y));
            opacity: 0;
          }
        }

        /* Button glow pulse animation */
        @keyframes glowPulse {
          0%, 100% {
            box-shadow: 0 0 20px rgba(255, 107, 53, 0.3), 0 0 40px rgba(255, 107, 53, 0.1);
          }
          50% {
            box-shadow: 0 0 30px rgba(255, 107, 53, 0.5), 0 0 60px rgba(255, 107, 53, 0.2);
          }
        }
      `}</style>
    </div>
  );
}
