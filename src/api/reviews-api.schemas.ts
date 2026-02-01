// Reviews API Response Schemas (snake_case from backend)
// These schemas validate Rust backend responses before transformation

import { z } from "zod";
import {
  ReviewerTypeSchema,
  ReviewStatusSchema,
  ReviewOutcomeSchema,
} from "@/types";

// ============================================================================
// Review Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Review response from Rust
 * Note: field names use snake_case as that's what Rust serde produces
 */
export const ReviewResponseSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  task_id: z.string(),
  reviewer_type: ReviewerTypeSchema,
  status: ReviewStatusSchema,
  notes: z.string().nullable().optional(),
  created_at: z.string(),
  completed_at: z.string().nullable().optional(),
});

export type ReviewResponse = z.infer<typeof ReviewResponseSchema>;

/**
 * Review action response from Rust
 */
export const ReviewActionResponseSchema = z.object({
  id: z.string(),
  review_id: z.string(),
  action_type: z.string(),
  target_task_id: z.string().nullable().optional(),
  created_at: z.string(),
});

export type ReviewActionResponse = z.infer<typeof ReviewActionResponseSchema>;

/**
 * Review issue from AI reviewer escalation
 */
export const ReviewIssueSchema = z.object({
  severity: z.string(), // "critical" | "major" | "minor" | "suggestion"
  file: z.string().nullish(), // can be string, null, or missing
  line: z.number().int().nullish(), // can be number, null, or missing
  description: z.string(),
});

export type ReviewIssue = z.infer<typeof ReviewIssueSchema>;

/**
 * Review note response from Rust (state history)
 */
export const ReviewNoteResponseSchema = z.object({
  id: z.string(),
  task_id: z.string(),
  reviewer: ReviewerTypeSchema,
  outcome: ReviewOutcomeSchema,
  notes: z.string().nullable().optional(),
  issues: z.array(ReviewIssueSchema).nullable().optional(),
  created_at: z.string(),
});

export type ReviewNoteResponse = z.infer<typeof ReviewNoteResponseSchema>;

/**
 * Fix task attempts response from Rust
 */
export const FixTaskAttemptsResponseSchema = z.object({
  task_id: z.string(),
  attempt_count: z.number().int().nonnegative(),
});

export type FixTaskAttemptsResponse = z.infer<typeof FixTaskAttemptsResponseSchema>;

/**
 * List schemas for array responses
 */
export const ReviewListResponseSchema = z.array(ReviewResponseSchema);
export const ReviewNoteListResponseSchema = z.array(ReviewNoteResponseSchema);
