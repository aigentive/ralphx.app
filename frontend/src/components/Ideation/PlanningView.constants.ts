/**
 * PlanningView Constants
 *
 * Shared design tokens and configuration for planning components.
 */

import type { Priority } from "@/types/ideation";

/**
 * Priority styling configuration
 * Used by ProposalCard and PlanningView for consistent priority appearance
 */
export const PRIORITY_CONFIG: Record<Priority, { gradient: string; glow: string; label: string }> = {
  critical: {
    gradient: "from-status-error/20 to-status-error/10",
    glow: "shadow-[0_0_12px_var(--status-error-muted)]",
    label: "Critical"
  },
  high: {
    gradient: "from-accent-primary/20 to-accent-primary/10",
    glow: "shadow-[0_0_12px_var(--accent-muted)]",
    label: "High"
  },
  medium: {
    gradient: "from-status-warning/15 to-status-warning/5",
    glow: "",
    label: "Medium"
  },
  low: {
    gradient: "from-text-muted/10 to-text-muted/5",
    glow: "",
    label: "Low"
  },
};

/**
 * CSS animation styles for planning components
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
    box-shadow: var(--shadow-glow-accent-soft);
  }
  50% {
    box-shadow: var(--shadow-glow-accent-active);
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
