/**
 * IdeationView Constants
 *
 * Shared design tokens and configuration for ideation components.
 */

import type { Priority } from "@/types/ideation";

/**
 * Priority styling configuration
 * Used by ProposalCard and IdeationView for consistent priority appearance
 */
export const PRIORITY_CONFIG: Record<Priority, { gradient: string; glow: string; label: string }> = {
  critical: {
    gradient: "from-red-500/20 to-red-600/10",
    glow: "shadow-[0_0_12px_rgba(239,68,68,0.1)]",
    label: "Critical"
  },
  high: {
    gradient: "from-[#ff6b35]/20 to-[#ff6b35]/10",
    glow: "shadow-[0_0_12px_rgba(255,107,53,0.1)]",
    label: "High"
  },
  medium: {
    gradient: "from-amber-500/15 to-amber-600/5",
    glow: "",
    label: "Medium"
  },
  low: {
    gradient: "from-slate-500/10 to-slate-600/5",
    glow: "",
    label: "Low"
  },
};

/**
 * CSS animation styles for ideation components
 * Injected via <style> tag for keyframe animations
 */
export const animationStyles = `
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

@keyframes subtlePulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes glowPulse {
  0%, 100% {
    box-shadow: 0 0 12px rgba(255,107,53,0.08),
                0 0 24px rgba(255,107,53,0.04),
                inset 0 1px 0 rgba(255,255,255,0.05);
  }
  50% {
    box-shadow: 0 0 18px rgba(255,107,53,0.15),
                0 0 36px rgba(255,107,53,0.08),
                inset 0 1px 0 rgba(255,255,255,0.08);
  }
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}
.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

.session-card-enter {
  animation: fadeSlideIn 0.3s ease-out forwards;
}

.active-session-glow {
  animation: glowPulse 3s ease-in-out infinite;
}

.shimmer-loading {
  background: linear-gradient(
    90deg,
    rgba(255,255,255,0) 0%,
    rgba(255,255,255,0.05) 50%,
    rgba(255,255,255,0) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 2s infinite;
}
`;
