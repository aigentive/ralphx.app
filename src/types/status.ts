// Internal status types and Zod schema
// Must match the 14 internal statuses from the Rust backend

import { z } from "zod";

/**
 * All 14 internal status values matching the Rust backend
 * Uses snake_case to match Rust serde serialization
 */
export const InternalStatusSchema = z.enum([
  "backlog",
  "ready",
  "blocked",
  "executing",
  "execution_done",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "revision_needed",
  "approved",
  "failed",
  "cancelled",
]);

export type InternalStatus = z.infer<typeof InternalStatusSchema>;

/**
 * All internal status values as a readonly array
 */
export const INTERNAL_STATUS_VALUES = InternalStatusSchema.options;

/**
 * Idle statuses where tasks are not being actively worked on
 */
export const IDLE_STATUSES: readonly InternalStatus[] = [
  "backlog",
  "ready",
  "blocked",
] as const;

/**
 * Active statuses where tasks are being worked on
 */
export const ACTIVE_STATUSES: readonly InternalStatus[] = [
  "executing",
  "execution_done",
  "qa_refining",
  "qa_testing",
  "pending_review",
  "revision_needed",
] as const;

/**
 * Terminal statuses where tasks are complete
 */
export const TERMINAL_STATUSES: readonly InternalStatus[] = [
  "approved",
  "failed",
  "cancelled",
] as const;

/**
 * Check if a status is a terminal status
 */
export function isTerminalStatus(status: InternalStatus): boolean {
  return (TERMINAL_STATUSES as readonly string[]).includes(status);
}

/**
 * Check if a status is an active status
 */
export function isActiveStatus(status: InternalStatus): boolean {
  return (ACTIVE_STATUSES as readonly string[]).includes(status);
}

/**
 * Check if a status is an idle status
 */
export function isIdleStatus(status: InternalStatus): boolean {
  return (IDLE_STATUSES as readonly string[]).includes(status);
}
