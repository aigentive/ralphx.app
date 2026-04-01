// Review Issue types and Zod schemas
// Must match the Rust backend ReviewIssue structs in review_commands_types.rs

import { z } from "zod";

// ========================================
// Enums
// ========================================

/**
 * Issue status in its lifecycle
 * Matches Rust IssueStatus enum with serde(rename_all = "snake_case")
 */
export const ISSUE_STATUS_VALUES = [
  "open",
  "in_progress",
  "addressed",
  "verified",
  "wontfix",
] as const;

export const IssueStatusSchema = z.enum(ISSUE_STATUS_VALUES);
export type IssueStatus = z.infer<typeof IssueStatusSchema>;

/**
 * Issue severity level
 * Matches Rust IssueSeverity enum with serde(rename_all = "snake_case")
 */
export const ISSUE_SEVERITY_VALUES = [
  "critical",
  "major",
  "minor",
  "suggestion",
] as const;

export const IssueSeveritySchema = z.enum(ISSUE_SEVERITY_VALUES);
export type IssueSeverity = z.infer<typeof IssueSeveritySchema>;

/**
 * Issue category
 * Matches Rust IssueCategory enum with serde(rename_all = "snake_case")
 */
export const ISSUE_CATEGORY_VALUES = ["bug", "missing", "quality", "design"] as const;

export const IssueCategorySchema = z.enum(ISSUE_CATEGORY_VALUES);
export type IssueCategory = z.infer<typeof IssueCategorySchema>;

// ========================================
// Response Schemas (snake_case from Rust)
// ========================================

/**
 * Review issue response schema matching Rust backend serialization (snake_case)
 * Backend outputs snake_case (Rust default). Transform layer converts to camelCase for UI.
 */
export const ReviewIssueResponseSchema = z.object({
  id: z.string().min(1),
  review_note_id: z.string().min(1),
  task_id: z.string().min(1),
  step_id: z.string().nullable(),
  no_step_reason: z.string().nullable(),
  title: z.string().min(1),
  description: z.string().nullable(),
  severity: IssueSeveritySchema,
  category: IssueCategorySchema.nullable(),
  file_path: z.string().nullable(),
  line_number: z.number().int().nullable(),
  code_snippet: z.string().nullable(),
  status: IssueStatusSchema,
  resolution_notes: z.string().nullable(),
  addressed_in_attempt: z.number().int().nullable(),
  verified_by_review_id: z.string().nullable(),
  created_at: z.string().datetime({ offset: true }),
  updated_at: z.string().datetime({ offset: true }),
});

/**
 * Frontend ReviewIssue type (camelCase)
 * This is what components and stores use. Transformed from snake_case API responses.
 */
export interface ReviewIssue {
  id: string;
  reviewNoteId: string;
  taskId: string;
  stepId: string | null;
  noStepReason: string | null;
  title: string;
  description: string | null;
  severity: IssueSeverity;
  category: IssueCategory | null;
  filePath: string | null;
  lineNumber: number | null;
  codeSnippet: string | null;
  status: IssueStatus;
  resolutionNotes: string | null;
  addressedInAttempt: number | null;
  verifiedByReviewId: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Transform function to convert snake_case API response to camelCase frontend type
 */
export function transformReviewIssue(
  raw: z.infer<typeof ReviewIssueResponseSchema>
): ReviewIssue {
  return {
    id: raw.id,
    reviewNoteId: raw.review_note_id,
    taskId: raw.task_id,
    stepId: raw.step_id,
    noStepReason: raw.no_step_reason,
    title: raw.title,
    description: raw.description,
    severity: raw.severity,
    category: raw.category,
    filePath: raw.file_path,
    lineNumber: raw.line_number,
    codeSnippet: raw.code_snippet,
    status: raw.status,
    resolutionNotes: raw.resolution_notes,
    addressedInAttempt: raw.addressed_in_attempt,
    verifiedByReviewId: raw.verified_by_review_id,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

// Legacy export for backward compatibility
export const ReviewIssueSchema = ReviewIssueResponseSchema;

// ========================================
// Severity Count & Breakdown (for Progress Summary)
// ========================================

/**
 * Severity count response schema (snake_case from Rust)
 */
export const SeverityCountResponseSchema = z.object({
  total: z.number().int().min(0),
  open: z.number().int().min(0),
  resolved: z.number().int().min(0),
});

/**
 * Frontend SeverityCount type (camelCase - same field names in this case)
 */
export interface SeverityCount {
  total: number;
  open: number;
  resolved: number;
}

/**
 * Transform function for SeverityCount
 */
export function transformSeverityCount(
  raw: z.infer<typeof SeverityCountResponseSchema>
): SeverityCount {
  return {
    total: raw.total,
    open: raw.open,
    resolved: raw.resolved,
  };
}

/**
 * Severity breakdown response schema (snake_case from Rust)
 */
export const SeverityBreakdownResponseSchema = z.object({
  critical: SeverityCountResponseSchema,
  major: SeverityCountResponseSchema,
  minor: SeverityCountResponseSchema,
  suggestion: SeverityCountResponseSchema,
});

/**
 * Frontend SeverityBreakdown type (camelCase - same field names in this case)
 */
export interface SeverityBreakdown {
  critical: SeverityCount;
  major: SeverityCount;
  minor: SeverityCount;
  suggestion: SeverityCount;
}

/**
 * Transform function for SeverityBreakdown
 */
export function transformSeverityBreakdown(
  raw: z.infer<typeof SeverityBreakdownResponseSchema>
): SeverityBreakdown {
  return {
    critical: transformSeverityCount(raw.critical),
    major: transformSeverityCount(raw.major),
    minor: transformSeverityCount(raw.minor),
    suggestion: transformSeverityCount(raw.suggestion),
  };
}

// ========================================
// Issue Progress Summary
// ========================================

/**
 * Issue progress summary response schema (snake_case from Rust)
 * Backend outputs snake_case (Rust default). Transform layer converts to camelCase for UI.
 */
export const IssueProgressSummaryResponseSchema = z.object({
  task_id: z.string().min(1),
  total: z.number().int().min(0),
  open: z.number().int().min(0),
  in_progress: z.number().int().min(0),
  addressed: z.number().int().min(0),
  verified: z.number().int().min(0),
  wontfix: z.number().int().min(0),
  percent_resolved: z.number().min(0).max(100),
  by_severity: SeverityBreakdownResponseSchema,
});

/**
 * Frontend IssueProgressSummary type (camelCase)
 */
export interface IssueProgressSummary {
  taskId: string;
  total: number;
  open: number;
  inProgress: number;
  addressed: number;
  verified: number;
  wontfix: number;
  percentResolved: number;
  bySeverity: SeverityBreakdown;
}

/**
 * Transform function for IssueProgressSummary
 */
export function transformIssueProgressSummary(
  raw: z.infer<typeof IssueProgressSummaryResponseSchema>
): IssueProgressSummary {
  return {
    taskId: raw.task_id,
    total: raw.total,
    open: raw.open,
    inProgress: raw.in_progress,
    addressed: raw.addressed,
    verified: raw.verified,
    wontfix: raw.wontfix,
    percentResolved: raw.percent_resolved,
    bySeverity: transformSeverityBreakdown(raw.by_severity),
  };
}

// Legacy export for backward compatibility
export const IssueProgressSummarySchema = IssueProgressSummaryResponseSchema;

// ========================================
// List Schemas
// ========================================

/**
 * Schema for review issue list response
 */
export const ReviewIssueListResponseSchema = z.array(ReviewIssueResponseSchema);
export type ReviewIssueListResponse = z.infer<typeof ReviewIssueListResponseSchema>;

/**
 * Transform function for issue list
 */
export function transformReviewIssueList(
  raw: ReviewIssueListResponse
): ReviewIssue[] {
  return raw.map(transformReviewIssue);
}

// ========================================
// Status Helpers
// ========================================

/**
 * Check if issue is open
 */
export function isIssueOpen(status: IssueStatus): boolean {
  return status === "open";
}

/**
 * Check if issue is in progress
 */
export function isIssueInProgress(status: IssueStatus): boolean {
  return status === "in_progress";
}

/**
 * Check if issue has been addressed
 */
export function isIssueAddressed(status: IssueStatus): boolean {
  return status === "addressed";
}

/**
 * Check if issue has been verified
 */
export function isIssueVerified(status: IssueStatus): boolean {
  return status === "verified";
}

/**
 * Check if issue was marked won't fix
 */
export function isIssueWontFix(status: IssueStatus): boolean {
  return status === "wontfix";
}

/**
 * Check if issue is in a terminal state (no further transitions expected)
 */
export function isIssueTerminal(status: IssueStatus): boolean {
  return status === "verified" || status === "wontfix";
}

/**
 * Check if issue is resolved (addressed, verified, or wontfix)
 */
export function isIssueResolved(status: IssueStatus): boolean {
  return status === "addressed" || status === "verified" || status === "wontfix";
}

/**
 * Check if issue needs work (open or in progress)
 */
export function isIssueNeedsWork(status: IssueStatus): boolean {
  return status === "open" || status === "in_progress";
}

// ========================================
// Severity Helpers
// ========================================

/**
 * Get priority order for severity (lower = higher priority)
 */
export function getSeverityPriority(severity: IssueSeverity): number {
  switch (severity) {
    case "critical":
      return 0;
    case "major":
      return 1;
    case "minor":
      return 2;
    case "suggestion":
      return 3;
  }
}

/**
 * Check if severity is blocking (critical or major)
 */
export function isSeverityBlocking(severity: IssueSeverity): boolean {
  return severity === "critical" || severity === "major";
}

/**
 * Sort issues by severity (critical first)
 */
export function sortBySeverity(issues: ReviewIssue[]): ReviewIssue[] {
  return [...issues].sort(
    (a, b) => getSeverityPriority(a.severity) - getSeverityPriority(b.severity)
  );
}

// ========================================
// Category Helpers
// ========================================

/**
 * Check if category is a code issue (bug or quality)
 */
export function isCodeIssue(category: IssueCategory | null): boolean {
  return category === "bug" || category === "quality";
}

/**
 * Check if category is a requirements issue (missing or design)
 */
export function isRequirementsIssue(category: IssueCategory | null): boolean {
  return category === "missing" || category === "design";
}
