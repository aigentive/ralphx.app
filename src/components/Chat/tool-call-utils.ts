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
      return { bg: "hsla(200, 70%, 50%, 0.15)", text: "hsl(200, 70%, 65%)" };
    case "plan":
      return { bg: "hsla(280, 60%, 50%, 0.15)", text: "hsl(280, 60%, 70%)" };
    case "bash":
      return { bg: "hsla(140, 60%, 40%, 0.15)", text: "hsl(140, 60%, 65%)" };
    case "general-purpose":
      return { bg: "hsla(220, 60%, 50%, 0.15)", text: "hsl(220, 60%, 70%)" };
    default:
      return { bg: "hsla(220, 10%, 50%, 0.15)", text: "hsl(220, 10%, 65%)" };
  }
}

/** Get model badge color */
export function getModelColor(model: string): { bg: string; text: string } {
  const m = model.toLowerCase();
  if (m.includes("opus")) return { bg: "hsla(14, 100%, 60%, 0.15)", text: "hsl(14, 100%, 65%)" };
  if (m.includes("sonnet")) return { bg: "hsla(40, 80%, 50%, 0.15)", text: "hsl(40, 80%, 65%)" };
  if (m.includes("haiku")) return { bg: "hsla(160, 60%, 45%, 0.15)", text: "hsl(160, 60%, 65%)" };
  return { bg: "hsla(220, 10%, 50%, 0.15)", text: "hsl(220, 10%, 65%)" };
}
