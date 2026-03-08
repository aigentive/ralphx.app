/**
 * Tests for task-metrics API module
 */

import { describe, it, expect, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  deriveComplexityTier,
  TaskMetricsSchema,
  getTaskMetrics,
  type TaskMetrics,
} from "./task-metrics";

const makeMetrics = (overrides: Partial<TaskMetrics> = {}): TaskMetrics => ({
  stepCount: 3,
  completedStepCount: 2,
  reviewCount: 1,
  approvedReviewCount: 1,
  executionMinutes: 5,
  totalAgeHours: 2,
  ...overrides,
});

describe("deriveComplexityTier", () => {
  it("returns Simple when executionMinutes < 10 and stepCount <= 5", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 5, stepCount: 3 }))).toBe("Simple");
  });

  it("returns Simple at boundary: executionMinutes = 9, stepCount = 5", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 9, stepCount: 5 }))).toBe("Simple");
  });

  it("returns Medium when executionMinutes is 10-29 and stepCount is 6-9", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 15, stepCount: 7 }))).toBe("Medium");
  });

  it("returns Medium when executionMinutes is 10 and stepCount <= 5", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 10, stepCount: 3 }))).toBe("Medium");
  });

  it("returns Medium when executionMinutes < 10 and stepCount is 6", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 5, stepCount: 6 }))).toBe("Medium");
  });

  it("returns Complex when executionMinutes >= 30", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 30, stepCount: 3 }))).toBe("Complex");
  });

  it("returns Complex when executionMinutes > 30", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 60, stepCount: 3 }))).toBe("Complex");
  });

  it("returns Complex when stepCount >= 10", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 5, stepCount: 10 }))).toBe("Complex");
  });

  it("returns Complex when both executionMinutes >= 30 AND stepCount >= 10", () => {
    expect(deriveComplexityTier(makeMetrics({ executionMinutes: 45, stepCount: 15 }))).toBe("Complex");
  });
});

describe("TaskMetricsSchema", () => {
  it("parses valid metrics data", () => {
    const raw = {
      stepCount: 5,
      completedStepCount: 3,
      reviewCount: 2,
      approvedReviewCount: 1,
      executionMinutes: 12.5,
      totalAgeHours: 4.2,
    };
    const parsed = TaskMetricsSchema.parse(raw);
    expect(parsed.stepCount).toBe(5);
    expect(parsed.completedStepCount).toBe(3);
    expect(parsed.reviewCount).toBe(2);
    expect(parsed.approvedReviewCount).toBe(1);
    expect(parsed.executionMinutes).toBe(12.5);
    expect(parsed.totalAgeHours).toBe(4.2);
  });

  it("rejects data missing required fields", () => {
    expect(() => TaskMetricsSchema.parse({ stepCount: 5 })).toThrow();
  });

  it("rejects non-numeric stepCount", () => {
    const raw = {
      stepCount: "five",
      completedStepCount: 3,
      reviewCount: 2,
      approvedReviewCount: 1,
      executionMinutes: 10,
      totalAgeHours: 1,
    };
    expect(() => TaskMetricsSchema.parse(raw)).toThrow();
  });
});

describe("getTaskMetrics", () => {
  it("calls invoke with correct command and taskId", async () => {
    const mockData = {
      stepCount: 4,
      completedStepCount: 4,
      reviewCount: 1,
      approvedReviewCount: 1,
      executionMinutes: 8,
      totalAgeHours: 1,
    };
    vi.mocked(invoke).mockResolvedValueOnce(mockData);

    const result = await getTaskMetrics("task-123");

    expect(invoke).toHaveBeenCalledWith("get_task_metrics", { taskId: "task-123" });
    expect(result.stepCount).toBe(4);
    expect(result.executionMinutes).toBe(8);
  });
});
