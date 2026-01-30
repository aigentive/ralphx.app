/**
 * Event type definitions for Tauri IPC communication
 *
 * These types define the structure of events emitted from the Rust backend
 * to the React frontend via Tauri's event system.
 */

import { z } from "zod";
import { InternalStatusSchema } from "./status";
import { TaskSchema } from "./task";

// ============================================================================
// Agent Message Events (high frequency)
// ============================================================================

/**
 * Schema for agent activity events
 * Emitted during agent execution for real-time activity streaming
 */
export const AgentMessageEventSchema = z.object({
  taskId: z.string(),
  type: z.enum(["thinking", "tool_call", "tool_result", "text", "error"]),
  content: z.string(),
  timestamp: z.number(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export type AgentMessageEvent = z.infer<typeof AgentMessageEventSchema>;

// ============================================================================
// Task Status Events
// ============================================================================

/**
 * Schema for task status changes
 * Emitted when a task transitions between states
 */
export const TaskStatusEventSchema = z.object({
  taskId: z.string(),
  fromStatus: z.string().nullable(),
  toStatus: z.string(),
  changedBy: z.enum(["user", "system", "ai_worker", "ai_reviewer", "ai_supervisor"]),
  reason: z.string().optional(),
});

export type TaskStatusEvent = z.infer<typeof TaskStatusEventSchema>;

// ============================================================================
// Supervisor Alert Events
// ============================================================================

/**
 * Schema for supervisor alerts
 * Emitted when the supervisor detects anomalies or issues
 */
export const SupervisorAlertEventSchema = z.object({
  taskId: z.string(),
  severity: z.enum(["low", "medium", "high", "critical"]),
  type: z.enum(["loop_detected", "stuck", "error", "escalation"]),
  message: z.string(),
  suggestedAction: z.string().optional(),
});

export type SupervisorAlertEvent = z.infer<typeof SupervisorAlertEventSchema>;

// ============================================================================
// Review Events
// ============================================================================

/**
 * Schema for review events
 * Emitted during the code review process
 */
export const ReviewEventSchema = z.object({
  taskId: z.string(),
  reviewId: z.string(),
  type: z.enum(["started", "completed", "needs_human", "fix_proposed"]),
  outcome: z.enum(["approved", "changes_requested", "escalated"]).optional(),
});

export type ReviewEvent = z.infer<typeof ReviewEventSchema>;

// ============================================================================
// File Change Events
// ============================================================================

/**
 * Schema for file change events (for diff viewer)
 * Emitted when files are modified by agents
 */
export const FileChangeEventSchema = z.object({
  projectId: z.string(),
  filePath: z.string(),
  changeType: z.enum(["created", "modified", "deleted"]),
});

export type FileChangeEvent = z.infer<typeof FileChangeEventSchema>;

// ============================================================================
// Progress Events
// ============================================================================

/**
 * Schema for progress events
 * Emitted to show task execution progress
 */
export const ProgressEventSchema = z.object({
  taskId: z.string(),
  progress: z.number().min(0).max(100),
  stage: z.string(),
});

export type ProgressEvent = z.infer<typeof ProgressEventSchema>;

// ============================================================================
// TaskEvent Discriminated Union
// ============================================================================

/**
 * Discriminated union for all task-related events
 * Used for Tauri event listening with runtime validation
 */
export const TaskEventSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("created"),
    task: TaskSchema,
  }),
  z.object({
    type: z.literal("updated"),
    taskId: z.string().uuid(),
    changes: TaskSchema.partial(),
  }),
  z.object({
    type: z.literal("deleted"),
    taskId: z.string().uuid(),
  }),
  z.object({
    type: z.literal("status_changed"),
    taskId: z.string().uuid(),
    from: InternalStatusSchema,
    to: InternalStatusSchema,
    changedBy: z.enum(["user", "system", "agent"]),
  }),
]);

export type TaskEvent = z.infer<typeof TaskEventSchema>;

// ============================================================================
// QA Events
// ============================================================================

/**
 * Schema for QA prep events
 * Emitted when QA preparation phase changes
 */
export const QAPrepEventSchema = z.object({
  taskId: z.string(),
  type: z.enum(["started", "completed", "failed"]),
  agentId: z.string().optional(),
  acceptanceCriteriaCount: z.number().int().nonnegative().optional(),
  testStepsCount: z.number().int().nonnegative().optional(),
  error: z.string().optional(),
});

export type QAPrepEvent = z.infer<typeof QAPrepEventSchema>;

/**
 * Schema for QA test events
 * Emitted during and after QA test execution
 */
export const QATestEventSchema = z.object({
  taskId: z.string(),
  type: z.enum(["started", "passed", "failed"]),
  agentId: z.string().optional(),
  totalSteps: z.number().int().nonnegative().optional(),
  passedSteps: z.number().int().nonnegative().optional(),
  failedSteps: z.number().int().nonnegative().optional(),
  error: z.string().optional(),
});

export type QATestEvent = z.infer<typeof QATestEventSchema>;

// ============================================================================
// Proposal Events
// ============================================================================

/**
 * Schema for task proposal events
 * Emitted when proposals are created, updated, or deleted in ideation sessions
 */
export const ProposalEventSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("created"),
    proposal: z.object({
      id: z.string(),
      sessionId: z.string(),
      title: z.string(),
      description: z.string().nullable(),
      category: z.string(),
      steps: z.array(z.string()),
      acceptanceCriteria: z.array(z.string()),
      suggestedPriority: z.string(),
      priorityScore: z.number(),
      priorityReason: z.string().nullable(),
      estimatedComplexity: z.string(),
      userPriority: z.string().nullable(),
      userModified: z.boolean(),
      status: z.string(),
      selected: z.boolean(),
      createdTaskId: z.string().nullable(),
      planArtifactId: z.string().nullable(),
      planVersionAtCreation: z.number().nullable(),
      sortOrder: z.number(),
      createdAt: z.string(),
      updatedAt: z.string(),
    }),
  }),
  z.object({
    type: z.literal("updated"),
    proposal: z.object({
      id: z.string(),
      sessionId: z.string(),
      title: z.string(),
      description: z.string().nullable(),
      category: z.string(),
      steps: z.array(z.string()),
      acceptanceCriteria: z.array(z.string()),
      suggestedPriority: z.string(),
      priorityScore: z.number(),
      priorityReason: z.string().nullable(),
      estimatedComplexity: z.string(),
      userPriority: z.string().nullable(),
      userModified: z.boolean(),
      status: z.string(),
      selected: z.boolean(),
      createdTaskId: z.string().nullable(),
      planArtifactId: z.string().nullable(),
      planVersionAtCreation: z.number().nullable(),
      sortOrder: z.number(),
      createdAt: z.string(),
      updatedAt: z.string(),
    }),
  }),
  z.object({
    type: z.literal("deleted"),
    proposalId: z.string(),
  }),
]);

export type ProposalEvent = z.infer<typeof ProposalEventSchema>;

/**
 * Schema for proposals:reordered event
 * Emitted when proposals are reordered within a session
 */
export const ProposalsReorderedEventSchema = z.object({
  sessionId: z.string(),
  proposals: z.array(
    z.object({
      id: z.string(),
      sessionId: z.string(),
      title: z.string(),
      description: z.string().nullable(),
      category: z.string(),
      steps: z.array(z.string()),
      acceptanceCriteria: z.array(z.string()),
      suggestedPriority: z.string(),
      priorityScore: z.number(),
      priorityReason: z.string().nullable(),
      estimatedComplexity: z.string(),
      userPriority: z.string().nullable(),
      userModified: z.boolean(),
      status: z.string(),
      selected: z.boolean(),
      createdTaskId: z.string().nullable(),
      planArtifactId: z.string().nullable(),
      planVersionAtCreation: z.number().nullable(),
      sortOrder: z.number(),
      createdAt: z.string(),
      updatedAt: z.string(),
    })
  ),
});

export type ProposalsReorderedEvent = z.infer<typeof ProposalsReorderedEventSchema>;

// ============================================================================
// Plan Artifact Events
// ============================================================================

/**
 * Schema for plan artifact events
 * Emitted when plan artifacts are created or updated in ideation sessions
 */
export const PlanArtifactEventSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("created"),
    sessionId: z.string(),
    artifact: z.object({
      id: z.string(),
      name: z.string(),
      content: z.string(),
      version: z.number(),
    }),
  }),
  z.object({
    type: z.literal("updated"),
    artifactId: z.string(),
    artifact: z.object({
      id: z.string(),
      name: z.string(),
      content: z.string(),
      version: z.number(),
    }),
  }),
]);

export type PlanArtifactEvent = z.infer<typeof PlanArtifactEventSchema>;
