/**
 * Workflow schema type definitions
 *
 * WorkflowSchema defines how external columns map to internal statuses,
 * enabling custom kanban workflows while preserving state machine behavior.
 */

import { z } from "zod";
import { InternalStatusSchema } from "./status";

// ============================================
// External Sync Configuration Types (future)
// ============================================

/**
 * Supported external sync providers
 */
export const SyncProviderSchema = z.enum(["jira", "github", "linear", "notion"]);
export type SyncProvider = z.infer<typeof SyncProviderSchema>;

/**
 * All sync provider values as a readonly array
 */
export const SYNC_PROVIDER_VALUES = SyncProviderSchema.options;

/**
 * Sync direction options
 */
export const SyncDirectionSchema = z.enum(["pull", "push", "bidirectional"]);
export type SyncDirection = z.infer<typeof SyncDirectionSchema>;

/**
 * All sync direction values as a readonly array
 */
export const SYNC_DIRECTION_VALUES = SyncDirectionSchema.options;

/**
 * Conflict resolution strategy
 */
export const ConflictResolutionSchema = z.enum([
  "external_wins",
  "internal_wins",
  "manual",
]);
export type ConflictResolution = z.infer<typeof ConflictResolutionSchema>;

/**
 * All conflict resolution values as a readonly array
 */
export const CONFLICT_RESOLUTION_VALUES = ConflictResolutionSchema.options;

/**
 * Mapping from an external status to internal status
 */
export const ExternalStatusMappingSchema = z.object({
  /** The external status name from the provider */
  externalStatus: z.string(),
  /** The internal status to map to */
  internalStatus: InternalStatusSchema,
  /** The workflow column to display in */
  columnId: z.string(),
});
export type ExternalStatusMapping = z.infer<typeof ExternalStatusMappingSchema>;

/**
 * Sync direction settings
 */
export const SyncSettingsSchema = z.object({
  /** Sync direction */
  direction: SyncDirectionSchema,
  /** Enable webhook for real-time sync */
  webhook: z.boolean().optional(),
});
export type SyncSettings = z.infer<typeof SyncSettingsSchema>;

/**
 * External sync configuration (placeholder for future implementation)
 */
export const ExternalSyncConfigSchema = z.object({
  /** The external provider */
  provider: SyncProviderSchema,
  /** Status mapping from external to internal (keyed by external status) */
  mapping: z.record(z.string(), ExternalStatusMappingSchema).default({}),
  /** Sync settings */
  sync: SyncSettingsSchema,
  /** How to resolve conflicts */
  conflictResolution: ConflictResolutionSchema,
});
export type ExternalSyncConfig = z.infer<typeof ExternalSyncConfigSchema>;

// ============================================
// Column Behavior and Workflow Types
// ============================================

/**
 * Column behavior configuration
 * Controls how tasks behave when moved to this column
 */
const ColumnBehaviorSchema = z.object({
  /** Skip AI review for tasks in this column */
  skipReview: z.boolean().optional(),
  /** Automatically advance to next column when execution completes */
  autoAdvance: z.boolean().optional(),
  /** Agent profile to use for tasks in this column */
  agentProfile: z.string().optional(),
});

/**
 * Schema for a workflow column
 * Maps an external column to an internal status
 */
export const WorkflowColumnSchema = z.object({
  /** Unique column identifier */
  id: z.string(),
  /** Display name for the column */
  name: z.string(),
  /** Optional color for the column header */
  color: z.string().optional(),
  /** Optional icon name for the column */
  icon: z.string().optional(),
  /** Internal status this column maps to */
  mapsTo: InternalStatusSchema,
  /** Optional behavior configuration */
  behavior: ColumnBehaviorSchema.optional(),
});

export type WorkflowColumn = z.infer<typeof WorkflowColumnSchema>;

/**
 * Default agent profiles for the workflow
 */
const WorkflowDefaultsSchema = z.object({
  /** Default worker agent profile */
  workerProfile: z.string().optional(),
  /** Default reviewer agent profile */
  reviewerProfile: z.string().optional(),
});

/**
 * Schema for a complete workflow definition
 * Note: Named WorkflowSchemaZ to avoid collision with the TypeScript type
 */
export const WorkflowSchemaZ = z.object({
  /** Unique workflow identifier */
  id: z.string(),
  /** Display name for the workflow */
  name: z.string(),
  /** Optional description */
  description: z.string().optional(),
  /** Ordered list of columns in the workflow */
  columns: z.array(WorkflowColumnSchema),
  /** Optional external sync configuration (future implementation) */
  externalSync: ExternalSyncConfigSchema.optional(),
  /** Optional default agent profiles */
  defaults: WorkflowDefaultsSchema.optional(),
  /** Whether this is the default workflow */
  isDefault: z.boolean().default(false),
});

export type WorkflowSchema = z.infer<typeof WorkflowSchemaZ>;

/**
 * Default RalphX workflow with 5 columns
 * Maps to the standard kanban board structure
 */
export const defaultWorkflow: WorkflowSchema = {
  id: "ralphx-default",
  name: "RalphX Default",
  description: "Standard kanban workflow for AI-driven development",
  columns: [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "ready", name: "Ready", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  isDefault: true,
};

/**
 * Jira-compatible workflow with 5 columns
 * Matches familiar Jira board structure with external sync config
 */
export const jiraCompatibleWorkflow: WorkflowSchema = {
  id: "jira-compat",
  name: "Jira Compatible",
  description: "Jira-style workflow with familiar columns",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "selected", name: "Selected for Dev", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_qa", name: "In QA", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  externalSync: {
    provider: "jira",
    mapping: {},
    sync: {
      direction: "bidirectional",
      webhook: true,
    },
    conflictResolution: "external_wins",
  },
  isDefault: false,
};

/**
 * Built-in workflows array for easy iteration
 */
export const BUILTIN_WORKFLOWS: readonly WorkflowSchema[] = [
  defaultWorkflow,
  jiraCompatibleWorkflow,
] as const;

/**
 * Get a built-in workflow by ID
 */
export function getBuiltinWorkflow(id: string): WorkflowSchema | undefined {
  return BUILTIN_WORKFLOWS.find((w) => w.id === id);
}
