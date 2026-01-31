/**
 * Agent Configuration for Welcome Screen Constellation
 *
 * Real agents from ralphx-plugin/agents/ with orbital parameters
 * for the animated constellation.
 *
 * Agents are organized into three orbital tiers:
 * - Inner (4 agents): Core workflow - orchestrator, worker, reviewer, qa
 * - Middle (4 agents): Support - supervisor, researcher, ideation, qa-prep
 * - Outer (5 agents): Utilities - chat agents, session-namer, dependency
 *
 * Anti-AI-Slop: Warm colors only, no purple/blue
 */

import {
  Brain,
  Code2,
  Eye,
  ShieldCheck,
  Activity,
  Search,
  Lightbulb,
  ClipboardList,
  MessageSquare,
  FolderKanban,
  MessagesSquare,
  Tag,
  GitBranch,
  type LucideIcon,
} from "lucide-react";

export interface OrbitingAgent {
  id: string;
  name: string;
  role: string;
  icon: LucideIcon;
  color: string;
  /** Orbital tier: inner (0.25), middle (0.4), outer (0.55) as fraction of min dimension */
  tier: "inner" | "middle" | "outer";
  /** Starting angle in radians */
  startAngle: number;
  /** Orbital period in seconds (full rotation) */
  period: number;
  /** Direction: 1 for clockwise, -1 for counter-clockwise */
  direction: 1 | -1;
}

/**
 * Orbital radius as fraction of the smaller viewport dimension
 * Adjusted to keep agents within bounds with safe margins
 */
export const ORBITAL_TIERS = {
  inner: 0.28,
  middle: 0.38,
  outer: 0.46,
} as const;

/** Safe margin from edges (percentage of dimension) */
export const EDGE_MARGIN = 0.04;

/** Minimum orbital radius in pixels (for very small screens) */
export const MIN_ORBITAL_RADIUS = 80;

/** Maximum orbital radius in pixels (for very large screens) */
export const MAX_ORBITAL_RADIUS = 600;

/**
 * All 13 agents from the RalphX plugin
 * Colors use warm palette: orange, green, amber, rose, teal, lime, yellow
 */
export const ORBITING_AGENTS: OrbitingAgent[] = [
  // Inner tier (4 agents) - Core workflow
  {
    id: "orchestrator",
    name: "Orchestrator",
    role: "Plans & coordinates",
    icon: Brain,
    color: "#ff6b35", // Warm orange (accent)
    tier: "inner",
    startAngle: 0,
    period: 45,
    direction: 1,
  },
  {
    id: "worker",
    name: "Worker",
    role: "Writes code",
    icon: Code2,
    color: "#4ade80", // Green
    tier: "inner",
    startAngle: Math.PI / 2,
    period: 45,
    direction: 1,
  },
  {
    id: "reviewer",
    name: "Reviewer",
    role: "Reviews changes",
    icon: Eye,
    color: "#f59e0b", // Amber
    tier: "inner",
    startAngle: Math.PI,
    period: 45,
    direction: 1,
  },
  {
    id: "qa-executor",
    name: "QA",
    role: "Tests the code",
    icon: ShieldCheck,
    color: "#22d3ee", // Cyan/teal (not blue)
    tier: "inner",
    startAngle: (3 * Math.PI) / 2,
    period: 45,
    direction: 1,
  },

  // Middle tier (4 agents) - Support
  {
    id: "supervisor",
    name: "Supervisor",
    role: "Monitors execution",
    icon: Activity,
    color: "#f43f5e", // Rose
    tier: "middle",
    startAngle: Math.PI / 4,
    period: 60,
    direction: -1,
  },
  {
    id: "deep-researcher",
    name: "Researcher",
    role: "Deep analysis",
    icon: Search,
    color: "#a3e635", // Lime
    tier: "middle",
    startAngle: (3 * Math.PI) / 4,
    period: 60,
    direction: -1,
  },
  {
    id: "orchestrator-ideation",
    name: "Ideation",
    role: "Brainstorms ideas",
    icon: Lightbulb,
    color: "#facc15", // Yellow
    tier: "middle",
    startAngle: (5 * Math.PI) / 4,
    period: 60,
    direction: -1,
  },
  {
    id: "qa-prep",
    name: "QA Prep",
    role: "Plans tests",
    icon: ClipboardList,
    color: "#fb923c", // Orange-400
    tier: "middle",
    startAngle: (7 * Math.PI) / 4,
    period: 60,
    direction: -1,
  },

  // Outer tier (5 agents) - Utilities
  {
    id: "chat-task",
    name: "Task Chat",
    role: "Task assistant",
    icon: MessageSquare,
    color: "#34d399", // Emerald
    tier: "outer",
    startAngle: 0,
    period: 80,
    direction: 1,
  },
  {
    id: "chat-project",
    name: "Project Chat",
    role: "Project assistant",
    icon: FolderKanban,
    color: "#fbbf24", // Amber-400
    tier: "outer",
    startAngle: (2 * Math.PI) / 5,
    period: 80,
    direction: 1,
  },
  {
    id: "review-chat",
    name: "Review Chat",
    role: "Discusses reviews",
    icon: MessagesSquare,
    color: "#f97316", // Orange-500
    tier: "outer",
    startAngle: (4 * Math.PI) / 5,
    period: 80,
    direction: 1,
  },
  {
    id: "session-namer",
    name: "Namer",
    role: "Names sessions",
    icon: Tag,
    color: "#a78bfa", // Violet (allowed, not blue)
    tier: "outer",
    startAngle: (6 * Math.PI) / 5,
    period: 80,
    direction: 1,
  },
  {
    id: "dependency-suggester",
    name: "Dependencies",
    role: "Links tasks",
    icon: GitBranch,
    color: "#2dd4bf", // Teal
    tier: "outer",
    startAngle: (8 * Math.PI) / 5,
    period: 80,
    direction: 1,
  },
];

/**
 * Calculate orbital radius based on viewport dimensions
 * Ensures agents stay within bounds with proper margins
 */
export function calculateOrbitalRadius(
  tier: "inner" | "middle" | "outer",
  viewportWidth: number,
  viewportHeight: number
): number {
  // Use the smaller dimension to ensure agents fit
  const minDimension = Math.min(viewportWidth, viewportHeight);

  // Apply margin to available space
  const availableSpace = minDimension * (1 - 2 * EDGE_MARGIN);

  // Calculate radius based on tier
  const tierFraction = ORBITAL_TIERS[tier];
  const radius = availableSpace * tierFraction;

  // Clamp to min/max bounds
  return Math.max(MIN_ORBITAL_RADIUS, Math.min(MAX_ORBITAL_RADIUS, radius));
}

/**
 * Calculate agent position at a given time
 * @param agent - The agent configuration
 * @param timeSeconds - Current time in seconds
 * @param centerX - Center X coordinate
 * @param centerY - Center Y coordinate
 * @param radius - Orbital radius for this tier
 * @returns {x, y} position
 */
export function calculateAgentPosition(
  agent: OrbitingAgent,
  timeSeconds: number,
  centerX: number,
  centerY: number,
  radius: number
): { x: number; y: number } {
  // Calculate current angle based on time and period
  const angularVelocity = (2 * Math.PI) / agent.period;
  const currentAngle =
    agent.startAngle + agent.direction * angularVelocity * timeSeconds;

  // Calculate position on the orbit
  const x = centerX + radius * Math.cos(currentAngle);
  const y = centerY + radius * Math.sin(currentAngle);

  return { x, y };
}
