/**
 * Tests for pipeline supervision tool handlers — Flow 3 (Phase 5)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ApiKeyContext } from "../../src/types.js";
import { BackendError } from "../../src/backend-client.js";

// Mock backend client
const mockGet = vi.fn();
const mockPost = vi.fn();

vi.mock("../../src/backend-client.js", () => ({
  getBackendClient: () => ({ get: mockGet, post: mockPost }),
  BackendError: class BackendError extends Error {
    statusCode: number;
    constructor(statusCode: number, message: string) {
      super(message);
      this.statusCode = statusCode;
    }
  },
}));

// Mock resume-scheduling composite
const mockResumeScheduling = vi.fn();
vi.mock("../../src/composites/resume-scheduling.js", () => ({
  resumeScheduling: (input: unknown, ctx: unknown) => mockResumeScheduling(input, ctx),
}));

import {
  handleGetTaskDetail,
  handleGetTaskDiff,
  handleGetReviewSummary,
  handleApproveReview,
  handleRequestChanges,
  handleGetMergePipeline,
  handleResolveEscalation,
  handlePauseTask,
  handleCancelTask,
  handleRetryTask,
  handleResumeScheduling,
} from "../../src/tools/pipeline.js";

const testContext: ApiKeyContext = {
  keyId: "key-test-001",
  projectIds: ["proj-alpha"],
  permissions: 3,
};

describe("handleGetTaskDetail", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns task details", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { id: "task-001", title: "My Task", steps: [] },
    });
    const result = await handleGetTaskDetail({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.id).toBe("task-001");
    expect(mockGet).toHaveBeenCalledWith("/api/external/task/task-001", testContext);
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleGetTaskDetail({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockGet).not.toHaveBeenCalled();
  });

  it("handles backend 404", async () => {
    mockGet.mockRejectedValueOnce(new BackendError(404, "Task not found"));
    const result = await handleGetTaskDetail({ task_id: "nonexistent" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(404);
  });
});

describe("handleGetTaskDiff", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns diff stats", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { task_id: "task-001", files_changed: 3, insertions: 50, deletions: 10 },
    });
    const result = await handleGetTaskDiff({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.files_changed).toBe(3);
    expect(mockGet).toHaveBeenCalledWith("/api/external/task/task-001/diff", testContext);
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleGetTaskDiff({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("handles backend error", async () => {
    mockGet.mockRejectedValueOnce(new BackendError(500, "Internal error"));
    const result = await handleGetTaskDiff({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(500);
  });
});

describe("handleGetReviewSummary", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns review summary", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { task_id: "task-001", review_notes: [], revision_count: 0 },
    });
    const result = await handleGetReviewSummary({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.revision_count).toBe(0);
    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/task/task-001/review_summary",
      testContext
    );
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleGetReviewSummary({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });
});

describe("handleApproveReview", () => {
  beforeEach(() => vi.clearAllMocks());

  it("approves review and returns new status", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "approved" },
    });
    const result = await handleApproveReview({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/review_action",
      testContext,
      { task_id: "task-001", action: "approve_review" }
    );
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleApproveReview({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockPost).not.toHaveBeenCalled();
  });

  it("handles scope violation", async () => {
    mockPost.mockRejectedValueOnce(new BackendError(403, "API key does not have access"));
    const result = await handleApproveReview({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(403);
  });

  it("handles invalid state transition", async () => {
    mockPost.mockRejectedValueOnce(new BackendError(422, "Cannot transition from current state"));
    const result = await handleApproveReview({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(422);
  });
});

describe("handleRequestChanges", () => {
  beforeEach(() => vi.clearAllMocks());

  it("requests changes with feedback", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "revision_needed" },
    });
    const result = await handleRequestChanges(
      { task_id: "task-001", feedback: "Fix the auth logic" },
      testContext
    );
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/review_action",
      testContext,
      { task_id: "task-001", action: "request_changes", feedback: "Fix the auth logic" }
    );
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleRequestChanges({ feedback: "something" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns missing_argument when feedback not provided", async () => {
    const result = await handleRequestChanges({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockPost).not.toHaveBeenCalled();
  });
});

describe("handleGetMergePipeline", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns merge pipeline tasks", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { project_id: "proj-alpha", tasks: [{ id: "t1", status: "pending_merge" }] },
    });
    const result = await handleGetMergePipeline({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.project_id).toBe("proj-alpha");
    expect(parsed.tasks).toHaveLength(1);
    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/merge_pipeline/proj-alpha",
      testContext
    );
  });

  it("returns missing_argument when project_id not provided", async () => {
    const result = await handleGetMergePipeline({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });
});

describe("handleResolveEscalation", () => {
  beforeEach(() => vi.clearAllMocks());

  it("resolves escalation with approve", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "approved" },
    });
    const result = await handleResolveEscalation(
      { task_id: "task-001", resolution: "approve" },
      testContext
    );
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/review_action",
      testContext,
      { task_id: "task-001", action: "resolve_escalation", resolution: "approve", feedback: undefined }
    );
  });

  it("resolves escalation with request_changes", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "revision_needed" },
    });
    const result = await handleResolveEscalation(
      { task_id: "task-001", resolution: "request_changes", feedback: "Please fix X" },
      testContext
    );
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleResolveEscalation({ resolution: "approve" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns missing_argument when resolution not provided", async () => {
    const result = await handleResolveEscalation({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns invalid_argument for bad resolution value", async () => {
    const result = await handleResolveEscalation(
      { task_id: "task-001", resolution: "ignore" },
      testContext
    );
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("invalid_argument");
    expect(mockPost).not.toHaveBeenCalled();
  });
});

describe("handlePauseTask", () => {
  beforeEach(() => vi.clearAllMocks());

  it("pauses a task", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "paused" },
    });
    const result = await handlePauseTask({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/task_transition",
      testContext,
      { task_id: "task-001", action: "pause" }
    );
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handlePauseTask({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockPost).not.toHaveBeenCalled();
  });

  it("handles backend error", async () => {
    mockPost.mockRejectedValueOnce(new BackendError(422, "Invalid state transition"));
    const result = await handlePauseTask({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(422);
  });
});

describe("handleCancelTask", () => {
  beforeEach(() => vi.clearAllMocks());

  it("cancels a task", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "cancelled" },
    });
    const result = await handleCancelTask({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/task_transition",
      testContext,
      { task_id: "task-001", action: "cancel" }
    );
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleCancelTask({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });
});

describe("handleRetryTask", () => {
  beforeEach(() => vi.clearAllMocks());

  it("retries a stopped task", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { success: true, task_id: "task-001", new_status: "ready" },
    });
    const result = await handleRetryTask({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/task_transition",
      testContext,
      { task_id: "task-001", action: "retry" }
    );
  });

  it("handles bad request for non-terminal task", async () => {
    mockPost.mockRejectedValueOnce(new BackendError(400, "Task must be in terminal state to retry"));
    const result = await handleRetryTask({ task_id: "task-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(400);
  });

  it("returns missing_argument when task_id not provided", async () => {
    const result = await handleRetryTask({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });
});

describe("handleResumeScheduling", () => {
  beforeEach(() => vi.clearAllMocks());

  it("delegates to resumeScheduling composite", async () => {
    mockResumeScheduling.mockResolvedValueOnce({
      success: true,
      taskIds: ["task-abc"],
      message: "Scheduling resumed successfully. 1 task(s) scheduled.",
    });
    const result = await handleResumeScheduling({ session_id: "sess-001" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(parsed.taskIds).toContain("task-abc");
    expect(mockResumeScheduling).toHaveBeenCalledWith({ sessionId: "sess-001" }, testContext);
  });

  it("returns missing_argument when session_id not provided", async () => {
    const result = await handleResumeScheduling({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockResumeScheduling).not.toHaveBeenCalled();
  });

  it("handles composite failure", async () => {
    mockResumeScheduling.mockResolvedValueOnce({
      success: false,
      taskIds: [],
      message: "Backend error loading session: Not Found",
    });
    const result = await handleResumeScheduling({ session_id: "sess-bad" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(false);
  });
});
