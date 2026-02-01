import { describe, it, expect } from "vitest";
import {
  IssueStatusSchema,
  IssueSeveritySchema,
  IssueCategorySchema,
  ReviewIssueResponseSchema,
  IssueProgressSummaryResponseSchema,
  SeverityCountResponseSchema,
  SeverityBreakdownResponseSchema,
  ReviewIssueListResponseSchema,
  ISSUE_STATUS_VALUES,
  ISSUE_SEVERITY_VALUES,
  ISSUE_CATEGORY_VALUES,
  transformReviewIssue,
  transformIssueProgressSummary,
  transformSeverityCount,
  transformSeverityBreakdown,
  transformReviewIssueList,
  isIssueOpen,
  isIssueInProgress,
  isIssueAddressed,
  isIssueVerified,
  isIssueWontFix,
  isIssueTerminal,
  isIssueResolved,
  isIssueNeedsWork,
  getSeverityPriority,
  isSeverityBlocking,
  sortBySeverity,
  isCodeIssue,
  isRequirementsIssue,
} from "./review-issue";

// ========================================
// Enum Schema Tests
// ========================================

describe("IssueStatusSchema", () => {
  it("should have 5 status values", () => {
    expect(ISSUE_STATUS_VALUES.length).toBe(5);
  });

  it("should parse valid statuses", () => {
    expect(IssueStatusSchema.parse("open")).toBe("open");
    expect(IssueStatusSchema.parse("in_progress")).toBe("in_progress");
    expect(IssueStatusSchema.parse("addressed")).toBe("addressed");
    expect(IssueStatusSchema.parse("verified")).toBe("verified");
    expect(IssueStatusSchema.parse("wontfix")).toBe("wontfix");
  });

  it("should reject invalid statuses", () => {
    expect(() => IssueStatusSchema.parse("pending")).toThrow();
    expect(() => IssueStatusSchema.parse("Open")).toThrow();
    expect(() => IssueStatusSchema.parse("")).toThrow();
  });
});

describe("IssueSeveritySchema", () => {
  it("should have 4 severity values", () => {
    expect(ISSUE_SEVERITY_VALUES.length).toBe(4);
  });

  it("should parse valid severities", () => {
    expect(IssueSeveritySchema.parse("critical")).toBe("critical");
    expect(IssueSeveritySchema.parse("major")).toBe("major");
    expect(IssueSeveritySchema.parse("minor")).toBe("minor");
    expect(IssueSeveritySchema.parse("suggestion")).toBe("suggestion");
  });

  it("should reject invalid severities", () => {
    expect(() => IssueSeveritySchema.parse("high")).toThrow();
    expect(() => IssueSeveritySchema.parse("Critical")).toThrow();
    expect(() => IssueSeveritySchema.parse("")).toThrow();
  });
});

describe("IssueCategorySchema", () => {
  it("should have 4 category values", () => {
    expect(ISSUE_CATEGORY_VALUES.length).toBe(4);
  });

  it("should parse valid categories", () => {
    expect(IssueCategorySchema.parse("bug")).toBe("bug");
    expect(IssueCategorySchema.parse("missing")).toBe("missing");
    expect(IssueCategorySchema.parse("quality")).toBe("quality");
    expect(IssueCategorySchema.parse("design")).toBe("design");
  });

  it("should reject invalid categories", () => {
    expect(() => IssueCategorySchema.parse("error")).toThrow();
    expect(() => IssueCategorySchema.parse("Bug")).toThrow();
    expect(() => IssueCategorySchema.parse("")).toThrow();
  });
});

// ========================================
// ReviewIssue Schema & Transform Tests
// ========================================

describe("ReviewIssueResponseSchema", () => {
  const validIssue = {
    id: "issue-123",
    review_note_id: "rn-456",
    task_id: "task-789",
    step_id: null,
    no_step_reason: "General code quality issue",
    title: "Missing error handling",
    description: "The function does not handle null input",
    severity: "major" as const,
    category: "bug" as const,
    file_path: "src/lib/utils.ts",
    line_number: 42,
    code_snippet: "function process(data) { return data.value; }",
    status: "open" as const,
    resolution_notes: null,
    addressed_in_attempt: null,
    verified_by_review_id: null,
    created_at: "2026-01-31T10:00:00+00:00",
    updated_at: "2026-01-31T10:00:00+00:00",
  };

  it("should parse a valid issue with all fields", () => {
    expect(() => ReviewIssueResponseSchema.parse(validIssue)).not.toThrow();
  });

  it("should parse an issue with step_id instead of no_step_reason", () => {
    const issueWithStep = {
      ...validIssue,
      step_id: "step-123",
      no_step_reason: null,
    };
    expect(() => ReviewIssueResponseSchema.parse(issueWithStep)).not.toThrow();
  });

  it("should parse an issue with minimal optional fields", () => {
    const minimalIssue = {
      id: "issue-123",
      review_note_id: "rn-456",
      task_id: "task-789",
      step_id: null,
      no_step_reason: null,
      title: "Test issue",
      description: null,
      severity: "minor" as const,
      category: null,
      file_path: null,
      line_number: null,
      code_snippet: null,
      status: "open" as const,
      resolution_notes: null,
      addressed_in_attempt: null,
      verified_by_review_id: null,
      created_at: "2026-01-31T10:00:00+00:00",
      updated_at: "2026-01-31T10:00:00+00:00",
    };
    expect(() => ReviewIssueResponseSchema.parse(minimalIssue)).not.toThrow();
  });

  it("should parse an addressed issue", () => {
    const addressedIssue = {
      ...validIssue,
      status: "addressed" as const,
      resolution_notes: "Fixed by adding null check",
      addressed_in_attempt: 2,
    };
    expect(() => ReviewIssueResponseSchema.parse(addressedIssue)).not.toThrow();
  });

  it("should parse a verified issue", () => {
    const verifiedIssue = {
      ...validIssue,
      status: "verified" as const,
      resolution_notes: "Fixed by adding null check",
      addressed_in_attempt: 2,
      verified_by_review_id: "rn-789",
    };
    expect(() => ReviewIssueResponseSchema.parse(verifiedIssue)).not.toThrow();
  });

  it("should reject issue with empty id", () => {
    expect(() =>
      ReviewIssueResponseSchema.parse({ ...validIssue, id: "" })
    ).toThrow();
  });

  it("should reject issue with empty title", () => {
    expect(() =>
      ReviewIssueResponseSchema.parse({ ...validIssue, title: "" })
    ).toThrow();
  });

  it("should reject issue with invalid status", () => {
    expect(() =>
      ReviewIssueResponseSchema.parse({ ...validIssue, status: "pending" })
    ).toThrow();
  });

  it("should reject issue with invalid severity", () => {
    expect(() =>
      ReviewIssueResponseSchema.parse({ ...validIssue, severity: "high" })
    ).toThrow();
  });
});

describe("transformReviewIssue", () => {
  const rawIssue = {
    id: "issue-123",
    review_note_id: "rn-456",
    task_id: "task-789",
    step_id: "step-1",
    no_step_reason: null,
    title: "Test issue",
    description: "Description",
    severity: "critical" as const,
    category: "bug" as const,
    file_path: "src/main.ts",
    line_number: 10,
    code_snippet: "code here",
    status: "addressed" as const,
    resolution_notes: "Fixed it",
    addressed_in_attempt: 1,
    verified_by_review_id: "rn-999",
    created_at: "2026-01-31T10:00:00+00:00",
    updated_at: "2026-01-31T11:00:00+00:00",
  };

  it("should transform snake_case to camelCase", () => {
    const result = transformReviewIssue(rawIssue);

    expect(result.id).toBe("issue-123");
    expect(result.reviewNoteId).toBe("rn-456");
    expect(result.taskId).toBe("task-789");
    expect(result.stepId).toBe("step-1");
    expect(result.noStepReason).toBeNull();
    expect(result.title).toBe("Test issue");
    expect(result.description).toBe("Description");
    expect(result.severity).toBe("critical");
    expect(result.category).toBe("bug");
    expect(result.filePath).toBe("src/main.ts");
    expect(result.lineNumber).toBe(10);
    expect(result.codeSnippet).toBe("code here");
    expect(result.status).toBe("addressed");
    expect(result.resolutionNotes).toBe("Fixed it");
    expect(result.addressedInAttempt).toBe(1);
    expect(result.verifiedByReviewId).toBe("rn-999");
    expect(result.createdAt).toBe("2026-01-31T10:00:00+00:00");
    expect(result.updatedAt).toBe("2026-01-31T11:00:00+00:00");
  });

  it("should handle null optional fields", () => {
    const rawWithNulls = {
      ...rawIssue,
      step_id: null,
      no_step_reason: "No step",
      description: null,
      category: null,
      file_path: null,
      line_number: null,
      code_snippet: null,
      resolution_notes: null,
      addressed_in_attempt: null,
      verified_by_review_id: null,
    };
    const result = transformReviewIssue(rawWithNulls);

    expect(result.stepId).toBeNull();
    expect(result.noStepReason).toBe("No step");
    expect(result.description).toBeNull();
    expect(result.category).toBeNull();
    expect(result.filePath).toBeNull();
    expect(result.lineNumber).toBeNull();
    expect(result.codeSnippet).toBeNull();
    expect(result.resolutionNotes).toBeNull();
    expect(result.addressedInAttempt).toBeNull();
    expect(result.verifiedByReviewId).toBeNull();
  });
});

// ========================================
// SeverityCount & Breakdown Tests
// ========================================

describe("SeverityCountResponseSchema", () => {
  it("should parse valid severity count", () => {
    const count = { total: 5, open: 2, resolved: 3 };
    expect(() => SeverityCountResponseSchema.parse(count)).not.toThrow();
  });

  it("should reject negative values", () => {
    expect(() =>
      SeverityCountResponseSchema.parse({ total: -1, open: 0, resolved: 0 })
    ).toThrow();
  });
});

describe("transformSeverityCount", () => {
  it("should transform severity count", () => {
    const raw = { total: 5, open: 2, resolved: 3 };
    const result = transformSeverityCount(raw);
    expect(result.total).toBe(5);
    expect(result.open).toBe(2);
    expect(result.resolved).toBe(3);
  });
});

describe("SeverityBreakdownResponseSchema", () => {
  const validBreakdown = {
    critical: { total: 1, open: 0, resolved: 1 },
    major: { total: 2, open: 1, resolved: 1 },
    minor: { total: 3, open: 2, resolved: 1 },
    suggestion: { total: 0, open: 0, resolved: 0 },
  };

  it("should parse valid breakdown", () => {
    expect(() => SeverityBreakdownResponseSchema.parse(validBreakdown)).not.toThrow();
  });
});

describe("transformSeverityBreakdown", () => {
  it("should transform all severity levels", () => {
    const raw = {
      critical: { total: 1, open: 0, resolved: 1 },
      major: { total: 2, open: 1, resolved: 1 },
      minor: { total: 3, open: 2, resolved: 1 },
      suggestion: { total: 4, open: 0, resolved: 4 },
    };
    const result = transformSeverityBreakdown(raw);

    expect(result.critical.total).toBe(1);
    expect(result.major.open).toBe(1);
    expect(result.minor.resolved).toBe(1);
    expect(result.suggestion.total).toBe(4);
  });
});

// ========================================
// IssueProgressSummary Tests
// ========================================

describe("IssueProgressSummaryResponseSchema", () => {
  const validSummary = {
    task_id: "task-123",
    total: 10,
    open: 2,
    in_progress: 1,
    addressed: 3,
    verified: 3,
    wontfix: 1,
    percent_resolved: 70.0,
    by_severity: {
      critical: { total: 2, open: 1, resolved: 1 },
      major: { total: 3, open: 0, resolved: 3 },
      minor: { total: 3, open: 1, resolved: 2 },
      suggestion: { total: 2, open: 0, resolved: 2 },
    },
  };

  it("should parse valid progress summary", () => {
    expect(() => IssueProgressSummaryResponseSchema.parse(validSummary)).not.toThrow();
  });

  it("should reject empty task_id", () => {
    expect(() =>
      IssueProgressSummaryResponseSchema.parse({ ...validSummary, task_id: "" })
    ).toThrow();
  });

  it("should reject negative counts", () => {
    expect(() =>
      IssueProgressSummaryResponseSchema.parse({ ...validSummary, total: -1 })
    ).toThrow();
  });

  it("should reject percent_resolved over 100", () => {
    expect(() =>
      IssueProgressSummaryResponseSchema.parse({
        ...validSummary,
        percent_resolved: 101,
      })
    ).toThrow();
  });
});

describe("transformIssueProgressSummary", () => {
  const rawSummary = {
    task_id: "task-123",
    total: 10,
    open: 2,
    in_progress: 1,
    addressed: 3,
    verified: 3,
    wontfix: 1,
    percent_resolved: 70.0,
    by_severity: {
      critical: { total: 2, open: 1, resolved: 1 },
      major: { total: 3, open: 0, resolved: 3 },
      minor: { total: 3, open: 1, resolved: 2 },
      suggestion: { total: 2, open: 0, resolved: 2 },
    },
  };

  it("should transform snake_case to camelCase", () => {
    const result = transformIssueProgressSummary(rawSummary);

    expect(result.taskId).toBe("task-123");
    expect(result.total).toBe(10);
    expect(result.open).toBe(2);
    expect(result.inProgress).toBe(1);
    expect(result.addressed).toBe(3);
    expect(result.verified).toBe(3);
    expect(result.wontfix).toBe(1);
    expect(result.percentResolved).toBe(70.0);
    expect(result.bySeverity.critical.total).toBe(2);
    expect(result.bySeverity.major.resolved).toBe(3);
  });
});

// ========================================
// List Schema & Transform Tests
// ========================================

describe("ReviewIssueListResponseSchema", () => {
  it("should parse empty array", () => {
    expect(() => ReviewIssueListResponseSchema.parse([])).not.toThrow();
  });

  it("should parse array of issues", () => {
    const issues = [
      {
        id: "issue-1",
        review_note_id: "rn-1",
        task_id: "task-1",
        step_id: null,
        no_step_reason: "Reason",
        title: "Issue 1",
        description: null,
        severity: "major" as const,
        category: null,
        file_path: null,
        line_number: null,
        code_snippet: null,
        status: "open" as const,
        resolution_notes: null,
        addressed_in_attempt: null,
        verified_by_review_id: null,
        created_at: "2026-01-31T10:00:00+00:00",
        updated_at: "2026-01-31T10:00:00+00:00",
      },
    ];
    expect(() => ReviewIssueListResponseSchema.parse(issues)).not.toThrow();
  });
});

describe("transformReviewIssueList", () => {
  it("should transform empty array", () => {
    const result = transformReviewIssueList([]);
    expect(result).toEqual([]);
  });

  it("should transform array of issues", () => {
    const raw = [
      {
        id: "issue-1",
        review_note_id: "rn-1",
        task_id: "task-1",
        step_id: null,
        no_step_reason: "Reason",
        title: "Issue 1",
        description: null,
        severity: "major" as const,
        category: null,
        file_path: null,
        line_number: null,
        code_snippet: null,
        status: "open" as const,
        resolution_notes: null,
        addressed_in_attempt: null,
        verified_by_review_id: null,
        created_at: "2026-01-31T10:00:00+00:00",
        updated_at: "2026-01-31T10:00:00+00:00",
      },
    ];
    const result = transformReviewIssueList(raw);
    expect(result.length).toBe(1);
    expect(result[0]?.reviewNoteId).toBe("rn-1");
  });
});

// ========================================
// Status Helper Tests
// ========================================

describe("Status helpers", () => {
  describe("isIssueOpen", () => {
    it("should return true for open", () => {
      expect(isIssueOpen("open")).toBe(true);
    });
    it("should return false for other statuses", () => {
      expect(isIssueOpen("in_progress")).toBe(false);
      expect(isIssueOpen("addressed")).toBe(false);
      expect(isIssueOpen("verified")).toBe(false);
      expect(isIssueOpen("wontfix")).toBe(false);
    });
  });

  describe("isIssueInProgress", () => {
    it("should return true for in_progress", () => {
      expect(isIssueInProgress("in_progress")).toBe(true);
    });
    it("should return false for other statuses", () => {
      expect(isIssueInProgress("open")).toBe(false);
      expect(isIssueInProgress("addressed")).toBe(false);
    });
  });

  describe("isIssueAddressed", () => {
    it("should return true for addressed", () => {
      expect(isIssueAddressed("addressed")).toBe(true);
    });
    it("should return false for other statuses", () => {
      expect(isIssueAddressed("open")).toBe(false);
      expect(isIssueAddressed("verified")).toBe(false);
    });
  });

  describe("isIssueVerified", () => {
    it("should return true for verified", () => {
      expect(isIssueVerified("verified")).toBe(true);
    });
    it("should return false for other statuses", () => {
      expect(isIssueVerified("open")).toBe(false);
      expect(isIssueVerified("addressed")).toBe(false);
    });
  });

  describe("isIssueWontFix", () => {
    it("should return true for wontfix", () => {
      expect(isIssueWontFix("wontfix")).toBe(true);
    });
    it("should return false for other statuses", () => {
      expect(isIssueWontFix("open")).toBe(false);
      expect(isIssueWontFix("verified")).toBe(false);
    });
  });

  describe("isIssueTerminal", () => {
    it("should return true for verified and wontfix", () => {
      expect(isIssueTerminal("verified")).toBe(true);
      expect(isIssueTerminal("wontfix")).toBe(true);
    });
    it("should return false for non-terminal statuses", () => {
      expect(isIssueTerminal("open")).toBe(false);
      expect(isIssueTerminal("in_progress")).toBe(false);
      expect(isIssueTerminal("addressed")).toBe(false);
    });
  });

  describe("isIssueResolved", () => {
    it("should return true for addressed, verified, wontfix", () => {
      expect(isIssueResolved("addressed")).toBe(true);
      expect(isIssueResolved("verified")).toBe(true);
      expect(isIssueResolved("wontfix")).toBe(true);
    });
    it("should return false for non-resolved statuses", () => {
      expect(isIssueResolved("open")).toBe(false);
      expect(isIssueResolved("in_progress")).toBe(false);
    });
  });

  describe("isIssueNeedsWork", () => {
    it("should return true for open and in_progress", () => {
      expect(isIssueNeedsWork("open")).toBe(true);
      expect(isIssueNeedsWork("in_progress")).toBe(true);
    });
    it("should return false for resolved statuses", () => {
      expect(isIssueNeedsWork("addressed")).toBe(false);
      expect(isIssueNeedsWork("verified")).toBe(false);
      expect(isIssueNeedsWork("wontfix")).toBe(false);
    });
  });
});

// ========================================
// Severity Helper Tests
// ========================================

describe("Severity helpers", () => {
  describe("getSeverityPriority", () => {
    it("should return 0 for critical (highest priority)", () => {
      expect(getSeverityPriority("critical")).toBe(0);
    });
    it("should return 1 for major", () => {
      expect(getSeverityPriority("major")).toBe(1);
    });
    it("should return 2 for minor", () => {
      expect(getSeverityPriority("minor")).toBe(2);
    });
    it("should return 3 for suggestion (lowest priority)", () => {
      expect(getSeverityPriority("suggestion")).toBe(3);
    });
    it("should have correct ordering", () => {
      expect(getSeverityPriority("critical")).toBeLessThan(
        getSeverityPriority("major")
      );
      expect(getSeverityPriority("major")).toBeLessThan(
        getSeverityPriority("minor")
      );
      expect(getSeverityPriority("minor")).toBeLessThan(
        getSeverityPriority("suggestion")
      );
    });
  });

  describe("isSeverityBlocking", () => {
    it("should return true for critical and major", () => {
      expect(isSeverityBlocking("critical")).toBe(true);
      expect(isSeverityBlocking("major")).toBe(true);
    });
    it("should return false for minor and suggestion", () => {
      expect(isSeverityBlocking("minor")).toBe(false);
      expect(isSeverityBlocking("suggestion")).toBe(false);
    });
  });

  describe("sortBySeverity", () => {
    it("should sort issues by severity (critical first)", () => {
      const issues = [
        { severity: "suggestion" },
        { severity: "critical" },
        { severity: "minor" },
        { severity: "major" },
      ] as Array<{ severity: "critical" | "major" | "minor" | "suggestion" } & Record<string, unknown>>;

      // Cast to full ReviewIssue type for testing
      const sortedPartial = sortBySeverity(issues as never);
      const severities = sortedPartial.map((i: { severity: string }) => i.severity);

      expect(severities).toEqual(["critical", "major", "minor", "suggestion"]);
    });

    it("should not mutate original array", () => {
      const issues = [
        { severity: "minor" },
        { severity: "critical" },
      ] as Array<{ severity: "critical" | "minor" } & Record<string, unknown>>;

      sortBySeverity(issues as never);
      expect(issues[0]?.severity).toBe("minor");
    });
  });
});

// ========================================
// Category Helper Tests
// ========================================

describe("Category helpers", () => {
  describe("isCodeIssue", () => {
    it("should return true for bug and quality", () => {
      expect(isCodeIssue("bug")).toBe(true);
      expect(isCodeIssue("quality")).toBe(true);
    });
    it("should return false for missing and design", () => {
      expect(isCodeIssue("missing")).toBe(false);
      expect(isCodeIssue("design")).toBe(false);
    });
    it("should return false for null", () => {
      expect(isCodeIssue(null)).toBe(false);
    });
  });

  describe("isRequirementsIssue", () => {
    it("should return true for missing and design", () => {
      expect(isRequirementsIssue("missing")).toBe(true);
      expect(isRequirementsIssue("design")).toBe(true);
    });
    it("should return false for bug and quality", () => {
      expect(isRequirementsIssue("bug")).toBe(false);
      expect(isRequirementsIssue("quality")).toBe(false);
    });
    it("should return false for null", () => {
      expect(isRequirementsIssue(null)).toBe(false);
    });
  });
});
