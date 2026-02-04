/**
 * TaskDetailModal constants and configuration
 */

import type { InternalStatus } from "@/types/task";

// Priority colors matching design spec
export const PRIORITY_COLORS: Record<number, { bg: string; text: string }> = {
  1: { bg: "var(--status-error)", text: "white" },
  2: { bg: "var(--accent-primary)", text: "white" },
  3: { bg: "var(--status-warning)", text: "var(--bg-base)" },
  4: { bg: "var(--bg-hover)", text: "var(--text-secondary)" },
};

// Status badge configuration matching design spec
export const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; bg: string; text: string }
> = {
  backlog: {
    label: "Backlog",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  ready: {
    label: "Ready",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  blocked: {
    label: "Blocked",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  executing: {
    label: "Executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  approved: {
    label: "Approved",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  failed: {
    label: "Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  reviewing: {
    label: "AI Review in Progress",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  review_passed: {
    label: "AI Review Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  escalated: {
    label: "Escalated",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  pending_merge: {
    label: "Pending Merge",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  merging: {
    label: "Merging",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  merge_conflict: {
    label: "Merge Conflict",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  merged: {
    label: "Merged",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  paused: {
    label: "Paused",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  stopped: {
    label: "Stopped",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
};

export const DEFAULT_PRIORITY_COLOR = { bg: "var(--bg-hover)", text: "var(--text-secondary)" };

// System-controlled statuses that cannot be manually edited
export const SYSTEM_CONTROLLED_STATUSES: InternalStatus[] = [
  "executing",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "revision_needed",
  "reviewing",
  "review_passed",
  "escalated",
  "re_executing",
  "pending_merge",
  "merging",
  "merge_conflict",
];
