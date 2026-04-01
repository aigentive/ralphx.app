/**
 * Supervisor type definitions for watchdog monitoring system
 *
 * These types define the structure for supervisor events, alerts, and actions
 * that monitor agent execution and detect anomalies.
 */

import { z } from "zod";

// ============================================================================
// Severity Levels
// ============================================================================

/**
 * Severity levels for supervisor alerts
 */
export const SeveritySchema = z.enum(["low", "medium", "high", "critical"]);
export type Severity = z.infer<typeof SeveritySchema>;

// ============================================================================
// Supervisor Actions
// ============================================================================

/**
 * Actions the supervisor can take in response to detected issues
 */
export const SupervisorActionTypeSchema = z.enum([
  "log", // Log warning, continue monitoring
  "inject_guidance", // Inject guidance into agent context
  "pause", // Pause task, mark as blocked
  "kill", // Kill task, mark as failed
]);
export type SupervisorActionType = z.infer<typeof SupervisorActionTypeSchema>;

/**
 * Full supervisor action with metadata
 */
export const SupervisorActionSchema = z.object({
  type: SupervisorActionTypeSchema,
  severity: SeveritySchema,
  reason: z.string(),
  guidance: z.string().optional(), // For inject_guidance actions
  timestamp: z.string().datetime(),
});
export type SupervisorAction = z.infer<typeof SupervisorActionSchema>;

// ============================================================================
// Detection Patterns
// ============================================================================

/**
 * Types of patterns the supervisor can detect
 */
export const DetectionPatternSchema = z.enum([
  "infinite_loop", // Same tool called 3+ times with identical args
  "stuck", // No progress for 5+ minutes
  "repeating_error", // Same error occurring repeatedly
  "high_token_usage", // Token usage exceeding threshold
  "time_exceeded", // Task running too long
  "poor_task_definition", // Agent requesting clarification multiple times
]);
export type DetectionPattern = z.infer<typeof DetectionPatternSchema>;

// ============================================================================
// Tool Call Info
// ============================================================================

/**
 * Information about a tool call
 */
export const ToolCallInfoSchema = z.object({
  toolName: z.string(),
  arguments: z.string(), // JSON string of arguments
  timestamp: z.string().datetime(),
  success: z.boolean(),
  error: z.string().optional(),
});
export type ToolCallInfo = z.infer<typeof ToolCallInfoSchema>;

// ============================================================================
// Error Info
// ============================================================================

/**
 * Information about an error
 */
export const ErrorInfoSchema = z.object({
  message: z.string(),
  source: z.string(), // Tool or component that errored
  recoverable: z.boolean(),
  timestamp: z.string().datetime(),
});
export type ErrorInfo = z.infer<typeof ErrorInfoSchema>;

// ============================================================================
// Progress Info
// ============================================================================

/**
 * Information about execution progress
 */
export const ProgressInfoSchema = z.object({
  hasFileChanges: z.boolean(),
  filesModified: z.number(),
  hasNewCommits: z.boolean(),
  tokensUsed: z.number(),
  elapsedSeconds: z.number(),
  timestamp: z.string().datetime(),
});
export type ProgressInfo = z.infer<typeof ProgressInfoSchema>;

// ============================================================================
// Supervisor Events
// ============================================================================

/**
 * TaskStart event - emitted when agent begins task execution
 */
export const TaskStartEventSchema = z.object({
  type: z.literal("task_start"),
  taskId: z.string(),
  agentRole: z.string(),
  timestamp: z.string().datetime(),
});
export type TaskStartEvent = z.infer<typeof TaskStartEventSchema>;

/**
 * ToolCall event - emitted for each tool invocation
 */
export const ToolCallEventSchema = z.object({
  type: z.literal("tool_call"),
  taskId: z.string(),
  info: ToolCallInfoSchema,
});
export type ToolCallEvent = z.infer<typeof ToolCallEventSchema>;

/**
 * Error event - emitted when an error occurs
 */
export const ErrorEventSchema = z.object({
  type: z.literal("error"),
  taskId: z.string(),
  info: ErrorInfoSchema,
});
export type ErrorEvent = z.infer<typeof ErrorEventSchema>;

/**
 * ProgressTick event - periodic progress check
 */
export const ProgressTickEventSchema = z.object({
  type: z.literal("progress_tick"),
  taskId: z.string(),
  info: ProgressInfoSchema,
});
export type ProgressTickEvent = z.infer<typeof ProgressTickEventSchema>;

/**
 * TokenThreshold event - token usage exceeded threshold
 */
export const TokenThresholdEventSchema = z.object({
  type: z.literal("token_threshold"),
  taskId: z.string(),
  tokensUsed: z.number(),
  threshold: z.number(),
  timestamp: z.string().datetime(),
});
export type TokenThresholdEvent = z.infer<typeof TokenThresholdEventSchema>;

/**
 * TimeThreshold event - execution time exceeded threshold
 */
export const TimeThresholdEventSchema = z.object({
  type: z.literal("time_threshold"),
  taskId: z.string(),
  elapsedMinutes: z.number(),
  thresholdMinutes: z.number(),
  timestamp: z.string().datetime(),
});
export type TimeThresholdEvent = z.infer<typeof TimeThresholdEventSchema>;

/**
 * Union of all supervisor events
 */
export const SupervisorEventSchema = z.discriminatedUnion("type", [
  TaskStartEventSchema,
  ToolCallEventSchema,
  ErrorEventSchema,
  ProgressTickEventSchema,
  TokenThresholdEventSchema,
  TimeThresholdEventSchema,
]);
export type SupervisorEvent = z.infer<typeof SupervisorEventSchema>;

// ============================================================================
// Supervisor Alerts
// ============================================================================

/**
 * Alert types that can be generated
 */
export const AlertTypeSchema = z.enum([
  "loop_detected",
  "stuck",
  "error",
  "escalation",
  "token_warning",
  "time_warning",
]);
export type AlertType = z.infer<typeof AlertTypeSchema>;

/**
 * Full supervisor alert with all context
 */
export const SupervisorAlertSchema = z.object({
  id: z.string().uuid(),
  taskId: z.string(),
  type: AlertTypeSchema,
  severity: SeveritySchema,
  pattern: DetectionPatternSchema.optional(),
  message: z.string(),
  details: z.string().optional(),
  suggestedAction: SupervisorActionTypeSchema.optional(),
  acknowledged: z.boolean(),
  createdAt: z.string().datetime(),
  acknowledgedAt: z.string().datetime().optional(),
});
export type SupervisorAlert = z.infer<typeof SupervisorAlertSchema>;

// ============================================================================
// Supervisor Configuration
// ============================================================================

/**
 * Configuration for supervisor thresholds
 */
export const SupervisorConfigSchema = z.object({
  loopDetectionThreshold: z.number().default(3), // identical calls before loop detected
  stuckTimeoutMinutes: z.number().default(5), // minutes of no progress before stuck
  tokenWarningThreshold: z.number().default(50000), // token count warning
  timeWarningMinutes: z.number().default(10), // execution time warning
  errorRepeatThreshold: z.number().default(3), // repeated errors before escalation
});
export type SupervisorConfig = z.infer<typeof SupervisorConfigSchema>;

// ============================================================================
// Detection Result
// ============================================================================

/**
 * Result of pattern detection
 */
export const DetectionResultSchema = z.object({
  detected: z.boolean(),
  pattern: DetectionPatternSchema.optional(),
  severity: SeveritySchema.optional(),
  message: z.string().optional(),
  suggestedAction: SupervisorActionTypeSchema.optional(),
});
export type DetectionResult = z.infer<typeof DetectionResultSchema>;

// ============================================================================
// Task Monitor State
// ============================================================================

/**
 * Per-task monitoring state
 */
export const TaskMonitorStateSchema = z.object({
  taskId: z.string(),
  agentRole: z.string(),
  startedAt: z.string().datetime(),
  toolCalls: z.array(ToolCallInfoSchema),
  errors: z.array(ErrorInfoSchema),
  lastProgress: ProgressInfoSchema.optional(),
  isPaused: z.boolean(),
  isKilled: z.boolean(),
  pauseReason: z.string().optional(),
});
export type TaskMonitorState = z.infer<typeof TaskMonitorStateSchema>;
