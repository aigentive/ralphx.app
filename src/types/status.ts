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
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "reviewing",
  "review_passed",
  "revision_needed",
  "re_executing",
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
  "re_executing",
  "qa_refining",
  "qa_testing",
  "pending_review",
  "reviewing",
  "review_passed",
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
 * Review statuses where tasks are in the review process
 */
export const REVIEW_STATUSES: readonly InternalStatus[] = [
  "pending_review",
  "reviewing",
  "review_passed",
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

/**
 * Check if a status is a review status
 */
export function isReviewStatus(status: InternalStatus): boolean {
  return (REVIEW_STATUSES as readonly string[]).includes(status);
}
