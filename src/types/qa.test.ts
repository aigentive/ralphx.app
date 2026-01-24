import { describe, it, expect } from "vitest";
import {
  // Acceptance Criteria
  AcceptanceCriteriaTypeSchema,
  AcceptanceCriterionSchema,
  AcceptanceCriteriaSchema,
  ACCEPTANCE_CRITERIA_TYPE_VALUES,
  // QA Test Steps
  QATestStepSchema,
  QATestStepsSchema,
  // QA Step Status
  QAStepStatusSchema,
  QA_STEP_STATUS_VALUES,
  isStepTerminal,
  isStepPassed,
  isStepFailed,
  // QA Overall Status
  QAOverallStatusSchema,
  QA_OVERALL_STATUS_VALUES,
  isOverallComplete,
  // QA Results
  QAStepResultSchema,
  QAResultsTotalsSchema,
  QAResultsSchema,
  calculateTotals,
  // TaskQA
  TaskQASchema,
  // Parsing utilities
  parseAcceptanceCriteria,
  safeParseAcceptanceCriteria,
  parseQATestSteps,
  safeParseQATestSteps,
  parseQAResults,
  safeParseQAResults,
  parseTaskQA,
  safeParseTaskQA,
} from "./qa";
import type {
  AcceptanceCriteriaType,
  AcceptanceCriterion,
  AcceptanceCriteria,
  QATestStep,
  QATestSteps,
  QAStepStatus,
  QAOverallStatus,
  QAStepResult,
  QAResultsTotals,
  QAResults,
  TaskQA,
} from "./qa";

// ============================================================================
// Acceptance Criteria Type Tests
// ============================================================================

describe("AcceptanceCriteriaType", () => {
  it("should have all expected values", () => {
    expect(ACCEPTANCE_CRITERIA_TYPE_VALUES).toEqual([
      "visual",
      "behavior",
      "data",
      "accessibility",
    ]);
  });

  it("should parse valid values", () => {
    expect(AcceptanceCriteriaTypeSchema.parse("visual")).toBe("visual");
    expect(AcceptanceCriteriaTypeSchema.parse("behavior")).toBe("behavior");
    expect(AcceptanceCriteriaTypeSchema.parse("data")).toBe("data");
    expect(AcceptanceCriteriaTypeSchema.parse("accessibility")).toBe("accessibility");
  });

  it("should reject invalid values", () => {
    expect(() => AcceptanceCriteriaTypeSchema.parse("invalid")).toThrow();
    expect(() => AcceptanceCriteriaTypeSchema.parse("")).toThrow();
    expect(() => AcceptanceCriteriaTypeSchema.parse(123)).toThrow();
  });
});

// ============================================================================
// Acceptance Criterion Tests
// ============================================================================

describe("AcceptanceCriterion", () => {
  it("should parse valid criterion", () => {
    const criterion = AcceptanceCriterionSchema.parse({
      id: "AC1",
      description: "User can see the task board",
      testable: true,
      type: "visual",
    });
    expect(criterion.id).toBe("AC1");
    expect(criterion.description).toBe("User can see the task board");
    expect(criterion.testable).toBe(true);
    expect(criterion.type).toBe("visual");
  });

  it("should reject missing required fields", () => {
    expect(() => AcceptanceCriterionSchema.parse({})).toThrow();
    expect(() => AcceptanceCriterionSchema.parse({ id: "AC1" })).toThrow();
  });

  it("should reject invalid type", () => {
    expect(() =>
      AcceptanceCriterionSchema.parse({
        id: "AC1",
        description: "Test",
        testable: true,
        type: "invalid",
      })
    ).toThrow();
  });

  it("should serialize correctly", () => {
    const criterion: AcceptanceCriterion = {
      id: "AC1",
      description: "Test",
      testable: true,
      type: "behavior",
    };
    const json = JSON.stringify(criterion);
    expect(json).toContain('"id":"AC1"');
    expect(json).toContain('"type":"behavior"');
  });
});

// ============================================================================
// Acceptance Criteria (Collection) Tests
// ============================================================================

describe("AcceptanceCriteria", () => {
  it("should parse valid criteria collection", () => {
    const criteria = AcceptanceCriteriaSchema.parse({
      acceptance_criteria: [
        { id: "AC1", description: "Visual test", testable: true, type: "visual" },
        { id: "AC2", description: "Behavior test", testable: true, type: "behavior" },
      ],
    });
    expect(criteria.acceptance_criteria).toHaveLength(2);
  });

  it("should parse empty criteria collection", () => {
    const criteria = AcceptanceCriteriaSchema.parse({
      acceptance_criteria: [],
    });
    expect(criteria.acceptance_criteria).toHaveLength(0);
  });

  it("should reject invalid criteria array", () => {
    expect(() =>
      AcceptanceCriteriaSchema.parse({
        acceptance_criteria: [{ invalid: true }],
      })
    ).toThrow();
  });

  it("should parse PRD format", () => {
    const json = `{
      "acceptance_criteria": [
        {
          "id": "AC1",
          "description": "User can see the task board with 7 columns",
          "testable": true,
          "type": "visual"
        },
        {
          "id": "AC2",
          "description": "Dragging a task to 'Planned' column triggers execution",
          "testable": true,
          "type": "behavior"
        }
      ]
    }`;
    const criteria = AcceptanceCriteriaSchema.parse(JSON.parse(json));
    expect(criteria.acceptance_criteria).toHaveLength(2);
    expect(criteria.acceptance_criteria[0].id).toBe("AC1");
    expect(criteria.acceptance_criteria[1].type).toBe("behavior");
  });

  it("parseAcceptanceCriteria parses valid data", () => {
    const result = parseAcceptanceCriteria({
      acceptance_criteria: [
        { id: "AC1", description: "Test", testable: true, type: "visual" },
      ],
    });
    expect(result.acceptance_criteria).toHaveLength(1);
  });

  it("safeParseAcceptanceCriteria returns null for invalid data", () => {
    expect(safeParseAcceptanceCriteria({ invalid: true })).toBeNull();
  });
});

// ============================================================================
// QA Test Step Tests
// ============================================================================

describe("QATestStep", () => {
  it("should parse valid test step", () => {
    const step = QATestStepSchema.parse({
      id: "QA1",
      criteria_id: "AC1",
      description: "Verify task board renders",
      commands: [
        "agent-browser open http://localhost:1420",
        "agent-browser wait --load",
      ],
      expected: "Board visible with 7 columns",
    });
    expect(step.id).toBe("QA1");
    expect(step.criteria_id).toBe("AC1");
    expect(step.commands).toHaveLength(2);
    expect(step.expected).toBe("Board visible with 7 columns");
  });

  it("should allow empty commands array", () => {
    const step = QATestStepSchema.parse({
      id: "QA1",
      criteria_id: "AC1",
      description: "Manual verification step",
      commands: [],
      expected: "Verified manually",
    });
    expect(step.commands).toHaveLength(0);
  });

  it("should reject missing fields", () => {
    expect(() =>
      QATestStepSchema.parse({
        id: "QA1",
        criteria_id: "AC1",
      })
    ).toThrow();
  });
});

// ============================================================================
// QA Test Steps (Collection) Tests
// ============================================================================

describe("QATestSteps", () => {
  it("should parse valid steps collection", () => {
    const steps = QATestStepsSchema.parse({
      qa_steps: [
        {
          id: "QA1",
          criteria_id: "AC1",
          description: "Test 1",
          commands: ["cmd1"],
          expected: "Result 1",
        },
        {
          id: "QA2",
          criteria_id: "AC2",
          description: "Test 2",
          commands: [],
          expected: "Result 2",
        },
      ],
    });
    expect(steps.qa_steps).toHaveLength(2);
  });

  it("should parse PRD format", () => {
    const json = `{
      "qa_steps": [
        {
          "id": "QA1",
          "criteria_id": "AC1",
          "description": "Verify task board renders with correct columns",
          "commands": [
            "agent-browser open http://localhost:1420",
            "agent-browser wait --load",
            "agent-browser snapshot -i -c",
            "agent-browser is visible [data-testid='column-draft']",
            "agent-browser is visible [data-testid='column-planned']",
            "agent-browser screenshot screenshots/task-board-columns.png"
          ],
          "expected": "All 7 columns visible"
        }
      ]
    }`;
    const steps = QATestStepsSchema.parse(JSON.parse(json));
    expect(steps.qa_steps).toHaveLength(1);
    expect(steps.qa_steps[0].commands).toHaveLength(6);
  });

  it("parseQATestSteps parses valid data", () => {
    const result = parseQATestSteps({
      qa_steps: [
        { id: "QA1", criteria_id: "AC1", description: "Test", commands: [], expected: "OK" },
      ],
    });
    expect(result.qa_steps).toHaveLength(1);
  });

  it("safeParseQATestSteps returns null for invalid data", () => {
    expect(safeParseQATestSteps({ invalid: true })).toBeNull();
  });
});

// ============================================================================
// QA Step Status Tests
// ============================================================================

describe("QAStepStatus", () => {
  it("should have all expected values", () => {
    expect(QA_STEP_STATUS_VALUES).toEqual([
      "pending",
      "running",
      "passed",
      "failed",
      "skipped",
    ]);
  });

  it("should parse valid values", () => {
    expect(QAStepStatusSchema.parse("pending")).toBe("pending");
    expect(QAStepStatusSchema.parse("running")).toBe("running");
    expect(QAStepStatusSchema.parse("passed")).toBe("passed");
    expect(QAStepStatusSchema.parse("failed")).toBe("failed");
    expect(QAStepStatusSchema.parse("skipped")).toBe("skipped");
  });

  it("should reject invalid values", () => {
    expect(() => QAStepStatusSchema.parse("invalid")).toThrow();
    expect(() => QAStepStatusSchema.parse("")).toThrow();
  });

  it("isStepTerminal returns true for terminal states", () => {
    expect(isStepTerminal("pending")).toBe(false);
    expect(isStepTerminal("running")).toBe(false);
    expect(isStepTerminal("passed")).toBe(true);
    expect(isStepTerminal("failed")).toBe(true);
    expect(isStepTerminal("skipped")).toBe(true);
  });

  it("isStepPassed returns true only for passed", () => {
    expect(isStepPassed("pending")).toBe(false);
    expect(isStepPassed("passed")).toBe(true);
    expect(isStepPassed("failed")).toBe(false);
  });

  it("isStepFailed returns true only for failed", () => {
    expect(isStepFailed("pending")).toBe(false);
    expect(isStepFailed("passed")).toBe(false);
    expect(isStepFailed("failed")).toBe(true);
  });
});

// ============================================================================
// QA Overall Status Tests
// ============================================================================

describe("QAOverallStatus", () => {
  it("should have all expected values", () => {
    expect(QA_OVERALL_STATUS_VALUES).toEqual([
      "pending",
      "running",
      "passed",
      "failed",
    ]);
  });

  it("should parse valid values", () => {
    expect(QAOverallStatusSchema.parse("pending")).toBe("pending");
    expect(QAOverallStatusSchema.parse("running")).toBe("running");
    expect(QAOverallStatusSchema.parse("passed")).toBe("passed");
    expect(QAOverallStatusSchema.parse("failed")).toBe("failed");
  });

  it("should reject invalid values", () => {
    expect(() => QAOverallStatusSchema.parse("invalid")).toThrow();
    expect(() => QAOverallStatusSchema.parse("skipped")).toThrow();
  });

  it("isOverallComplete returns true for terminal states", () => {
    expect(isOverallComplete("pending")).toBe(false);
    expect(isOverallComplete("running")).toBe(false);
    expect(isOverallComplete("passed")).toBe(true);
    expect(isOverallComplete("failed")).toBe(true);
  });
});

// ============================================================================
// QA Step Result Tests
// ============================================================================

describe("QAStepResult", () => {
  it("should parse passed result with screenshot", () => {
    const result = QAStepResultSchema.parse({
      step_id: "QA1",
      status: "passed",
      screenshot: "screenshots/qa1.png",
    });
    expect(result.step_id).toBe("QA1");
    expect(result.status).toBe("passed");
    expect(result.screenshot).toBe("screenshots/qa1.png");
    expect(result.error).toBeUndefined();
  });

  it("should parse failed result with error", () => {
    const result = QAStepResultSchema.parse({
      step_id: "QA1",
      status: "failed",
      error: "Element not found",
    });
    expect(result.status).toBe("failed");
    expect(result.error).toBe("Element not found");
  });

  it("should parse failed result with expected/actual comparison", () => {
    const result = QAStepResultSchema.parse({
      step_id: "QA1",
      status: "failed",
      expected: "7 columns",
      actual: "5 columns",
    });
    expect(result.expected).toBe("7 columns");
    expect(result.actual).toBe("5 columns");
  });

  it("should parse skipped result", () => {
    const result = QAStepResultSchema.parse({
      step_id: "QA1",
      status: "skipped",
      error: "Previous step failed",
    });
    expect(result.status).toBe("skipped");
    expect(result.error).toBe("Previous step failed");
  });

  it("should handle null optional fields", () => {
    const result = QAStepResultSchema.parse({
      step_id: "QA1",
      status: "passed",
      screenshot: null,
      actual: null,
      expected: null,
      error: null,
    });
    expect(result.screenshot).toBeNull();
    expect(result.actual).toBeNull();
    expect(result.expected).toBeNull();
    expect(result.error).toBeNull();
  });
});

// ============================================================================
// QA Results Totals Tests
// ============================================================================

describe("QAResultsTotals", () => {
  it("should parse valid totals", () => {
    const totals = QAResultsTotalsSchema.parse({
      total_steps: 5,
      passed_steps: 3,
      failed_steps: 1,
      skipped_steps: 1,
    });
    expect(totals.total_steps).toBe(5);
    expect(totals.passed_steps).toBe(3);
    expect(totals.failed_steps).toBe(1);
    expect(totals.skipped_steps).toBe(1);
  });

  it("should reject negative numbers", () => {
    expect(() =>
      QAResultsTotalsSchema.parse({
        total_steps: -1,
        passed_steps: 0,
        failed_steps: 0,
        skipped_steps: 0,
      })
    ).toThrow();
  });
});

describe("calculateTotals", () => {
  it("should calculate totals from step results", () => {
    const results: QAStepResult[] = [
      { step_id: "QA1", status: "passed" },
      { step_id: "QA2", status: "passed" },
      { step_id: "QA3", status: "failed", error: "Error" },
      { step_id: "QA4", status: "skipped" },
    ];
    const totals = calculateTotals(results);
    expect(totals.total_steps).toBe(4);
    expect(totals.passed_steps).toBe(2);
    expect(totals.failed_steps).toBe(1);
    expect(totals.skipped_steps).toBe(1);
  });

  it("should handle empty results", () => {
    const totals = calculateTotals([]);
    expect(totals.total_steps).toBe(0);
    expect(totals.passed_steps).toBe(0);
    expect(totals.failed_steps).toBe(0);
    expect(totals.skipped_steps).toBe(0);
  });
});

// ============================================================================
// QA Results Tests
// ============================================================================

describe("QAResults", () => {
  it("should parse valid results", () => {
    const results = QAResultsSchema.parse({
      task_id: "task-123",
      overall_status: "passed",
      total_steps: 2,
      passed_steps: 2,
      failed_steps: 0,
      steps: [
        { step_id: "QA1", status: "passed" },
        { step_id: "QA2", status: "passed" },
      ],
    });
    expect(results.task_id).toBe("task-123");
    expect(results.overall_status).toBe("passed");
    expect(results.steps).toHaveLength(2);
  });

  it("should parse PRD format", () => {
    const json = `{
      "task_id": "task-123",
      "overall_status": "passed",
      "total_steps": 5,
      "passed_steps": 5,
      "failed_steps": 0,
      "steps": [
        {
          "step_id": "QA1",
          "status": "passed",
          "screenshot": "screenshots/qa1-result.png",
          "actual": null,
          "expected": null,
          "error": null
        }
      ]
    }`;
    const results = QAResultsSchema.parse(JSON.parse(json));
    expect(results.task_id).toBe("task-123");
    expect(results.overall_status).toBe("passed");
    expect(results.steps[0].screenshot).toBe("screenshots/qa1-result.png");
  });

  it("parseQAResults parses valid data", () => {
    const result = parseQAResults({
      task_id: "task-123",
      overall_status: "pending",
      total_steps: 1,
      passed_steps: 0,
      failed_steps: 0,
      steps: [{ step_id: "QA1", status: "pending" }],
    });
    expect(result.task_id).toBe("task-123");
  });

  it("safeParseQAResults returns null for invalid data", () => {
    expect(safeParseQAResults({ invalid: true })).toBeNull();
  });
});

// ============================================================================
// TaskQA Tests
// ============================================================================

describe("TaskQA", () => {
  it("should parse minimal TaskQA", () => {
    const taskQA = TaskQASchema.parse({
      id: "qa-123",
      task_id: "task-123",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(taskQA.id).toBe("qa-123");
    expect(taskQA.task_id).toBe("task-123");
    expect(taskQA.acceptance_criteria).toBeUndefined();
    expect(taskQA.test_results).toBeUndefined();
  });

  it("should parse TaskQA with prep data", () => {
    const taskQA = TaskQASchema.parse({
      id: "qa-123",
      task_id: "task-123",
      acceptance_criteria: {
        acceptance_criteria: [
          { id: "AC1", description: "Test", testable: true, type: "visual" },
        ],
      },
      qa_test_steps: {
        qa_steps: [
          { id: "QA1", criteria_id: "AC1", description: "Step 1", commands: [], expected: "OK" },
        ],
      },
      prep_agent_id: "agent-1",
      prep_started_at: "2026-01-24T12:00:00Z",
      prep_completed_at: "2026-01-24T12:05:00Z",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(taskQA.acceptance_criteria?.acceptance_criteria).toHaveLength(1);
    expect(taskQA.qa_test_steps?.qa_steps).toHaveLength(1);
    expect(taskQA.prep_agent_id).toBe("agent-1");
  });

  it("should parse TaskQA with refinement data", () => {
    const taskQA = TaskQASchema.parse({
      id: "qa-123",
      task_id: "task-123",
      actual_implementation: "Added login button to header",
      refined_test_steps: {
        qa_steps: [
          { id: "QA1", criteria_id: "AC1", description: "Refined step", commands: [], expected: "OK" },
        ],
      },
      refinement_agent_id: "agent-2",
      refinement_completed_at: "2026-01-24T12:10:00Z",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(taskQA.actual_implementation).toBe("Added login button to header");
    expect(taskQA.refined_test_steps?.qa_steps).toHaveLength(1);
  });

  it("should parse TaskQA with test results", () => {
    const taskQA = TaskQASchema.parse({
      id: "qa-123",
      task_id: "task-123",
      test_results: {
        task_id: "task-123",
        overall_status: "passed",
        total_steps: 1,
        passed_steps: 1,
        failed_steps: 0,
        steps: [{ step_id: "QA1", status: "passed" }],
      },
      screenshots: ["screenshots/test1.png", "screenshots/test2.png"],
      test_agent_id: "agent-3",
      test_completed_at: "2026-01-24T12:15:00Z",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(taskQA.test_results?.overall_status).toBe("passed");
    expect(taskQA.screenshots).toHaveLength(2);
  });

  it("should default screenshots to empty array", () => {
    const taskQA = TaskQASchema.parse({
      id: "qa-123",
      task_id: "task-123",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(taskQA.screenshots).toEqual([]);
  });

  it("parseTaskQA parses valid data", () => {
    const result = parseTaskQA({
      id: "qa-123",
      task_id: "task-123",
      created_at: "2026-01-24T12:00:00Z",
    });
    expect(result.id).toBe("qa-123");
  });

  it("safeParseTaskQA returns null for invalid data", () => {
    expect(safeParseTaskQA({ invalid: true })).toBeNull();
  });
});

// ============================================================================
// JSON Roundtrip Tests
// ============================================================================

describe("JSON roundtrip", () => {
  it("AcceptanceCriteria survives JSON serialization", () => {
    const original: AcceptanceCriteria = {
      acceptance_criteria: [
        { id: "AC1", description: "Test visual", testable: true, type: "visual" },
        { id: "AC2", description: "Test behavior", testable: false, type: "behavior" },
      ],
    };
    const json = JSON.stringify(original);
    const parsed = AcceptanceCriteriaSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });

  it("QATestSteps survives JSON serialization", () => {
    const original: QATestSteps = {
      qa_steps: [
        {
          id: "QA1",
          criteria_id: "AC1",
          description: "Step 1",
          commands: ["cmd1", "cmd2"],
          expected: "Result",
        },
      ],
    };
    const json = JSON.stringify(original);
    const parsed = QATestStepsSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });

  it("QAResults survives JSON serialization", () => {
    const original: QAResults = {
      task_id: "task-123",
      overall_status: "failed",
      total_steps: 3,
      passed_steps: 2,
      failed_steps: 1,
      steps: [
        { step_id: "QA1", status: "passed", screenshot: "ss1.png" },
        { step_id: "QA2", status: "passed" },
        { step_id: "QA3", status: "failed", error: "Element not found" },
      ],
    };
    const json = JSON.stringify(original);
    const parsed = QAResultsSchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });

  it("TaskQA survives JSON serialization", () => {
    const original: TaskQA = {
      id: "qa-123",
      task_id: "task-123",
      acceptance_criteria: {
        acceptance_criteria: [
          { id: "AC1", description: "Test", testable: true, type: "visual" },
        ],
      },
      qa_test_steps: {
        qa_steps: [
          { id: "QA1", criteria_id: "AC1", description: "Step", commands: [], expected: "OK" },
        ],
      },
      prep_agent_id: "agent-1",
      prep_started_at: "2026-01-24T12:00:00Z",
      prep_completed_at: "2026-01-24T12:05:00Z",
      actual_implementation: "Implementation summary",
      refined_test_steps: {
        qa_steps: [
          { id: "QA1", criteria_id: "AC1", description: "Refined", commands: ["cmd"], expected: "OK" },
        ],
      },
      refinement_agent_id: "agent-2",
      refinement_completed_at: "2026-01-24T12:10:00Z",
      test_results: {
        task_id: "task-123",
        overall_status: "passed",
        total_steps: 1,
        passed_steps: 1,
        failed_steps: 0,
        steps: [{ step_id: "QA1", status: "passed" }],
      },
      screenshots: ["ss1.png"],
      test_agent_id: "agent-3",
      test_completed_at: "2026-01-24T12:15:00Z",
      created_at: "2026-01-24T12:00:00Z",
    };
    const json = JSON.stringify(original);
    const parsed = TaskQASchema.parse(JSON.parse(json));
    expect(parsed).toEqual(original);
  });
});
