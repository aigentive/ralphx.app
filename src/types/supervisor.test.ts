import { describe, it, expect } from "vitest";
import {
  SeveritySchema,
  SupervisorActionTypeSchema,
  SupervisorActionSchema,
  DetectionPatternSchema,
  ToolCallInfoSchema,
  ErrorInfoSchema,
  ProgressInfoSchema,
  SupervisorEventSchema,
  SupervisorAlertSchema,
  SupervisorConfigSchema,
  DetectionResultSchema,
  TaskMonitorStateSchema,
} from "./supervisor";

describe("SeveritySchema", () => {
  it("validates valid severities", () => {
    expect(SeveritySchema.safeParse("low").success).toBe(true);
    expect(SeveritySchema.safeParse("medium").success).toBe(true);
    expect(SeveritySchema.safeParse("high").success).toBe(true);
    expect(SeveritySchema.safeParse("critical").success).toBe(true);
  });

  it("rejects invalid severities", () => {
    expect(SeveritySchema.safeParse("invalid").success).toBe(false);
    expect(SeveritySchema.safeParse(123).success).toBe(false);
  });
});

describe("SupervisorActionTypeSchema", () => {
  it("validates valid action types", () => {
    expect(SupervisorActionTypeSchema.safeParse("log").success).toBe(true);
    expect(SupervisorActionTypeSchema.safeParse("inject_guidance").success).toBe(true);
    expect(SupervisorActionTypeSchema.safeParse("pause").success).toBe(true);
    expect(SupervisorActionTypeSchema.safeParse("kill").success).toBe(true);
  });

  it("rejects invalid action types", () => {
    expect(SupervisorActionTypeSchema.safeParse("restart").success).toBe(false);
  });
});

describe("SupervisorActionSchema", () => {
  it("validates a complete action", () => {
    const action = {
      type: "inject_guidance",
      severity: "medium",
      reason: "Loop detected",
      guidance: "Try a different approach",
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorActionSchema.safeParse(action);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("inject_guidance");
      expect(result.data.guidance).toBe("Try a different approach");
    }
  });

  it("validates action without optional guidance", () => {
    const action = {
      type: "pause",
      severity: "high",
      reason: "Task is stuck",
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorActionSchema.safeParse(action);
    expect(result.success).toBe(true);
  });
});

describe("DetectionPatternSchema", () => {
  it("validates all detection patterns", () => {
    const patterns = [
      "infinite_loop",
      "stuck",
      "repeating_error",
      "high_token_usage",
      "time_exceeded",
      "poor_task_definition",
    ];

    for (const pattern of patterns) {
      expect(DetectionPatternSchema.safeParse(pattern).success).toBe(true);
    }
  });
});

describe("ToolCallInfoSchema", () => {
  it("validates a successful tool call", () => {
    const info = {
      toolName: "Write",
      arguments: '{"path": "test.txt"}',
      timestamp: "2026-01-24T12:00:00Z",
      success: true,
    };

    const result = ToolCallInfoSchema.safeParse(info);
    expect(result.success).toBe(true);
  });

  it("validates a failed tool call with error", () => {
    const info = {
      toolName: "Write",
      arguments: '{"path": "test.txt"}',
      timestamp: "2026-01-24T12:00:00Z",
      success: false,
      error: "Permission denied",
    };

    const result = ToolCallInfoSchema.safeParse(info);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.error).toBe("Permission denied");
    }
  });
});

describe("ErrorInfoSchema", () => {
  it("validates an error info", () => {
    const info = {
      message: "File not found",
      source: "Read",
      recoverable: true,
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = ErrorInfoSchema.safeParse(info);
    expect(result.success).toBe(true);
  });
});

describe("ProgressInfoSchema", () => {
  it("validates progress info", () => {
    const info = {
      hasFileChanges: true,
      filesModified: 3,
      hasNewCommits: false,
      tokensUsed: 15000,
      elapsedSeconds: 180,
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = ProgressInfoSchema.safeParse(info);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.filesModified).toBe(3);
    }
  });
});

describe("SupervisorEventSchema", () => {
  it("validates TaskStart event", () => {
    const event = {
      type: "task_start",
      taskId: "task-123",
      agentRole: "worker",
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("task_start");
    }
  });

  it("validates ToolCall event", () => {
    const event = {
      type: "tool_call",
      taskId: "task-123",
      info: {
        toolName: "Write",
        arguments: "{}",
        timestamp: "2026-01-24T12:00:00Z",
        success: true,
      },
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates Error event", () => {
    const event = {
      type: "error",
      taskId: "task-123",
      info: {
        message: "Error message",
        source: "Write",
        recoverable: true,
        timestamp: "2026-01-24T12:00:00Z",
      },
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates ProgressTick event", () => {
    const event = {
      type: "progress_tick",
      taskId: "task-123",
      info: {
        hasFileChanges: false,
        filesModified: 0,
        hasNewCommits: false,
        tokensUsed: 5000,
        elapsedSeconds: 60,
        timestamp: "2026-01-24T12:00:00Z",
      },
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates TokenThreshold event", () => {
    const event = {
      type: "token_threshold",
      taskId: "task-123",
      tokensUsed: 60000,
      threshold: 50000,
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates TimeThreshold event", () => {
    const event = {
      type: "time_threshold",
      taskId: "task-123",
      elapsedMinutes: 15,
      thresholdMinutes: 10,
      timestamp: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid event type", () => {
    const event = {
      type: "invalid_type",
      taskId: "task-123",
    };

    const result = SupervisorEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});

describe("SupervisorAlertSchema", () => {
  it("validates a complete alert", () => {
    const alert = {
      id: "550e8400-e29b-41d4-a716-446655440000",
      taskId: "task-123",
      type: "loop_detected",
      severity: "high",
      pattern: "infinite_loop",
      message: "Same Write call detected 3 times",
      details: "File: test.txt",
      suggestedAction: "inject_guidance",
      acknowledged: false,
      createdAt: "2026-01-24T12:00:00Z",
    };

    const result = SupervisorAlertSchema.safeParse(alert);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("loop_detected");
      expect(result.data.acknowledged).toBe(false);
    }
  });

  it("validates acknowledged alert", () => {
    const alert = {
      id: "550e8400-e29b-41d4-a716-446655440000",
      taskId: "task-123",
      type: "stuck",
      severity: "medium",
      message: "No progress for 5 minutes",
      acknowledged: true,
      createdAt: "2026-01-24T12:00:00Z",
      acknowledgedAt: "2026-01-24T12:05:00Z",
    };

    const result = SupervisorAlertSchema.safeParse(alert);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.acknowledged).toBe(true);
      expect(result.data.acknowledgedAt).toBe("2026-01-24T12:05:00Z");
    }
  });
});

describe("SupervisorConfigSchema", () => {
  it("validates config with defaults", () => {
    const config = {};
    const result = SupervisorConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.loopDetectionThreshold).toBe(3);
      expect(result.data.stuckTimeoutMinutes).toBe(5);
      expect(result.data.tokenWarningThreshold).toBe(50000);
      expect(result.data.timeWarningMinutes).toBe(10);
      expect(result.data.errorRepeatThreshold).toBe(3);
    }
  });

  it("validates custom config", () => {
    const config = {
      loopDetectionThreshold: 5,
      stuckTimeoutMinutes: 10,
      tokenWarningThreshold: 100000,
      timeWarningMinutes: 30,
      errorRepeatThreshold: 5,
    };

    const result = SupervisorConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.loopDetectionThreshold).toBe(5);
    }
  });
});

describe("DetectionResultSchema", () => {
  it("validates no detection", () => {
    const result = {
      detected: false,
    };

    const parsed = DetectionResultSchema.safeParse(result);
    expect(parsed.success).toBe(true);
  });

  it("validates positive detection", () => {
    const result = {
      detected: true,
      pattern: "infinite_loop",
      severity: "high",
      message: "Loop detected: Write called 4 times",
      suggestedAction: "pause",
    };

    const parsed = DetectionResultSchema.safeParse(result);
    expect(parsed.success).toBe(true);
    if (parsed.success) {
      expect(parsed.data.pattern).toBe("infinite_loop");
    }
  });
});

describe("TaskMonitorStateSchema", () => {
  it("validates initial monitor state", () => {
    const state = {
      taskId: "task-123",
      agentRole: "worker",
      startedAt: "2026-01-24T12:00:00Z",
      toolCalls: [],
      errors: [],
      isPaused: false,
      isKilled: false,
    };

    const result = TaskMonitorStateSchema.safeParse(state);
    expect(result.success).toBe(true);
  });

  it("validates state with tool calls and errors", () => {
    const state = {
      taskId: "task-123",
      agentRole: "worker",
      startedAt: "2026-01-24T12:00:00Z",
      toolCalls: [
        {
          toolName: "Write",
          arguments: "{}",
          timestamp: "2026-01-24T12:01:00Z",
          success: true,
        },
      ],
      errors: [
        {
          message: "Warning",
          source: "Lint",
          recoverable: true,
          timestamp: "2026-01-24T12:02:00Z",
        },
      ],
      lastProgress: {
        hasFileChanges: true,
        filesModified: 1,
        hasNewCommits: false,
        tokensUsed: 5000,
        elapsedSeconds: 120,
        timestamp: "2026-01-24T12:02:00Z",
      },
      isPaused: false,
      isKilled: false,
    };

    const result = TaskMonitorStateSchema.safeParse(state);
    expect(result.success).toBe(true);
  });

  it("validates paused state", () => {
    const state = {
      taskId: "task-123",
      agentRole: "worker",
      startedAt: "2026-01-24T12:00:00Z",
      toolCalls: [],
      errors: [],
      isPaused: true,
      isKilled: false,
      pauseReason: "Loop detected",
    };

    const result = TaskMonitorStateSchema.safeParse(state);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.isPaused).toBe(true);
      expect(result.data.pauseReason).toBe("Loop detected");
    }
  });
});
