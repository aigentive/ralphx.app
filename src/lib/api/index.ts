/**
 * Tauri API wrappers
 *
 * Type-safe wrappers for Tauri commands with Zod validation.
 */

// Workflows API
export {
  getWorkflows,
  getWorkflow,
  createWorkflow,
  updateWorkflow,
  deleteWorkflow,
  setDefaultWorkflow,
  getActiveWorkflowColumns,
  getBuiltinWorkflows,
  WorkflowResponseSchema,
  WorkflowColumnResponseSchema,
  WorkflowColumnInputSchema,
  CreateWorkflowInputSchema,
  UpdateWorkflowInputSchema,
  type WorkflowResponse,
  type WorkflowColumnResponse,
  type WorkflowColumnInput,
  type CreateWorkflowInput,
  type UpdateWorkflowInput,
} from "./workflows";
