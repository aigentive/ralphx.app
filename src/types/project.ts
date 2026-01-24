// Project types and Zod schema
// Must match the Rust backend Project struct

import { z } from "zod";

/**
 * Git mode for project repository
 */
export const GitModeSchema = z.enum(["local", "worktree"]);
export type GitMode = z.infer<typeof GitModeSchema>;

/**
 * Project schema matching Rust backend serialization
 * Note: field names use camelCase as that's what serde_json produces with rename_all
 */
export const ProjectSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  workingDirectory: z.string().min(1),
  gitMode: GitModeSchema,
  worktreePath: z.string().nullable(),
  worktreeBranch: z.string().nullable(),
  baseBranch: z.string().nullable(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type Project = z.infer<typeof ProjectSchema>;

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
