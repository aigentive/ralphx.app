/**
 * WelcomeScreen - Impressive animated welcome screen for first-run experience
 *
 * "Terminal Symphony" design: sophisticated dark terminal environment with
 * floating code fragments, particle effects, and warm orange accents.
 *
 * Anti-AI-Slop: No purple/blue gradients, warm orange #ff6b35, SF Pro typography
 */

import { Sparkles, X } from "lucide-react";
import TerminalCanvas from "./TerminalCanvas";
import ParticleField from "./ParticleField";

interface WelcomeScreenProps {
  onCreateProject: () => void;
  /** Optional callback when closing manually-opened welcome screen (via ⌘⇧W or Escape) */
  onClose?: (() => void) | undefined;
}

export default function WelcomeScreen({ onCreateProject, onClose }: WelcomeScreenProps) {
  return (
    <div
      className="flex-1 flex flex-col items-center justify-center relative overflow-hidden"
      style={{ backgroundColor: "var(--bg-base)" }}
      data-testid="welcome-screen"
    >
      {/* Close button - only shown when manually opened (onClose provided) */}
      {onClose && (
        <button
          onClick={onClose}
          className="absolute top-6 right-6 z-20 p-2 rounded-lg transition-all duration-200 hover:scale-105 active:scale-95"
          style={{
            backgroundColor: "rgba(255, 255, 255, 0.05)",
            border: "1px solid rgba(255, 255, 255, 0.1)",
            color: "var(--text-secondary)",
          }}
          aria-label="Close welcome screen"
          data-testid="close-welcome-screen"
        >
          <X className="w-5 h-5" />
        </button>
      )}

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
          className="text-center mb-12 hero-section"
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
          className="w-full mb-12 terminal-section"
          style={{
            animation: "fadeSlideIn 0.6s ease-out 0.15s forwards",
            opacity: 0,
          }}
        >
          <TerminalCanvas />
        </div>

        {/* CTA section */}
        <div
          className="flex flex-col items-center gap-4 cta-section"
          style={{
            animation: "fadeSlideIn 0.6s ease-out 0.3s forwards",
            opacity: 0,
          }}
        >
          {/* Primary CTA button with glow */}
          <button
            onClick={onCreateProject}
            className="group flex items-center gap-3 px-8 py-4 rounded-xl text-lg font-semibold transition-all duration-300 hover:scale-[1.02] active:scale-[0.98] cta-button"
            style={{
              backgroundColor: "var(--accent-primary)",
              color: "#fff",
              fontFamily: "var(--font-body)",
              boxShadow:
                "0 0 20px rgba(255, 107, 53, 0.3), 0 0 40px rgba(255, 107, 53, 0.1)",
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

      {/* CSS animations - centralized for all WelcomeScreen components */}
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

        /* Apply animations to subcomponents */
        .terminal-cursor {
          animation: terminalBlink 1s step-end infinite;
        }

        .code-fragment {
          animation: codeFloat 4s ease-in-out infinite;
        }

        .particle {
          animation: particleDrift ease-in-out infinite;
        }

        .cta-button {
          animation: glowPulse 3s ease-in-out infinite;
        }
      `}</style>
    </div>
  );
}
