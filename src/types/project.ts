// Project types and Zod schema
// Must match the Rust backend Project struct

import { z } from "zod";

/**
 * Git mode for project repository
 */
export const GitModeSchema = z.enum(["local", "worktree"]);
export type GitMode = z.infer<typeof GitModeSchema>;

/**
 * Backend response schema - expects snake_case from Rust serialization
 */
export const ProjectResponseSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  working_directory: z.string().min(1),
  git_mode: GitModeSchema,
  worktree_path: z.string().nullable(),
  worktree_branch: z.string().nullable(),
  base_branch: z.string().nullable(),
  // Accept RFC3339 timestamps with offset (e.g., +00:00) not just Z
  created_at: z.string().datetime({ offset: true }),
  updated_at: z.string().datetime({ offset: true }),
});

/**
 * Frontend Project interface - uses camelCase
 */
export interface Project {
  id: string;
  name: string;
  workingDirectory: string;
  gitMode: GitMode;
  worktreePath: string | null;
  worktreeBranch: string | null;
  baseBranch: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Transform snake_case backend response to camelCase frontend type
 */
export function transformProject(
  response: z.infer<typeof ProjectResponseSchema>
): Project {
  return {
    id: response.id,
    name: response.name,
    workingDirectory: response.working_directory,
    gitMode: response.git_mode,
    worktreePath: response.worktree_path,
    worktreeBranch: response.worktree_branch,
    baseBranch: response.base_branch,
    createdAt: response.created_at,
    updatedAt: response.updated_at,
  };
}

// Legacy export for backward compatibility - use ProjectResponseSchema instead
export const ProjectSchema = ProjectResponseSchema;

/**
 * Schema for creating a new project
 * Excludes auto-generated fields (id, timestamps)
 */
export const CreateProjectSchema = z.object({
  name: z.string().min(1, "Project name is required"),
  workingDirectory: z.string().min(1, "Working directory is required"),
  gitMode: GitModeSchema.default("local"),
  worktreePath: z.string().optional(),
  worktreeBranch: z.string().optional(),
  baseBranch: z.string().optional(),
});

export type CreateProject = z.infer<typeof CreateProjectSchema>;

/**
 * Schema for updating a project
 * All fields are optional
 */
export const UpdateProjectSchema = z.object({
  name: z.string().min(1).optional(),
  workingDirectory: z.string().min(1).optional(),
  gitMode: GitModeSchema.optional(),
  worktreePath: z.string().nullable().optional(),
  worktreeBranch: z.string().nullable().optional(),
  baseBranch: z.string().nullable().optional(),
});

export type UpdateProject = z.infer<typeof UpdateProjectSchema>;
