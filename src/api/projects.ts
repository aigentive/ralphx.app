// Projects and Workflows API module
// Extracted from src/lib/tauri.ts following the domain API pattern

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  ProjectResponseSchema,
  transformProject,
  type CreateProject,
  type UpdateProject,
  type Project,
} from "@/types/project";
import {
  WorkflowResponseSchema,
  transformWorkflow,
  type WorkflowSchema,
} from "@/types/workflow";
import { typedInvoke, typedInvokeWithTransform } from "@/lib/tauri";

/**
 * Project list schema for array responses (snake_case from backend)
 */
const ProjectListResponseSchema = z.array(ProjectResponseSchema);

/**
 * Transform project list from snake_case to camelCase
 */
function transformProjectList(
  response: z.infer<typeof ProjectListResponseSchema>
): Project[] {
  return response.map(transformProject);
}

/**
 * Workflow list schema for array responses
 */
const WorkflowListResponseSchema = z.array(WorkflowResponseSchema);

/**
 * Get git branches for a working directory
 * @param workingDirectory The path to the git repository
 * @returns Array of branch names (main/master sorted first)
 */
export async function getGitBranches(workingDirectory: string): Promise<string[]> {
  const result = await invoke<string[]>("get_git_branches", { workingDirectory });
  return result;
}

/**
 * Projects API object containing all typed Tauri command wrappers for projects
 */
export const projectsApi = {
  /**
   * List all projects
   * @returns Array of projects
   */
  list: () =>
    typedInvokeWithTransform(
      "list_projects",
      {},
      ProjectListResponseSchema,
      transformProjectList
    ),

  /**
   * Get a single project by ID
   * @param projectId The project ID
   * @returns The project
   */
  get: (projectId: string) =>
    typedInvokeWithTransform(
      "get_project",
      { projectId },
      ProjectResponseSchema,
      transformProject
    ),

  /**
   * Create a new project
   * @param input Project creation data
   * @returns The created project
   */
  create: (input: CreateProject) =>
    typedInvokeWithTransform(
      "create_project",
      { input },
      ProjectResponseSchema,
      transformProject
    ),

  /**
   * Update an existing project
   * @param projectId The project ID
   * @param input Partial project data to update
   * @returns The updated project
   */
  update: (projectId: string, input: UpdateProject) =>
    typedInvokeWithTransform(
      "update_project",
      { projectId, input },
      ProjectResponseSchema,
      transformProject
    ),

  /**
   * Delete a project
   * @param projectId The project ID
   * @returns true if deleted
   */
  delete: (projectId: string) =>
    typedInvoke("delete_project", { projectId }, z.boolean()),
} as const;

/**
 * Workflows API object containing all typed Tauri command wrappers for workflows
 */
export const workflowsApi = {
  /**
   * Get a workflow by ID
   * @param workflowId The workflow ID
   * @returns The workflow
   */
  get: async (workflowId: string): Promise<WorkflowSchema> => {
    const raw = await typedInvoke(
      "get_workflow",
      { id: workflowId },
      WorkflowResponseSchema.nullable()
    );
    if (!raw) throw new Error(`Workflow not found: ${workflowId}`);
    return transformWorkflow(raw);
  },

  /**
   * List all workflows
   * @returns Array of workflows
   */
  list: (): Promise<WorkflowSchema[]> =>
    typedInvokeWithTransform(
      "list_workflows",
      {},
      WorkflowListResponseSchema,
      (workflows) => workflows.map(transformWorkflow)
    ),

  /**
   * Seed builtin workflows if they don't exist
   * @returns Number of workflows created
   */
  seedBuiltin: () => typedInvoke("seed_builtin_workflows", {}, z.number()),
} as const;
