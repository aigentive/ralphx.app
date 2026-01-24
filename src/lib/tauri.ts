// Tauri invoke wrappers with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  TaskSchema,
  TaskListSchema,
  type CreateTask,
  type UpdateTask,
} from "@/types/task";
import {
  ProjectSchema,
  type CreateProject,
  type UpdateProject,
} from "@/types/project";

/**
 * Generic invoke wrapper with runtime Zod validation
 * @param cmd The Tauri command name
 * @param args The arguments to pass to the command
 * @param schema The Zod schema to validate the response
 * @returns The validated response
 * @throws If the response doesn't match the schema
 */
export async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

/**
 * Health check response schema
 */
export const HealthResponseSchema = z.object({
  status: z.string(),
});

export type HealthResponse = z.infer<typeof HealthResponseSchema>;

/**
 * Project list schema for array responses
 */
const ProjectListSchema = z.array(ProjectSchema);

/**
 * API object containing all typed Tauri command wrappers
 */
export const api = {
  health: {
    /**
     * Check if the backend is running
     * @returns { status: "ok" } if healthy
     */
    check: () => typedInvoke("health_check", {}, HealthResponseSchema),
  },

  tasks: {
    /**
     * List all tasks for a project
     * @param projectId The project ID
     * @returns Array of tasks
     */
    list: (projectId: string) =>
      typedInvoke("list_tasks", { projectId }, TaskListSchema),

    /**
     * Get a single task by ID
     * @param taskId The task ID
     * @returns The task
     */
    get: (taskId: string) => typedInvoke("get_task", { taskId }, TaskSchema),

    /**
     * Create a new task
     * @param input Task creation data
     * @returns The created task
     */
    create: (input: CreateTask) =>
      typedInvoke("create_task", { input }, TaskSchema),

    /**
     * Update an existing task
     * @param taskId The task ID
     * @param input Partial task data to update
     * @returns The updated task
     */
    update: (taskId: string, input: UpdateTask) =>
      typedInvoke("update_task", { taskId, input }, TaskSchema),

    /**
     * Delete a task
     * @param taskId The task ID
     * @returns true if deleted
     */
    delete: (taskId: string) =>
      typedInvoke("delete_task", { taskId }, z.boolean()),

    /**
     * Move a task to a new status
     * @param taskId The task ID
     * @param toStatus The target status
     * @returns The updated task
     */
    move: (taskId: string, toStatus: string) =>
      typedInvoke("move_task", { taskId, toStatus }, TaskSchema),
  },

  projects: {
    /**
     * List all projects
     * @returns Array of projects
     */
    list: () => typedInvoke("list_projects", {}, ProjectListSchema),

    /**
     * Get a single project by ID
     * @param projectId The project ID
     * @returns The project
     */
    get: (projectId: string) =>
      typedInvoke("get_project", { projectId }, ProjectSchema),

    /**
     * Create a new project
     * @param input Project creation data
     * @returns The created project
     */
    create: (input: CreateProject) =>
      typedInvoke("create_project", { input }, ProjectSchema),

    /**
     * Update an existing project
     * @param projectId The project ID
     * @param input Partial project data to update
     * @returns The updated project
     */
    update: (projectId: string, input: UpdateProject) =>
      typedInvoke("update_project", { projectId, input }, ProjectSchema),

    /**
     * Delete a project
     * @param projectId The project ID
     * @returns true if deleted
     */
    delete: (projectId: string) =>
      typedInvoke("delete_project", { projectId }, z.boolean()),
  },
} as const;
