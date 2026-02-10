/**
 * Shared constants, types, and utilities for tool call widgets.
 *
 * Extracted from shared.tsx to keep component files under 500 LOC.
 * Design reference: mockups/tool-call-widgets.html
 */

import type { ReactNode } from "react";

// ============================================================================
// Tool Call Type (canonical definition — re-exported from ToolCallIndicator)
// ============================================================================

/**
 * Tool call structure from Claude CLI stream-json output
 */
export interface ToolCall {
  /** Unique identifier for this tool call */
  id: string;
  /** Name of the tool that was called */
  name: string;
  /** Arguments passed to the tool (can be any JSON value) */
  arguments: unknown;
  /** Result returned from the tool (can be any JSON value) */
  result?: unknown;
  /** Error message if tool call failed */
  error?: string;
  /** Diff context for Edit/Write tool calls (old file content for computing diffs) */
  diffContext?: {
    oldContent?: string;
    filePath: string;
  };
}

// ============================================================================
// CSS Constants (shared across all widgets)
// ============================================================================

export const COLLAPSED_HEIGHT = 73;
export const COLLAPSED_HEIGHT_COMPACT = 52;
export const GRADIENT_HEIGHT = 36;
export const TRANSITION_SPEED = "200ms";

export const colors = {
  bgBase: "hsl(220 10% 8%)",
  bgSurface: "hsl(220 10% 12%)",
  bgElevated: "hsl(220 10% 14%)",
  bgHover: "hsl(220 10% 16%)",
  bgTerminal: "hsl(220 10% 10%)",
  textPrimary: "hsl(220 10% 90%)",
  textSecondary: "hsl(220 10% 60%)",
  textMuted: "hsl(220 10% 45%)",
  accent: "hsl(14 100% 60%)",
  accentDim: "hsla(14 100% 60% / 0.10)",
  accentBorder: "hsla(14 100% 60% / 0.30)",
  success: "#34c759",
  successDim: "hsla(145 60% 45% / 0.10)",
  error: "#ff453a",
  errorDim: "hsla(0 70% 55% / 0.10)",
  blue: "hsl(220 60% 50%)",
  blueDim: "hsla(220 60% 50% / 0.12)",
  border: "hsl(220 10% 18%)",
  borderSubtle: "hsl(220 10% 15%)",
} as const;

// ============================================================================
// Badge Variant Type
// ============================================================================

export type BadgeVariant = "muted" | "success" | "accent" | "error" | "blue";

export const badgeStyles: Record<BadgeVariant, { bg: string; color: string }> = {
  muted: { bg: colors.border, color: colors.textMuted },
  success: { bg: colors.successDim, color: colors.success },
  accent: { bg: colors.accentDim, color: colors.accent },
  error: { bg: colors.errorDim, color: colors.error },
  blue: { bg: colors.blueDim, color: colors.blue },
};

// ============================================================================
// Safe extraction helpers (used by all widgets to pull typed fields from unknown args/results)
// ============================================================================

/** Safely extract a string field from an unknown object */
export function getString(obj: unknown, key: string): string | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "string" ? val : undefined;
  }
  return undefined;
}

/** Safely extract a number field from an unknown object */
export function getNumber(obj: unknown, key: string): number | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "number" ? val : undefined;
  }
  return undefined;
}

/** Safely extract a string array field from an unknown object */
export function getStringArray(obj: unknown, key: string): string[] | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    if (Array.isArray(val) && val.every((v) => typeof v === "string")) return val;
  }
  return undefined;
}

/** Safely extract a boolean field from an unknown object */
export function getBool(obj: unknown, key: string): boolean | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "boolean" ? val : undefined;
  }
  return undefined;
}

/** Safely extract an array field from an unknown object */
export function getArray(obj: unknown, key: string): unknown[] | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return Array.isArray(val) ? val : undefined;
  }
  return undefined;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Parse a tool result into an array of non-empty lines.
 * Handles: plain string, MCP wrapper [{text: "..."}], object with text property, and string arrays.
 */
export function parseToolResultAsLines(result: unknown): string[] {
  if (!result) return [];

  let text = "";

  if (typeof result === "string") {
    text = result;
  } else if (Array.isArray(result)) {
    // MCP result wrapper: [{type: "text", text: "..."}]
    const first = result[0];
    if (first && typeof first === "object" && "text" in first) {
      text = String((first as { text: string }).text);
    } else {
      // Array of strings
      return result.filter((item): item is string => typeof item === "string");
    }
  } else if (typeof result === "object" && result !== null && "text" in result) {
    text = String((result as { text: string }).text);
  }

  if (!text) return [];

  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

/** Shorten a file path by collapsing middle directories */
export function shortenPath(path: string, maxLength: number): string {
  if (path.length <= maxLength) return path;

  const parts = path.split("/");
  if (parts.length <= 2) return path;

  // Keep first directory and last 2 segments
  const first = parts[0] || "";
  const last2 = parts.slice(-2).join("/");
  const shortened = `${first}/.../${last2}`;

  if (shortened.length <= maxLength) return shortened;

  // Last resort: just show .../ + filename
  const filename = parts[parts.length - 1] || "";
  return `.../${filename}`;
}

// ============================================================================
// Widget Props (standard interface for all widget components)
// ============================================================================

export interface ToolCallWidgetProps {
  /** The tool call data */
  toolCall: ToolCall;
  /** Compact mode */
  compact?: boolean;
  /** Additional className */
  className?: string;
}

// ============================================================================
// Step Line Types
// ============================================================================

export type StepLineVariant = "started" | "completed" | "added" | "skipped" | "failed";

export interface StepLineConfig {
  color: string;
  label: string;
  badgeVariant: BadgeVariant;
}

export const stepVariantConfig: Record<StepLineVariant, StepLineConfig> = {
  started: { color: colors.accent, label: "started", badgeVariant: "muted" },
  completed: { color: colors.success, label: "completed", badgeVariant: "success" },
  added: { color: colors.blue, label: "added", badgeVariant: "blue" },
  skipped: { color: colors.textMuted, label: "skipped", badgeVariant: "muted" },
  failed: { color: colors.error, label: "failed", badgeVariant: "error" },
};

// ============================================================================
// Component Prop Types (used by shared.tsx components)
// ============================================================================

export interface WidgetCardProps {
  /** Header content (passed to WidgetHeader or custom) */
  header: ReactNode;
  /** Body content (shown when expanded/collapsed with gradient) */
  children: ReactNode;
  /** Start expanded */
  defaultExpanded?: boolean;
  /** Compact mode for nested/subagent rendering */
  compact?: boolean;
  /** Additional className */
  className?: string;
  /** If true, body is always fully visible (no collapse) — for <=3 result items */
  alwaysExpanded?: boolean;
}

export interface WidgetHeaderProps {
  icon?: ReactNode;
  title: string;
  badge?: ReactNode;
  compact?: boolean;
  /** Use monospace font for title (file paths, patterns) */
  mono?: boolean;
}

export interface GradientFadeProps {
  visible: boolean;
  bgColor?: string;
}

export interface CodePreviewProps {
  /** Lines of code to display */
  lines: string[];
  /** Starting line number (default 1) */
  startLine?: number;
  compact?: boolean;
}

export interface InlineIndicatorProps {
  icon?: ReactNode;
  text: string;
}

export interface StepLineProps {
  variant: StepLineVariant;
  title: string;
  note?: string | undefined;
  compact?: boolean;
}

export interface FilePathProps {
  path: string;
  /** Max characters before shortening */
  maxLength?: number;
}

export interface BadgeProps {
  variant: BadgeVariant;
  children: ReactNode;
  compact?: boolean;
}

export interface WidgetRowProps {
  compact?: boolean | undefined;
  children: ReactNode;
}
