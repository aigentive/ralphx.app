// Task types and Zod schema
// Must match the Rust backend Task struct

import { z } from "zod";
import { InternalStatusSchema } from "./status";
export type { InternalStatus } from "./status";

/**
 * Task schema matching Rust backend serialization
 * Note: field names use camelCase as that's what serde_json produces with rename_all
 */
/**
 * Task schema matching Rust backend serialization
 * Note: field names use camelCase as that's what serde_json produces with rename_all
 */
export const TaskSchema = z.object({
  id: z.string().min(1),
  projectId: z.string().min(1),
  category: z.string().min(1),
  title: z.string().min(1),
  description: z.string().nullable(),
  priority: z.number().int(),
  internalStatus: InternalStatusSchema,
  /** Whether this task needs a review point (human-in-loop checkpoint) */
  needsReviewPoint: z.boolean().default(false),
  // Accept RFC3339 timestamps with offset (e.g., +00:00)
  createdAt: z.string().datetime({ offset: true }),
  updatedAt: z.string().datetime({ offset: true }),
  startedAt: z.string().datetime({ offset: true }).nullable(),
  completedAt: z.string().datetime({ offset: true }).nullable(),
});

export type Task = z.infer<typeof TaskSchema>;

/**
 * Common task categories
 */
export const TASK_CATEGORIES = [
  "feature",
  "bug",
  "chore",
  "docs",
  "test",
  "refactor",
] as const;

export const TaskCategorySchema = z.enum(TASK_CATEGORIES);
export type TaskCategory = z.infer<typeof TaskCategorySchema>;

/**
 * Schema for creating a new task
 * Excludes auto-generated fields (id, timestamps, status)
 */
export const CreateTaskSchema = z.object({
  projectId: z.string().min(1, "Project ID is required"),
  category: z.string().min(1, "Category is required").default("feature"),
  title: z.string().min(1, "Title is required"),
  description: z.string().optional(),
  priority: z.number().int().default(0),
  /** Override for QA enablement. null means inherit from global settings. */
  needsQa: z.boolean().nullable().optional(),
});

export type CreateTask = z.infer<typeof CreateTaskSchema>;

/**
 * Schema for updating a task
 * All fields are optional
 */
export const UpdateTaskSchema = z.object({
  category: z.string().min(1).optional(),
  title: z.string().min(1).optional(),
  description: z.string().nullable().optional(),
  priority: z.number().int().optional(),
});

export type UpdateTask = z.infer<typeof UpdateTaskSchema>;

/**
 * Schema for task list response
 */
export const TaskListSchema = z.array(TaskSchema);
export type TaskList = z.infer<typeof TaskListSchema>;
