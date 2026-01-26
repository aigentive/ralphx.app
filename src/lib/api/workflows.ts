/**
 * Tauri API wrappers for workflow operations
 *
 * Provides type-safe functions for workflow CRUD with Zod validation.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { InternalStatusSchema } from "@/types/status";

// ============================================================================
// Response Schemas (matching Rust WorkflowResponse structures)
// ============================================================================

/**
 * Schema for workflow column response from Rust backend
 * Note: Uses camelCase to match Rust serde serialization (rename_all = "camelCase")
 */
export const WorkflowColumnResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  mapsTo: InternalStatusSchema,
  color: z.string().optional(),
  icon: z.string().optional(),
  skipReview: z.boolean().optional(),
  autoAdvance: z.boolean().optional(),
  agentProfile: z.string().optional(),
});

export type WorkflowColumnResponse = z.infer<typeof WorkflowColumnResponseSchema>;

/**
 * Schema for workflow response from Rust backend
 * Note: Uses camelCase to match Rust serde serialization (rename_all = "camelCase")
 */
export const WorkflowResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().optional(),
  columns: z.array(WorkflowColumnResponseSchema).min(1),
  isDefault: z.boolean(),
  workerProfile: z.string().optional(),
  reviewerProfile: z.string().optional(),
});

export type WorkflowResponse = z.infer<typeof WorkflowResponseSchema>;

/**
 * Schema for array of workflow responses
 */
const WorkflowListResponseSchema = z.array(WorkflowResponseSchema);

/**
 * Schema for array of column responses
 */
const ColumnListResponseSchema = z.array(WorkflowColumnResponseSchema);

// ============================================================================
// Input Schemas (for validating client-side input before sending)
// ============================================================================

/**
 * Schema for column input when creating/updating workflows
 */
export const WorkflowColumnInputSchema = z.object({
  id: z.string(),
  name: z.string(),
  maps_to: InternalStatusSchema,
  color: z.string().optional(),
  icon: z.string().optional(),
  skip_review: z.boolean().optional(),
  auto_advance: z.boolean().optional(),
  agent_profile: z.string().optional(),
});

export type WorkflowColumnInput = z.infer<typeof WorkflowColumnInputSchema>;

/**
 * Schema for creating a new workflow
 */
export const CreateWorkflowInputSchema = z.object({
  name: z.string().min(1),
  description: z.string().optional(),
  columns: z.array(WorkflowColumnInputSchema).min(1),
  is_default: z.boolean().optional(),
  worker_profile: z.string().optional(),
  reviewer_profile: z.string().optional(),
});

export type CreateWorkflowInput = z.infer<typeof CreateWorkflowInputSchema>;

/**
 * Schema for updating an existing workflow (all fields optional)
 */
export const UpdateWorkflowInputSchema = z.object({
  name: z.string().min(1).optional(),
  description: z.string().optional(),
  columns: z.array(WorkflowColumnInputSchema).min(1).optional(),
  is_default: z.boolean().optional(),
  worker_profile: z.string().optional(),
  reviewer_profile: z.string().optional(),
});

export type UpdateWorkflowInput = z.infer<typeof UpdateWorkflowInputSchema>;

// ============================================================================
// API Functions
// ============================================================================

/**
 * List all workflows
 * @returns Array of workflow responses
 */
export async function getWorkflows(): Promise<WorkflowResponse[]> {
  const result = await invoke("get_workflows", {});
  return WorkflowListResponseSchema.parse(result);
}

/**
 * Get a single workflow by ID
 * @param id The workflow ID
 * @returns The workflow or null if not found
 */
export async function getWorkflow(id: string): Promise<WorkflowResponse | null> {
  const result = await invoke("get_workflow", { id });
  return WorkflowResponseSchema.nullable().parse(result);
}

/**
 * Create a new workflow
 * @param input Workflow creation data
 * @returns The created workflow
 * @throws ZodError if input validation fails
 */
export async function createWorkflow(input: CreateWorkflowInput): Promise<WorkflowResponse> {
  // Validate input before sending
  const validatedInput = CreateWorkflowInputSchema.parse(input);
  const result = await invoke("create_workflow", { input: validatedInput });
  return WorkflowResponseSchema.parse(result);
}

/**
 * Update an existing workflow
 * @param id The workflow ID
 * @param input Partial workflow data to update
 * @returns The updated workflow
 * @throws ZodError if input validation fails
 */
export async function updateWorkflow(
  id: string,
  input: UpdateWorkflowInput
): Promise<WorkflowResponse> {
  // Validate input before sending
  const validatedInput = UpdateWorkflowInputSchema.parse(input);
  const result = await invoke("update_workflow", { id, input: validatedInput });
  return WorkflowResponseSchema.parse(result);
}

/**
 * Delete a workflow by ID
 * @param id The workflow ID
 */
export async function deleteWorkflow(id: string): Promise<void> {
  await invoke("delete_workflow", { id });
}

/**
 * Set a workflow as the default
 * @param id The workflow ID to set as default
 * @returns The updated workflow
 */
export async function setDefaultWorkflow(id: string): Promise<WorkflowResponse> {
  const result = await invoke("set_default_workflow", { id });
  return WorkflowResponseSchema.parse(result);
}

/**
 * Get the columns for the currently active/default workflow
 * @returns Array of workflow columns
 */
export async function getActiveWorkflowColumns(): Promise<WorkflowColumnResponse[]> {
  const result = await invoke("get_active_workflow_columns", {});
  return ColumnListResponseSchema.parse(result);
}

/**
 * Get the built-in workflow definitions (RalphX Default, Jira Compatible)
 * @returns Array of built-in workflow responses
 */
export async function getBuiltinWorkflows(): Promise<WorkflowResponse[]> {
  const result = await invoke("get_builtin_workflows", {});
  return WorkflowListResponseSchema.parse(result);
}
