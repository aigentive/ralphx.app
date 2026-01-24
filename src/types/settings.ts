// Project Settings types
// Global project configuration with Zod schemas

import { z } from "zod";
import { ModelSchema } from "./agent-profile";

// ============================================================================
// Execution Settings
// ============================================================================

export const ExecutionSettingsSchema = z.object({
  /** Maximum concurrent tasks to run */
  max_concurrent_tasks: z.number().int().min(1).max(10).default(2),
  /** Auto-commit changes after task completion */
  auto_commit: z.boolean().default(true),
  /** Commit message prefix (conventional commits) */
  commit_message_prefix: z.string().default("feat: "),
  /** Pause queue when a task fails */
  pause_on_failure: z.boolean().default(true),
  /** Review before destructive operations */
  review_before_destructive: z.boolean().default(true),
});

export type ExecutionSettings = z.infer<typeof ExecutionSettingsSchema>;

export const DEFAULT_EXECUTION_SETTINGS: ExecutionSettings = {
  max_concurrent_tasks: 2,
  auto_commit: true,
  commit_message_prefix: "feat: ",
  pause_on_failure: true,
  review_before_destructive: true,
};

// ============================================================================
// Model Settings
// ============================================================================

export const ModelSettingsSchema = z.object({
  /** Default model for task execution */
  model: ModelSchema.default("sonnet"),
  /** Allow upgrading to Opus for complex tasks */
  allow_opus_upgrade: z.boolean().default(true),
});

export type ModelSettings = z.infer<typeof ModelSettingsSchema>;

export const DEFAULT_MODEL_SETTINGS: ModelSettings = {
  model: "sonnet",
  allow_opus_upgrade: true,
};

// ============================================================================
// Project Review Settings (snake_case to match plan spec)
// ============================================================================

export const ProjectReviewSettingsSchema = z.object({
  /** Enable AI review of completed tasks */
  ai_review_enabled: z.boolean().default(true),
  /** Auto-create fix tasks for review failures */
  ai_review_auto_fix: z.boolean().default(true),
  /** Require approval before executing fix tasks */
  require_fix_approval: z.boolean().default(false),
  /** Require human review even after AI approval */
  require_human_review: z.boolean().default(false),
  /** Maximum fix attempts before moving to backlog */
  max_fix_attempts: z.number().int().min(1).max(10).default(3),
});

export type ProjectReviewSettings = z.infer<typeof ProjectReviewSettingsSchema>;

export const DEFAULT_PROJECT_REVIEW_SETTINGS: ProjectReviewSettings = {
  ai_review_enabled: true,
  ai_review_auto_fix: true,
  require_fix_approval: false,
  require_human_review: false,
  max_fix_attempts: 3,
};

// ============================================================================
// Supervisor Settings
// ============================================================================

export const SupervisorSettingsSchema = z.object({
  /** Enable watchdog monitoring */
  supervisor_enabled: z.boolean().default(true),
  /** Number of identical tool calls before loop detection */
  loop_threshold: z.number().int().min(2).max(10).default(3),
  /** Seconds without progress before stuck detection */
  stuck_timeout: z.number().int().min(60).max(1800).default(300),
});

export type SupervisorSettings = z.infer<typeof SupervisorSettingsSchema>;

export const DEFAULT_SUPERVISOR_SETTINGS: SupervisorSettings = {
  supervisor_enabled: true,
  loop_threshold: 3,
  stuck_timeout: 300,
};

// ============================================================================
// Combined Project Settings
// ============================================================================

export const ProjectSettingsSchema = z.object({
  execution: ExecutionSettingsSchema.default(DEFAULT_EXECUTION_SETTINGS),
  model: ModelSettingsSchema.default(DEFAULT_MODEL_SETTINGS),
  review: ProjectReviewSettingsSchema.default(DEFAULT_PROJECT_REVIEW_SETTINGS),
  supervisor: SupervisorSettingsSchema.default(DEFAULT_SUPERVISOR_SETTINGS),
});

export type ProjectSettings = z.infer<typeof ProjectSettingsSchema>;

export const DEFAULT_PROJECT_SETTINGS: ProjectSettings = {
  execution: DEFAULT_EXECUTION_SETTINGS,
  model: DEFAULT_MODEL_SETTINGS,
  review: DEFAULT_PROJECT_REVIEW_SETTINGS,
  supervisor: DEFAULT_SUPERVISOR_SETTINGS,
};

// ============================================================================
// Settings Profile
// ============================================================================

export const SettingsProfileSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  description: z.string().optional(),
  settings: ProjectSettingsSchema,
  isDefault: z.boolean().default(false),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type SettingsProfile = z.infer<typeof SettingsProfileSchema>;

// ============================================================================
// Parsing Utilities
// ============================================================================

export function parseProjectSettings(data: unknown): ProjectSettings {
  return ProjectSettingsSchema.parse(data);
}

export function safeParseProjectSettings(data: unknown): ProjectSettings | null {
  const result = ProjectSettingsSchema.safeParse(data);
  return result.success ? result.data : null;
}

export function parseSettingsProfile(data: unknown): SettingsProfile {
  return SettingsProfileSchema.parse(data);
}

export function safeParseSettingsProfile(data: unknown): SettingsProfile | null {
  const result = SettingsProfileSchema.safeParse(data);
  return result.success ? result.data : null;
}
