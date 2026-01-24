/**
 * Workflow schema type definitions
 *
 * WorkflowSchema defines how external columns map to internal statuses,
 * enabling custom kanban workflows while preserving state machine behavior.
 */

import { z } from "zod";
import { InternalStatusSchema } from "./status";

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
  /** Optional default agent profiles */
  defaults: WorkflowDefaultsSchema.optional(),
});

export type WorkflowSchema = z.infer<typeof WorkflowSchemaZ>;
