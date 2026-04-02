import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  TaskEventSchema,
  type TaskEvent,
  AgentMessageEventSchema,
  RecoveryPromptEventSchema,
  TaskStatusEventSchema,
  SupervisorAlertEventSchema,
  ReviewEventSchema,
  FileChangeEventSchema,
  ProgressEventSchema,
  QAPrepEventSchema,
  QATestEventSchema,
  PlanVerificationStatusChangedSchema,
} from "./events";

describe("TaskEventSchema", () => {
  describe("created event", () => {
    it("validates a valid created event", () => {
      const event = {
        type: "created",
        task: {
          id: "550e8400-e29b-41d4-a716-446655440000",
          project_id: "550e8400-e29b-41d4-a716-446655440001",
          category: "feature",
          title: "Test task",
          description: null,
          internal_status: "backlog",
          needs_review_point: false,
          priority: 0,
          created_at: "2026-01-24T12:00:00Z",
          updated_at: "2026-01-24T12:00:00Z",
          started_at: null,
          completed_at: null,
          archived_at: null,
          blocked_reason: null,
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

    it("validates status_changed event with auto changedBy", () => {
      const event = {
        type: "status_changed",
        taskId: "550e8400-e29b-41d4-a716-446655440000",
        from: "pending_review",
        to: "reviewing",
        changedBy: "auto",
      };

      const result = TaskEventSchema.safeParse(event);
      expect(result.success).toBe(true);
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

describe("RecoveryPromptEventSchema", () => {
  it("validates a valid recovery prompt", () => {
    const event = {
      taskId: "550e8400-e29b-41d4-a716-446655440000",
      status: "executing",
      contextType: "execution",
      reason: "Execution run missing but max concurrency is reached.",
      primaryAction: { id: "restart", label: "Restart" },
      secondaryAction: { id: "cancel", label: "Cancel" },
    };

    const result = RecoveryPromptEventSchema.safeParse(event);
    expect(result.success).toBe(true);
  });

  it("rejects an invalid recovery prompt", () => {
    const event = {
      taskId: "invalid",
      status: "executing",
      contextType: "execution",
      reason: "",
      primaryAction: { id: "restart", label: "Restart" },
      secondaryAction: { id: "cancel", label: "Cancel" },
    };

    const result = RecoveryPromptEventSchema.safeParse(event);
    expect(result.success).toBe(false);
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

// ============================================================================
// Contract test: PlanVerificationStatusChangedSchema vs Rust fixture
// ============================================================================

describe("PlanVerificationStatusChangedSchema — contract test", () => {
  const fixturePath = resolve(
    __dirname,
    "../../../src-tauri/tests/fixtures/verification_event.json"
  );

  it("parses the Rust-generated verification_event.json fixture", () => {
    const raw: unknown = JSON.parse(readFileSync(fixturePath, "utf-8"));
    const result = PlanVerificationStatusChangedSchema.safeParse(raw);
    expect(result.success, result.success ? "" : JSON.stringify((result as { error: unknown }).error)).toBe(true);
  });

  it("accepts fixture and exposes current_gaps array", () => {
    const raw: unknown = JSON.parse(readFileSync(fixturePath, "utf-8"));
    const result = PlanVerificationStatusChangedSchema.parse(raw);
    expect(result.generation).toBe(3);
    expect(result.current_gaps).toBeDefined();
    expect(Array.isArray(result.current_gaps)).toBe(true);
    expect(result.current_gaps!.length).toBeGreaterThan(0);
    expect(result.current_gaps![0]).toMatchObject({
      severity: expect.stringMatching(/^(critical|high|medium|low)$/),
      category: expect.any(String),
      description: expect.any(String),
    });
  });

  it("accepts fixture and exposes rounds array", () => {
    const raw: unknown = JSON.parse(readFileSync(fixturePath, "utf-8"));
    const result = PlanVerificationStatusChangedSchema.parse(raw);
    expect(result.rounds).toBeDefined();
    expect(Array.isArray(result.rounds)).toBe(true);
    expect(result.rounds!.length).toBeGreaterThan(0);
    expect(result.rounds![0]).toMatchObject({
      fingerprints: expect.any(Array),
      gap_score: expect.any(Number),
    });
  });

  it("is backward compatible — parses event without current_gaps and rounds", () => {
    const minimal = {
      session_id: "sess-001",
      status: "reviewing",
      in_progress: true,
    };
    const result = PlanVerificationStatusChangedSchema.safeParse(minimal);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.current_gaps).toBeUndefined();
      expect(result.data.rounds).toBeUndefined();
    }
  });

  it("accepts imported_verified status (defensive fix — backend skips emission currently)", () => {
    const result = PlanVerificationStatusChangedSchema.safeParse({
      session_id: "test",
      status: "imported_verified",
      in_progress: false,
    });
    expect(result.success).toBe(true);
  });

  it("accepts null numeric fields from reset/start events", () => {
    const raw = {
      session_id: "sess-001",
      status: "unverified",
      in_progress: false,
      generation: null,
      round: null,
      max_rounds: null,
      gap_score: null,
      current_gaps: [],
      rounds: [],
    };
    const result = PlanVerificationStatusChangedSchema.parse(raw);
    expect(result.generation).toBeNull();
    expect(result.round).toBeNull();
    expect(result.max_rounds).toBeNull();
    expect(result.gap_score).toBeNull();
  });
});
