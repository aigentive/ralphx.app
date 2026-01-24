// QA Configuration types
// Global and per-task QA settings with Zod schemas

import { z } from "zod";

// ==================== QA Prep Status ====================

export const QAPrepStatusSchema = z.enum([
  "pending",
  "running",
  "completed",
  "failed",
]);

export type QAPrepStatus = z.infer<typeof QAPrepStatusSchema>;

export const QA_PREP_STATUS_VALUES = QAPrepStatusSchema.options;

export function isPrepComplete(status: QAPrepStatus): boolean {
  return status === "completed";
}

export function isPrepFailed(status: QAPrepStatus): boolean {
  return status === "failed";
}

// ==================== QA Test Status ====================

export const QATestStatusSchema = z.enum([
  "pending",
  "waiting_for_prep",
  "running",
  "passed",
  "failed",
]);

export type QATestStatus = z.infer<typeof QATestStatusSchema>;

export const QA_TEST_STATUS_VALUES = QATestStatusSchema.options;

export function isTestTerminal(status: QATestStatus): boolean {
  return status === "passed" || status === "failed";
}

export function isTestPassed(status: QATestStatus): boolean {
  return status === "passed";
}

export function isTestFailed(status: QATestStatus): boolean {
  return status === "failed";
}

// ==================== QA Settings (Global) ====================

export const QASettingsSchema = z.object({
  /** Master toggle for QA system */
  qa_enabled: z.boolean(),
  /** Automatically enable QA for UI-related tasks */
  auto_qa_for_ui_tasks: z.boolean(),
  /** Automatically enable QA for API tasks */
  auto_qa_for_api_tasks: z.boolean(),
  /** Enable QA Prep phase (background acceptance criteria generation) */
  qa_prep_enabled: z.boolean(),
  /** Enable browser-based testing */
  browser_testing_enabled: z.boolean(),
  /** URL for browser testing (typically dev server) */
  browser_testing_url: z.string().url(),
});

export type QASettings = z.infer<typeof QASettingsSchema>;

export const DEFAULT_QA_SETTINGS: QASettings = {
  qa_enabled: true,
  auto_qa_for_ui_tasks: true,
  auto_qa_for_api_tasks: false,
  qa_prep_enabled: true,
  browser_testing_enabled: true,
  browser_testing_url: "http://localhost:1420",
};

/** Categories that are considered UI tasks */
const UI_CATEGORIES = ["ui", "component", "feature"];

/** Categories that are considered API tasks */
const API_CATEGORIES = ["api", "backend", "endpoint"];

/**
 * Check if QA should run for a given task category based on global settings
 */
export function shouldRunQAForCategory(
  settings: QASettings,
  category: string
): boolean {
  if (!settings.qa_enabled) {
    return false;
  }

  if (UI_CATEGORIES.includes(category)) {
    return settings.auto_qa_for_ui_tasks;
  }

  if (API_CATEGORIES.includes(category)) {
    return settings.auto_qa_for_api_tasks;
  }

  return false;
}

// ==================== Task QA Config (Per-Task) ====================

export const TaskQAConfigSchema = z.object({
  /** Override for QA enablement. null means inherit from global settings. */
  needs_qa: z.boolean().nullable(),
  /** Current status of QA preparation phase */
  qa_prep_status: QAPrepStatusSchema,
  /** Current status of QA testing phase */
  qa_test_status: QATestStatusSchema,
});

export type TaskQAConfig = z.infer<typeof TaskQAConfigSchema>;

export const DEFAULT_TASK_QA_CONFIG: TaskQAConfig = {
  needs_qa: null,
  qa_prep_status: "pending",
  qa_test_status: "pending",
};

/**
 * Check if QA is required for a task, considering global settings
 */
export function requiresQA(
  config: TaskQAConfig,
  globalSettings: QASettings,
  taskCategory: string
): boolean {
  // Explicit override takes precedence
  if (config.needs_qa !== null) {
    return config.needs_qa;
  }

  // Fall back to global settings
  return shouldRunQAForCategory(globalSettings, taskCategory);
}

/**
 * Create a TaskQAConfig with explicit QA requirement
 */
export function createTaskQAConfig(needsQA: boolean): TaskQAConfig {
  return {
    needs_qa: needsQA,
    qa_prep_status: "pending",
    qa_test_status: "pending",
  };
}

/**
 * Create a TaskQAConfig that inherits from global settings
 */
export function createInheritedTaskQAConfig(): TaskQAConfig {
  return { ...DEFAULT_TASK_QA_CONFIG };
}

// ==================== Parsing Utilities ====================

export function parseQASettings(data: unknown): QASettings {
  return QASettingsSchema.parse(data);
}

export function safeParseQASettings(data: unknown): QASettings | null {
  const result = QASettingsSchema.safeParse(data);
  return result.success ? result.data : null;
}

export function parseTaskQAConfig(data: unknown): TaskQAConfig {
  return TaskQAConfigSchema.parse(data);
}

export function safeParseTaskQAConfig(data: unknown): TaskQAConfig | null {
  const result = TaskQAConfigSchema.safeParse(data);
  return result.success ? result.data : null;
}
