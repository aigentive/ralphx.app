/**
 * Tauri API wrappers for methodology operations
 *
 * Provides type-safe functions for methodology management with Zod validation.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Response Schemas (matching Rust response structures)
// ============================================================================

/**
 * Schema for methodology phase response from Rust backend
 * Note: Uses snake_case to match Rust serde serialization
 */
export const MethodologyPhaseResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  order: z.number().int().nonnegative(),
  description: z.string().nullable(),
  agent_profiles: z.array(z.string()),
  column_ids: z.array(z.string()),
});

export type MethodologyPhaseResponse = z.infer<typeof MethodologyPhaseResponseSchema>;

/**
 * Schema for methodology template response from Rust backend
 */
export const MethodologyTemplateResponseSchema = z.object({
  artifact_type: z.string(),
  template_path: z.string(),
  name: z.string().nullable(),
  description: z.string().nullable(),
});

export type MethodologyTemplateResponse = z.infer<typeof MethodologyTemplateResponseSchema>;

/**
 * Schema for methodology response from Rust backend
 */
export const MethodologyResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().nullable(),
  agent_profiles: z.array(z.string()),
  skills: z.array(z.string()),
  workflow_id: z.string(),
  workflow_name: z.string(),
  phases: z.array(MethodologyPhaseResponseSchema),
  templates: z.array(MethodologyTemplateResponseSchema),
  is_active: z.boolean(),
  phase_count: z.number().int().nonnegative(),
  agent_count: z.number().int().nonnegative(),
  created_at: z.string(),
});

export type MethodologyResponse = z.infer<typeof MethodologyResponseSchema>;

/**
 * Schema for workflow in activation response
 */
export const WorkflowSchemaResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().nullable(),
  column_count: z.number().int().nonnegative(),
});

export type WorkflowSchemaResponse = z.infer<typeof WorkflowSchemaResponseSchema>;

/**
 * Schema for methodology activation response from Rust backend
 */
export const MethodologyActivationResponseSchema = z.object({
  methodology: MethodologyResponseSchema,
  workflow: WorkflowSchemaResponseSchema,
  agent_profiles: z.array(z.string()),
  skills: z.array(z.string()),
  previous_methodology_id: z.string().nullable(),
});

export type MethodologyActivationResponse = z.infer<typeof MethodologyActivationResponseSchema>;

/**
 * Schema for array of methodology responses
 */
const MethodologyListResponseSchema = z.array(MethodologyResponseSchema);

// ============================================================================
// API Functions
// ============================================================================

/**
 * Get all methodologies
 * @returns Array of methodology responses
 */
export async function getMethodologies(): Promise<MethodologyResponse[]> {
  const result = await invoke("get_methodologies", {});
  return MethodologyListResponseSchema.parse(result);
}

/**
 * Get the currently active methodology (if any)
 * @returns The active methodology or null if none is active
 */
export async function getActiveMethodology(): Promise<MethodologyResponse | null> {
  const result = await invoke("get_active_methodology", {});
  return MethodologyResponseSchema.nullable().parse(result);
}

/**
 * Activate a methodology by ID, deactivating any currently active one
 * @param id The methodology ID to activate
 * @returns The activation response including workflow and agent info
 */
export async function activateMethodology(
  id: string
): Promise<MethodologyActivationResponse> {
  const result = await invoke("activate_methodology", { id });
  return MethodologyActivationResponseSchema.parse(result);
}

/**
 * Deactivate a methodology by ID
 * @param id The methodology ID to deactivate
 * @returns The deactivated methodology
 */
export async function deactivateMethodology(
  id: string
): Promise<MethodologyResponse> {
  const result = await invoke("deactivate_methodology", { id });
  return MethodologyResponseSchema.parse(result);
}
