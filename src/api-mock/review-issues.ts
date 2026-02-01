/**
 * Mock Review Issues API
 *
 * Mirrors the interface of src/api/review-issues.ts with mock implementations.
 */

import type {
  ReviewIssue,
  IssueProgressSummary,
  IssueStatus,
  IssueSeverity,
  IssueCategory,
} from "@/types/review-issue";
import type {
  VerifyIssueInput,
  ReopenIssueInput,
  MarkIssueInProgressInput,
  MarkIssueAddressedInput,
  IssueStatusFilter,
} from "@/api/review-issues";

// ============================================================================
// Mock Data Factory
// ============================================================================

function createMockIssue(
  taskId: string,
  index: number,
  overrides: Partial<ReviewIssue> = {}
): ReviewIssue {
  const severities: IssueSeverity[] = [
    "critical",
    "major",
    "minor",
    "suggestion",
  ];
  const categories: IssueCategory[] = ["bug", "missing", "quality", "design"];
  const statuses: IssueStatus[] = [
    "open",
    "in_progress",
    "addressed",
    "verified",
  ];

  const severity = severities[index % severities.length] ?? "minor";
  const category = categories[index % categories.length] ?? "bug";
  const status = statuses[index % statuses.length] ?? "open";

  return {
    id: `issue-${taskId}-${index}`,
    reviewNoteId: `note-${taskId}-1`,
    taskId,
    stepId: index % 2 === 0 ? `step-${taskId}-${index}` : null,
    noStepReason:
      index % 2 !== 0 ? "Issue applies to overall implementation" : null,
    title: `Mock Issue ${index + 1}`,
    description: `This is a mock issue description for testing purposes.`,
    severity,
    category,
    filePath: index % 3 === 0 ? `src/components/Example${index}.tsx` : null,
    lineNumber: index % 3 === 0 ? 42 + index * 10 : null,
    codeSnippet: null,
    status,
    resolutionNotes:
      status === "addressed"
        ? "Fixed by updating the implementation"
        : null,
    addressedInAttempt: status === "addressed" ? 2 : null,
    verifiedByReviewId: null,
    createdAt: new Date(Date.now() - (index + 1) * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - index * 30 * 60 * 1000).toISOString(),
    ...overrides,
  };
}

// ============================================================================
// Mock Review Issues API
// ============================================================================

export const mockReviewIssuesApi = {
  getByTaskId: async (
    taskId: string,
    statusFilter?: IssueStatusFilter
  ): Promise<ReviewIssue[]> => {
    // Generate mock issues for visual testing
    const issues = [
      createMockIssue(taskId, 0, {
        title: "Missing error handling in API call",
        severity: "critical",
        status: "open",
        category: "bug",
      }),
      createMockIssue(taskId, 1, {
        title: "Variable naming doesn't follow conventions",
        severity: "minor",
        status: "addressed",
        category: "quality",
        resolutionNotes: "Renamed variables to follow camelCase convention",
        addressedInAttempt: 2,
      }),
      createMockIssue(taskId, 2, {
        title: "Missing unit tests for edge cases",
        severity: "major",
        status: "in_progress",
        category: "missing",
      }),
      createMockIssue(taskId, 3, {
        title: "Consider adding loading state indicator",
        severity: "suggestion",
        status: "verified",
        category: "design",
      }),
    ];

    if (statusFilter === "open") {
      return issues.filter(
        (issue) => issue.status === "open" || issue.status === "in_progress"
      );
    }

    return issues;
  },

  getProgress: async (taskId: string): Promise<IssueProgressSummary> => {
    return {
      taskId,
      total: 4,
      open: 1,
      inProgress: 1,
      addressed: 1,
      verified: 1,
      wontfix: 0,
      percentResolved: 50,
      bySeverity: {
        critical: { total: 1, open: 1, resolved: 0 },
        major: { total: 1, open: 0, resolved: 0 },
        minor: { total: 1, open: 0, resolved: 1 },
        suggestion: { total: 1, open: 0, resolved: 1 },
      },
    };
  },

  verify: async (input: VerifyIssueInput): Promise<ReviewIssue> => {
    // Return a mock verified issue
    return createMockIssue("task-1", 0, {
      id: input.issue_id,
      status: "verified",
      verifiedByReviewId: input.review_note_id,
    });
  },

  reopen: async (input: ReopenIssueInput): Promise<ReviewIssue> => {
    // Return a mock reopened issue
    return createMockIssue("task-1", 0, {
      id: input.issue_id,
      status: "open",
      resolutionNotes: input.reason ?? null,
    });
  },

  markInProgress: async (
    input: MarkIssueInProgressInput
  ): Promise<ReviewIssue> => {
    // Return a mock in-progress issue
    return createMockIssue("task-1", 0, {
      id: input.issue_id,
      status: "in_progress",
    });
  },

  markAddressed: async (
    input: MarkIssueAddressedInput
  ): Promise<ReviewIssue> => {
    // Return a mock addressed issue
    return createMockIssue("task-1", 0, {
      id: input.issue_id,
      status: "addressed",
      resolutionNotes: input.resolution_notes,
      addressedInAttempt: input.attempt_number,
    });
  },
} as const;
