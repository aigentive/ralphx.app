/**
 * Shared utilities for TaskToolCallCard and TaskSubagentCard.
 */

/** Format milliseconds into human-readable duration */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const remainSecs = secs % 60;
  return `${mins}m ${remainSecs}s`;
}

/** Get a display-friendly color for the subagent type badge */
export function getSubagentTypeColor(subagentType: string): { bg: string; text: string } {
  const type = subagentType.toLowerCase();
  switch (type) {
    case "explore":
      return {
        bg: "color-mix(in srgb, var(--status-info) 15%, transparent)",
        text: "var(--status-info)",
      };
    case "plan":
      return {
        bg: "color-mix(in srgb, var(--accent-primary) 15%, transparent)",
        text: "var(--accent-primary)",
      };
    case "bash":
      return {
        bg: "color-mix(in srgb, var(--status-success) 15%, transparent)",
        text: "var(--status-success)",
      };
    case "general-purpose":
      return {
        bg: "color-mix(in srgb, var(--status-info) 15%, transparent)",
        text: "var(--status-info)",
      };
    default:
      return {
        bg: "color-mix(in srgb, var(--text-muted) 15%, transparent)",
        text: "var(--text-secondary)",
      };
  }
}

/** Get model badge color */
export function getModelColor(model: string): { bg: string; text: string } {
  const m = model.toLowerCase();
  if (m.includes("opus"))
    return {
      bg: "color-mix(in srgb, var(--accent-primary) 15%, transparent)",
      text: "var(--accent-primary)",
    };
  if (m.includes("sonnet"))
    return {
      bg: "color-mix(in srgb, var(--status-warning) 15%, transparent)",
      text: "var(--status-warning)",
    };
  if (m.includes("haiku"))
    return {
      bg: "color-mix(in srgb, var(--status-success) 15%, transparent)",
      text: "var(--status-success)",
    };
  return {
    bg: "color-mix(in srgb, var(--text-muted) 15%, transparent)",
    text: "var(--text-secondary)",
  };
}
