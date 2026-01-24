import { describe, it, expect } from "vitest";
import {
  TaskEventSchema,
  type TaskEvent,
  AgentMessageEventSchema,
  TaskStatusEventSchema,
  SupervisorAlertEventSchema,
  ReviewEventSchema,
  FileChangeEventSchema,
  ProgressEventSchema,
  QAPrepEventSchema,
  QATestEventSchema,
} from "./events";

describe("TaskEventSchema", () => {
  describe("created event", () => {
    it("validates a valid created event", () => {
      const event = {
        type: "created",
        task: {
          id: "550e8400-e29b-41d4-a716-446655440000",
          projectId: "550e8400-e29b-41d4-a716-446655440001",
          category: "feature",
          title: "Test task",
          description: null,
          internalStatus: "backlog",
          priority: 0,
          createdAt: "2026-01-24T12:00:00Z",
          updatedAt: "2026-01-24T12:00:00Z",
          startedAt: null,
          completedAt: null,
        },
      };

      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.type).toBe("created");
      }
    });

    it("rejects created event without task", () => {
      const event = { type: "created" };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });
  });

  describe("updated event", () => {
    it("validates a valid updated event", () => {
      const event = {
        type: "updated",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
        changes: { title: "Updated title" },
      };

      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.type).toBe("updated");
      }
    });

    it("rejects updated event without taskId", () => {
      const event = { type: "updated", changes: { title: "Test" } };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it("rejects updated event with invalid taskId", () => {
      const event = {
        type: "updated",
        taskId: "invalid-uuid",
        changes: { title: "Test" },
      };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });
  });

  describe("deleted event", () => {
    it("validates a valid deleted event", () => {
      const event = {
        type: "deleted",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
      };

      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.type).toBe("deleted");
      }
    });

    it("rejects deleted event without taskId", () => {
      const event = { type: "deleted" };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });
  });

  describe("status_changed event", () => {
    it("validates a valid status_changed event", () => {
      const event = {
        type: "status_changed",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
        from: "backlog",
        to: "ready",
        changedBy: "user",
      };

      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.type).toBe("status_changed");
      }
    });

    it("rejects status_changed event with invalid status", () => {
      const event = {
        type: "status_changed",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
        from: "invalid_status",
        to: "ready",
        changedBy: "user",
      };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it("rejects status_changed event with invalid changedBy", () => {
      const event = {
        type: "status_changed",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
        from: "backlog",
        to: "ready",
        changedBy: "invalid",
      };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });
  });

  describe("invalid events", () => {
    it("rejects unknown event type", () => {
      const event = { type: "unknown", taskId: "123" };
      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(false);
    });

    it("rejects empty object", () => {
      const result = TaskEventSchema.safeParse({});
      expect(result.success).toBe(false);
    });

    it("rejects null", () => {
      const result = TaskEventSchema.safeParse(null);
      expect(result.success).toBe(false);
    });
  });

  describe("type inference", () => {
    it("correctly infers TaskEvent type", () => {
      const event: TaskEvent = {
        type: "created",
        task: {
          id: "550e8400-e29b-41d4-a716-446655440000",
          projectId: "550e8400-e29b-41d4-a716-446655440001",
          category: "feature",
          title: "Test",
          description: null,
          internalStatus: "backlog",
          priority: 0,
          createdAt: "2026-01-24T12:00:00Z",
          updatedAt: "2026-01-24T12:00:00Z",
          startedAt: null,
          completedAt: null,
        },
      };
      expect(event.type).toBe("created");
    });
  });
});

describe("AgentMessageEventSchema", () => {
  it("validates a valid agent message event", () => {
    const event = {
      taskId: "task-123",
      type: "thinking",
      content: "Processing request...",
      timestamp: Date.now(),
    };

    const result = AgentMessageEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates with optional metadata", () => {
    const event = {
      taskId: "task-123",
      type: "tool_call",
      content: "read_file",
      timestamp: Date.now(),
      metadata: { file: "test.ts" },
    };

    const result = AgentMessageEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid message type", () => {
    const event = {
      taskId: "task-123",
      type: "invalid_type",
      content: "test",
      timestamp: Date.now(),
    };

    const result = AgentMessageEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});

describe("TaskStatusEventSchema", () => {
  it("validates a valid status event", () => {
    const event = {
      taskId: "task-123",
      fromStatus: "backlog",
      toStatus: "ready",
      changedBy: "user",
    };

    const result = TaskStatusEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates with null fromStatus", () => {
    const event = {
      taskId: "task-123",
      fromStatus: null,
      toStatus: "backlog",
      changedBy: "system",
    };

    const result = TaskStatusEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates with optional reason", () => {
    const event = {
      taskId: "task-123",
      fromStatus: "executing",
      toStatus: "blocked",
      changedBy: "ai_worker",
      reason: "Waiting for user input",
    };

    const result = TaskStatusEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });
});

describe("SupervisorAlertEventSchema", () => {
  it("validates a valid supervisor alert", () => {
    const event = {
      taskId: "task-123",
      severity: "high",
      type: "loop_detected",
      message: "Agent is repeating the same action",
    };

    const result = SupervisorAlertEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates with optional suggestedAction", () => {
    const event = {
      taskId: "task-123",
      severity: "critical",
      type: "stuck",
      message: "No progress for 5 minutes",
      suggestedAction: "Restart the agent",
    };

    const result = SupervisorAlertEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid severity", () => {
    const event = {
      taskId: "task-123",
      severity: "urgent", // not valid
      type: "error",
      message: "Test",
    };

    const result = SupervisorAlertEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});

describe("ReviewEventSchema", () => {
  it("validates a valid review event", () => {
    const event = {
      taskId: "task-123",
      reviewId: "review-456",
      type: "started",
    };

    const result = ReviewEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates with optional outcome", () => {
    const event = {
      taskId: "task-123",
      reviewId: "review-456",
      type: "completed",
      outcome: "approved",
    };

    const result = ReviewEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });
});

describe("FileChangeEventSchema", () => {
  it("validates a valid file change event", () => {
    const event = {
      projectId: "project-123",
      filePath: "/src/test.ts",
      changeType: "modified",
    };

    const result = FileChangeEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid change type", () => {
    const event = {
      projectId: "project-123",
      filePath: "/src/test.ts",
      changeType: "renamed", // not valid
    };

    const result = FileChangeEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});

describe("ProgressEventSchema", () => {
  it("validates a valid progress event", () => {
    const event = {
      taskId: "task-123",
      progress: 50,
      stage: "Running tests",
    };

    const result = ProgressEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates edge case progress values", () => {
    expect(ProgressEventSchema.safeParse({ taskId: "t", progress: 0, stage: "s" }).success).toBe(true);
    expect(ProgressEventSchema.safeParse({ taskId: "t", progress: 100, stage: "s" }).success).toBe(true);
  });
});

describe("QAPrepEventSchema", () => {
  it("validates a qa_prep_started event", () => {
    const event = {
      taskId: "task-123",
      type: "started",
      agentId: "agent-456",
    };

    const result = QAPrepEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates a qa_prep_completed event with counts", () => {
    const event = {
      taskId: "task-123",
      type: "completed",
      agentId: "agent-456",
      acceptanceCriteriaCount: 5,
      testStepsCount: 10,
    };

    const result = QAPrepEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates a qa_prep_failed event with error", () => {
    const event = {
      taskId: "task-123",
      type: "failed",
      error: "Failed to generate acceptance criteria",
    };

    const result = QAPrepEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid type", () => {
    const event = {
      taskId: "task-123",
      type: "running", // not valid
    };

    const result = QAPrepEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });

  it("rejects missing taskId", () => {
    const event = {
      type: "started",
    };

    const result = QAPrepEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});

describe("QATestEventSchema", () => {
  it("validates a qa_testing_started event", () => {
    const event = {
      taskId: "task-123",
      type: "started",
      agentId: "agent-789",
    };

    const result = QATestEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates a qa_passed event with step counts", () => {
    const event = {
      taskId: "task-123",
      type: "passed",
      agentId: "agent-789",
      totalSteps: 5,
      passedSteps: 5,
      failedSteps: 0,
    };

    const result = QATestEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("validates a qa_failed event with step counts and error", () => {
    const event = {
      taskId: "task-123",
      type: "failed",
      agentId: "agent-789",
      totalSteps: 5,
      passedSteps: 3,
      failedSteps: 2,
      error: "2 tests failed: visibility check, click handler",
    };

    const result = QATestEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects invalid type", () => {
    const event = {
      taskId: "task-123",
      type: "running", // not valid
    };

    const result = QATestEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });

  it("rejects negative step counts", () => {
    const event = {
      taskId: "task-123",
      type: "passed",
      totalSteps: -1,
    };

    const result = QATestEventSchema.safeParse(event);
    expect(result.success).toBe(false);
  });
});
