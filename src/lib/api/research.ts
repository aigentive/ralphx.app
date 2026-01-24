/**
 * Tauri API wrappers for research process operations
 *
 * Provides type-safe functions for research process lifecycle management with Zod validation.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { ResearchDepthPresetSchema, ResearchProcessStatusSchema } from "@/types/research";

// ============================================================================
// Response Schemas (matching Rust response structures)
// ============================================================================

/**
 * Schema for research process response from Rust backend
 * Note: Uses snake_case to match Rust serde serialization
 */
export const ResearchProcessResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  question: z.string(),
  context: z.string().nullable(),
  scope: z.string().nullable(),
  constraints: z.array(z.string()),
  agent_profile_id: z.string(),
  depth_preset: ResearchDepthPresetSchema.nullable(),
  max_iterations: z.number().int().positive(),
  timeout_hours: z.number().positive(),
  checkpoint_interval: z.number().int().positive(),
  target_bucket: z.string(),
  status: ResearchProcessStatusSchema,
  current_iteration: z.number().int().nonnegative(),
  progress_percentage: z.number().nonnegative(),
  error_message: z.string().nullable(),
  created_at: z.string(),
  started_at: z.string().nullable(),
  completed_at: z.string().nullable(),
});

export type ResearchProcessResponse = z.infer<typeof ResearchProcessResponseSchema>;

/**
 * Schema for research depth preset response from Rust backend
 */
export const ResearchPresetResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  max_iterations: z.number().int().positive(),
  timeout_hours: z.number().positive(),
  checkpoint_interval: z.number().int().positive(),
  description: z.string(),
});

export type ResearchPresetResponse = z.infer<typeof ResearchPresetResponseSchema>;

/**
 * Schema for array of research process responses
 */
const ResearchProcessListResponseSchema = z.array(ResearchProcessResponseSchema);

/**
 * Schema for array of preset responses
 */
const PresetListResponseSchema = z.array(ResearchPresetResponseSchema);

// ============================================================================
// Input Schemas (for validating client-side input before sending)
// ============================================================================

/**
 * Schema for custom depth input
 */
export const CustomDepthInputSchema = z.object({
  max_iterations: z.number().int().positive(),
  timeout_hours: z.number().positive(),
  checkpoint_interval: z.number().int().positive(),
});

export type CustomDepthInput = z.infer<typeof CustomDepthInputSchema>;

/**
 * Schema for starting a new research process
 */
export const StartResearchInputSchema = z.object({
  name: z.string().min(1),
  question: z.string().min(1),
  context: z.string().optional(),
  scope: z.string().optional(),
  constraints: z.array(z.string()).optional(),
  agent_profile_id: z.string().min(1),
  depth_preset: ResearchDepthPresetSchema.optional(),
  custom_depth: CustomDepthInputSchema.optional(),
  target_bucket: z.string().optional(),
});

export type StartResearchInput = z.infer<typeof StartResearchInputSchema>;

// ============================================================================
// API Functions
// ============================================================================

/**
 * Start a new research process
 * @param input Research process configuration
 * @returns The started research process
 * @throws ZodError if input validation fails
 */
export async function startResearch(
  input: StartResearchInput
): Promise<ResearchProcessResponse> {
  // Validate input before sending
  const validatedInput = StartResearchInputSchema.parse(input);
  const result = await invoke("start_research", { input: validatedInput });
  return ResearchProcessResponseSchema.parse(result);
}

/**
 * Pause a running research process
 * @param id The research process ID
 * @returns The paused research process
 */
export async function pauseResearch(id: string): Promise<ResearchProcessResponse> {
  const result = await invoke("pause_research", { id });
  return ResearchProcessResponseSchema.parse(result);
}

/**
 * Resume a paused research process
 * @param id The research process ID
 * @returns The resumed research process
 */
export async function resumeResearch(id: string): Promise<ResearchProcessResponse> {
  const result = await invoke("resume_research", { id });
  return ResearchProcessResponseSchema.parse(result);
}

/**
 * Stop/cancel a research process
 * @param id The research process ID
 * @returns The stopped research process (status: failed)
 */
export async function stopResearch(id: string): Promise<ResearchProcessResponse> {
  const result = await invoke("stop_research", { id });
  return ResearchProcessResponseSchema.parse(result);
}

/**
 * Get all research processes, optionally filtered by status
 * @param status Optional status filter
 * @returns Array of research process responses
 */
export async function getResearchProcesses(
  status?: string
): Promise<ResearchProcessResponse[]> {
  const result = await invoke("get_research_processes", { status: status ?? null });
  return ResearchProcessListResponseSchema.parse(result);
}

/**
 * Get a single research process by ID
 * @param id The research process ID
 * @returns The research process or null if not found
 */
export async function getResearchProcess(
  id: string
): Promise<ResearchProcessResponse | null> {
  const result = await invoke("get_research_process", { id });
  return ResearchProcessResponseSchema.nullable().parse(result);
}

/**
 * Get available research depth presets
 * @returns Array of preset configurations
 */
export async function getResearchPresets(): Promise<ResearchPresetResponse[]> {
  const result = await invoke("get_research_presets", {});
  return PresetListResponseSchema.parse(result);
}
