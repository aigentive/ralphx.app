// Task Step types and Zod schema
// Must match the Rust backend TaskStep struct

import { z } from "zod";

/**
 * Task step status enum
 * Matches Rust TaskStepStatus enum with serde(rename_all = "snake_case")
 */
export const TaskStepStatusSchema = z.enum([
  "pending",
  "in_progress",
  "completed",
  "skipped",
  "failed",
  "cancelled",
]);

export type TaskStepStatus = z.infer<typeof TaskStepStatusSchema>;

/**
 * Task step schema matching Rust backend serialization
 * Note: field names use camelCase as that's what serde_json produces with rename_all
 */
export const TaskStepSchema = z.object({
  id: z.string().min(1),
  taskId: z.string().min(1),
  title: z.string().min(1),
  description: z.string().nullable(),
  status: TaskStepStatusSchema,
  sortOrder: z.number().int(),
  dependsOn: z.string().nullable(),
  createdBy: z.string().min(1),
  completionNote: z.string().nullable(),
  // Accept RFC3339 timestamps with offset (e.g., +00:00)
  createdAt: z.string().datetime({ offset: true }),
  updatedAt: z.string().datetime({ offset: true }),
  startedAt: z.string().datetime({ offset: true }).nullable(),
  completedAt: z.string().datetime({ offset: true }).nullable(),
});

export type TaskStep = z.infer<typeof TaskStepSchema>;

/**
 * Step progress summary schema
 * Provides aggregated statistics and current/next step info
 */
export const StepProgressSummarySchema = z.object({
  taskId: z.string().min(1),
  total: z.number().int().min(0),
  completed: z.number().int().min(0),
  inProgress: z.number().int().min(0),
  pending: z.number().int().min(0),
  skipped: z.number().int().min(0),
  failed: z.number().int().min(0),
  currentStep: TaskStepSchema.nullable(),
  nextStep: TaskStepSchema.nullable(),
  percentComplete: z.number().min(0).max(100),
});

export type StepProgressSummary = z.infer<typeof StepProgressSummarySchema>;

/**
 * Status helpers
 */
export const isTaskStepPending = (status: TaskStepStatus) => status === "pending";
export const isTaskStepInProgress = (status: TaskStepStatus) => status === "in_progress";
export const isTaskStepCompleted = (status: TaskStepStatus) => status === "completed";
export const isTaskStepSkipped = (status: TaskStepStatus) => status === "skipped";
export const isTaskStepFailed = (status: TaskStepStatus) => status === "failed";
export const isTaskStepCancelled = (status: TaskStepStatus) => status === "cancelled";
export const isTaskStepTerminal = (status: TaskStepStatus) =>
  status === "completed" || status === "skipped" || status === "failed" || status === "cancelled";
export const isTaskStepActive = (status: TaskStepStatus) => status === "in_progress";

/**
 * All possible step status values
 */
export const TASK_STEP_STATUS_VALUES = TaskStepStatusSchema.options;
