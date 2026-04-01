/**
 * Research type definitions for the extensibility system
 *
 * Support for long-running research agents with configurable depth presets.
 * Research processes have a lifecycle: pending -> running -> (paused) -> completed/failed
 */

import { z } from "zod";
import { ArtifactTypeSchema } from "./artifact";

// ============================================
// Research Depth Presets
// ============================================

/**
 * Predefined research depth presets
 * - quick-scan: 10 iterations, 30 min - Fast overview
 * - standard: 50 iterations, 2 hrs - Thorough investigation
 * - deep-dive: 200 iterations, 8 hrs - Comprehensive analysis
 * - exhaustive: 500 iterations, 24 hrs - Leave no stone unturned
 */
export const ResearchDepthPresetSchema = z.enum([
  "quick-scan",
  "standard",
  "deep-dive",
  "exhaustive",
]);

export type ResearchDepthPreset = z.infer<typeof ResearchDepthPresetSchema>;

/**
 * All research depth preset values as a readonly array
 */
export const RESEARCH_DEPTH_PRESET_VALUES = ResearchDepthPresetSchema.options;

// ============================================
// Custom Depth
// ============================================

/**
 * Custom depth configuration for fine-grained control
 */
export const CustomDepthSchema = z.object({
  /** Maximum number of iterations before stopping */
  maxIterations: z.number().int().positive(),
  /** Maximum time in hours before stopping */
  timeoutHours: z.number().positive(),
  /** Save checkpoint every N iterations */
  checkpointInterval: z.number().int().positive(),
});

export type CustomDepth = z.infer<typeof CustomDepthSchema>;

/**
 * Preset configurations as CustomDepth values
 */
export const RESEARCH_PRESETS: Record<ResearchDepthPreset, CustomDepth> = {
  "quick-scan": { maxIterations: 10, timeoutHours: 0.5, checkpointInterval: 5 },
  standard: { maxIterations: 50, timeoutHours: 2, checkpointInterval: 10 },
  "deep-dive": { maxIterations: 200, timeoutHours: 8, checkpointInterval: 25 },
  exhaustive: { maxIterations: 500, timeoutHours: 24, checkpointInterval: 50 },
};

/**
 * Get the CustomDepth configuration for a preset
 */
export function getPresetConfig(preset: ResearchDepthPreset): CustomDepth {
  return RESEARCH_PRESETS[preset];
}

// ============================================
// Research Depth (Preset or Custom)
// ============================================

/**
 * Schema for preset depth - just the preset name string
 */
export const ResearchDepthPresetVariantSchema = z.object({
  type: z.literal("preset"),
  preset: ResearchDepthPresetSchema,
});

export type ResearchDepthPresetVariant = z.infer<
  typeof ResearchDepthPresetVariantSchema
>;

/**
 * Schema for custom depth - full configuration
 */
export const ResearchDepthCustomVariantSchema = z.object({
  type: z.literal("custom"),
  config: CustomDepthSchema,
});

export type ResearchDepthCustomVariant = z.infer<
  typeof ResearchDepthCustomVariantSchema
>;

/**
 * Research depth - either a preset or custom configuration
 */
export const ResearchDepthSchema = z.discriminatedUnion("type", [
  ResearchDepthPresetVariantSchema,
  ResearchDepthCustomVariantSchema,
]);

export type ResearchDepth = z.infer<typeof ResearchDepthSchema>;

/**
 * Create a preset depth
 */
export function createPresetDepth(
  preset: ResearchDepthPreset
): ResearchDepthPresetVariant {
  return { type: "preset", preset };
}

/**
 * Create a custom depth
 */
export function createCustomDepth(
  config: CustomDepth
): ResearchDepthCustomVariant {
  return { type: "custom", config };
}

/**
 * Resolve a depth to its CustomDepth configuration
 */
export function resolveDepth(depth: ResearchDepth): CustomDepth {
  if (depth.type === "preset") {
    return getPresetConfig(depth.preset);
  }
  return depth.config;
}

/**
 * Check if a depth is a preset
 */
export function isPresetDepth(
  depth: ResearchDepth
): depth is ResearchDepthPresetVariant {
  return depth.type === "preset";
}

/**
 * Check if a depth is a custom configuration
 */
export function isCustomDepth(
  depth: ResearchDepth
): depth is ResearchDepthCustomVariant {
  return depth.type === "custom";
}

// ============================================
// Research Process Status
// ============================================

/**
 * Status of a research process
 * - pending: Not yet started
 * - running: Currently executing
 * - paused: Temporarily paused
 * - completed: Successfully completed
 * - failed: Failed with error
 */
export const ResearchProcessStatusSchema = z.enum([
  "pending",
  "running",
  "paused",
  "completed",
  "failed",
]);

export type ResearchProcessStatus = z.infer<typeof ResearchProcessStatusSchema>;

/**
 * All research process status values as a readonly array
 */
export const RESEARCH_PROCESS_STATUS_VALUES =
  ResearchProcessStatusSchema.options;

/**
 * Active statuses (not yet terminal)
 */
export const ACTIVE_RESEARCH_STATUSES: readonly ResearchProcessStatus[] = [
  "pending",
  "running",
] as const;

/**
 * Terminal statuses (completed or failed)
 */
export const TERMINAL_RESEARCH_STATUSES: readonly ResearchProcessStatus[] = [
  "completed",
  "failed",
] as const;

/**
 * Check if a status is active (pending or running)
 */
export function isActiveResearchStatus(status: ResearchProcessStatus): boolean {
  return (ACTIVE_RESEARCH_STATUSES as readonly string[]).includes(status);
}

/**
 * Check if a status is terminal (completed or failed)
 */
export function isTerminalResearchStatus(
  status: ResearchProcessStatus
): boolean {
  return (TERMINAL_RESEARCH_STATUSES as readonly string[]).includes(status);
}

/**
 * Check if a status is paused
 */
export function isPausedResearchStatus(status: ResearchProcessStatus): boolean {
  return status === "paused";
}

// ============================================
// Research Brief
// ============================================

/**
 * The research brief - the question and context for research
 */
export const ResearchBriefSchema = z.object({
  /** The main question to research */
  question: z.string().min(1),
  /** Optional additional context */
  context: z.string().optional(),
  /** Optional scope limitations */
  scope: z.string().optional(),
  /** Optional constraints on the research */
  constraints: z.array(z.string()).default([]),
});

export type ResearchBrief = z.infer<typeof ResearchBriefSchema>;

/**
 * Create a simple research brief with just a question
 */
export function createResearchBrief(question: string): ResearchBrief {
  return { question, constraints: [] };
}

/**
 * Create a full research brief with all fields
 */
export function createFullResearchBrief(
  question: string,
  context?: string,
  scope?: string,
  constraints?: string[]
): ResearchBrief {
  return {
    question,
    context,
    scope,
    constraints: constraints ?? [],
  };
}

// ============================================
// Research Output
// ============================================

/**
 * Configuration for research process output
 */
export const ResearchOutputSchema = z.object({
  /** The bucket to store output artifacts in */
  targetBucket: z.string(),
  /** The types of artifacts this research produces */
  artifactTypes: z.array(ArtifactTypeSchema).default([]),
});

export type ResearchOutput = z.infer<typeof ResearchOutputSchema>;

/**
 * Default research output configuration
 */
export const DEFAULT_RESEARCH_OUTPUT: ResearchOutput = {
  targetBucket: "research-outputs",
  artifactTypes: ["research_document", "findings", "recommendations"],
};

/**
 * Create a research output configuration
 */
export function createResearchOutput(
  targetBucket: string,
  artifactTypes?: string[]
): ResearchOutput {
  return {
    targetBucket,
    artifactTypes: (artifactTypes ?? []) as ResearchOutput["artifactTypes"],
  };
}

// ============================================
// Research Progress
// ============================================

/**
 * Progress tracking for a research process
 */
export const ResearchProgressSchema = z.object({
  /** Current iteration number */
  currentIteration: z.number().int().nonnegative().default(0),
  /** Current status */
  status: ResearchProcessStatusSchema.default("pending"),
  /** ID of the last checkpoint artifact (if any) */
  lastCheckpoint: z.string().optional(),
  /** Error message if failed */
  errorMessage: z.string().optional(),
});

export type ResearchProgress = z.infer<typeof ResearchProgressSchema>;

/**
 * Create initial research progress
 */
export function createResearchProgress(): ResearchProgress {
  return {
    currentIteration: 0,
    status: "pending",
  };
}

/**
 * Calculate progress percentage
 */
export function calculateProgressPercentage(
  currentIteration: number,
  maxIterations: number
): number {
  if (maxIterations === 0) return 0;
  return Math.min((currentIteration / maxIterations) * 100, 100);
}

/**
 * Check if a checkpoint should be saved at the current iteration
 */
export function shouldCheckpoint(
  currentIteration: number,
  checkpointInterval: number
): boolean {
  if (checkpointInterval === 0 || currentIteration === 0) return false;
  return currentIteration % checkpointInterval === 0;
}

// ============================================
// Research Process
// ============================================

/**
 * A research process - a long-running research agent with configurable depth
 */
export const ResearchProcessSchema = z.object({
  /** Unique identifier */
  id: z.string(),
  /** Display name */
  name: z.string(),
  /** The research brief (question, context, scope, constraints) */
  brief: ResearchBriefSchema,
  /** Depth configuration (preset or custom) */
  depth: ResearchDepthSchema,
  /** Agent profile ID to use for this research */
  agentProfileId: z.string(),
  /** Output configuration */
  output: ResearchOutputSchema,
  /** Progress tracking */
  progress: ResearchProgressSchema,
  /** When the process was created (ISO 8601 string) */
  createdAt: z.string(),
  /** When the process was started (if started) */
  startedAt: z.string().optional(),
  /** When the process was completed (if completed) */
  completedAt: z.string().optional(),
});

export type ResearchProcess = z.infer<typeof ResearchProcessSchema>;

/**
 * Input for creating a new research process
 */
export const CreateResearchProcessInputSchema = z.object({
  /** Display name */
  name: z.string(),
  /** The research brief */
  brief: ResearchBriefSchema,
  /** Depth configuration */
  depth: ResearchDepthSchema.optional(),
  /** Agent profile ID */
  agentProfileId: z.string(),
  /** Output configuration */
  output: ResearchOutputSchema.optional(),
});

export type CreateResearchProcessInput = z.infer<
  typeof CreateResearchProcessInputSchema
>;

/**
 * Get the resolved depth configuration for a research process
 */
export function getResolvedDepth(process: ResearchProcess): CustomDepth {
  return resolveDepth(process.depth);
}

/**
 * Get the progress percentage for a research process
 */
export function getProcessProgressPercentage(process: ResearchProcess): number {
  const maxIterations = getResolvedDepth(process).maxIterations;
  return calculateProgressPercentage(
    process.progress.currentIteration,
    maxIterations
  );
}

/**
 * Check if a research process should checkpoint at its current iteration
 */
export function processShouldCheckpoint(process: ResearchProcess): boolean {
  const interval = getResolvedDepth(process).checkpointInterval;
  return shouldCheckpoint(process.progress.currentIteration, interval);
}

/**
 * Check if a research process has reached max iterations
 */
export function isMaxIterationsReached(process: ResearchProcess): boolean {
  const maxIterations = getResolvedDepth(process).maxIterations;
  return process.progress.currentIteration >= maxIterations;
}

/**
 * Check if a research process is active
 */
export function isProcessActive(process: ResearchProcess): boolean {
  return isActiveResearchStatus(process.progress.status);
}

/**
 * Check if a research process is terminal
 */
export function isProcessTerminal(process: ResearchProcess): boolean {
  return isTerminalResearchStatus(process.progress.status);
}

/**
 * Check if a research process is paused
 */
export function isProcessPaused(process: ResearchProcess): boolean {
  return isPausedResearchStatus(process.progress.status);
}

// ============================================
// Research Preset Info (for UI display)
// ============================================

/**
 * Display information for a research preset
 */
export const ResearchPresetInfoSchema = z.object({
  /** The preset identifier */
  preset: ResearchDepthPresetSchema,
  /** Human-readable name */
  name: z.string(),
  /** Description of the preset */
  description: z.string(),
  /** The configuration values */
  config: CustomDepthSchema,
});

export type ResearchPresetInfo = z.infer<typeof ResearchPresetInfoSchema>;

/**
 * All research preset info for UI display
 */
export const RESEARCH_PRESET_INFO: readonly ResearchPresetInfo[] = [
  {
    preset: "quick-scan",
    name: "Quick Scan",
    description: "Fast overview - 10 iterations, 30 min timeout",
    config: RESEARCH_PRESETS["quick-scan"],
  },
  {
    preset: "standard",
    name: "Standard",
    description: "Thorough investigation - 50 iterations, 2 hrs timeout",
    config: RESEARCH_PRESETS["standard"],
  },
  {
    preset: "deep-dive",
    name: "Deep Dive",
    description: "Comprehensive analysis - 200 iterations, 8 hrs timeout",
    config: RESEARCH_PRESETS["deep-dive"],
  },
  {
    preset: "exhaustive",
    name: "Exhaustive",
    description: "Leave no stone unturned - 500 iterations, 24 hrs timeout",
    config: RESEARCH_PRESETS["exhaustive"],
  },
] as const;

/**
 * Get preset info by preset identifier
 */
export function getPresetInfo(
  preset: ResearchDepthPreset
): ResearchPresetInfo | undefined {
  return RESEARCH_PRESET_INFO.find((info) => info.preset === preset);
}

// ============================================
// Parsing Helpers
// ============================================

/**
 * Parse and validate a ResearchProcess
 */
export function parseResearchProcess(data: unknown): ResearchProcess {
  return ResearchProcessSchema.parse(data);
}

/**
 * Safely parse a ResearchProcess, returning null on failure
 */
export function safeParseResearchProcess(data: unknown): ResearchProcess | null {
  const result = ResearchProcessSchema.safeParse(data);
  return result.success ? result.data : null;
}

/**
 * Parse and validate a ResearchBrief
 */
export function parseResearchBrief(data: unknown): ResearchBrief {
  return ResearchBriefSchema.parse(data);
}

/**
 * Safely parse a ResearchBrief, returning null on failure
 */
export function safeParseResearchBrief(data: unknown): ResearchBrief | null {
  const result = ResearchBriefSchema.safeParse(data);
  return result.success ? result.data : null;
}

/**
 * Parse and validate a ResearchDepth
 */
export function parseResearchDepth(data: unknown): ResearchDepth {
  return ResearchDepthSchema.parse(data);
}

/**
 * Safely parse a ResearchDepth, returning null on failure
 */
export function safeParseResearchDepth(data: unknown): ResearchDepth | null {
  const result = ResearchDepthSchema.safeParse(data);
  return result.success ? result.data : null;
}
