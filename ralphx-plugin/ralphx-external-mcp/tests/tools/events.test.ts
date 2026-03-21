/**
 * Tests for event monitoring tool handlers — Flow 4 (Phase 6)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ApiKeyContext } from "../../src/types.js";

// Mock backend client
const mockGet = vi.fn();

vi.mock("../../src/backend-client.js", () => ({
  getBackendClient: () => ({ get: mockGet }),
  BackendError: class BackendError extends Error {
    statusCode: number;
    constructor(statusCode: number, message: string) {
      super(message);
      this.name = "BackendError";
      this.statusCode = statusCode;
    }
  },
}));

import {
  handleGetRecentEvents,
  handleSubscribeEvents,
  handleGetAttentionItems,
  handleGetExecutionCapacity,
} from "../../src/tools/events.js";

const testContext: ApiKeyContext = {
  keyId: "key-test-001",
  projectIds: ["proj-alpha"],
  permissions: 3,
};

const wildcardContext: ApiKeyContext = {
  keyId: "key-wildcard",
  projectIds: [],
  permissions: 3,
};

// ─── v1_get_recent_events ────────────────────────────────────────────────────

describe("handleGetRecentEvents", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns events list with cursor", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: {
        events: [{ id: 42, event_type: "task_status_changed", project_id: "proj-alpha", payload: {}, created_at: "2026-01-01T00:00:00Z" }],
        next_cursor: 42,
        has_more: false,
      },
    });
    const result = await handleGetRecentEvents({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.events).toHaveLength(1);
    expect(parsed.next_cursor).toBe(42);
    expect(parsed.has_more).toBe(false);
  });

  it("passes cursor param when provided", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", cursor: 10 }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.cursor).toBe("10");
  });

  it("omits cursor param when cursor is 0", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", cursor: 0 }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.cursor).toBeUndefined();
  });

  it("caps limit at 200", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", limit: 999 }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.limit).toBe("200");
  });

  it("defaults limit to 50", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha" }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.limit).toBe("50");
  });

  it("returns error when project_id missing", async () => {
    const result = await handleGetRecentEvents({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns scope_violation when project not in context", async () => {
    const result = await handleGetRecentEvents({ project_id: "proj-other" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("scope_violation");
  });

  it("allows any project when context.projectIds is empty (wildcard)", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    const result = await handleGetRecentEvents({ project_id: "proj-anything" }, wildcardContext);
    const parsed = JSON.parse(result);
    expect(parsed.events).toBeDefined();
  });

  it("returns backend_error on backend failure", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(500, "Internal server error"));
    const result = await handleGetRecentEvents({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(500);
  });

  it("accepts last_id as backward-compat alias for cursor", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", last_id: 99 }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.cursor).toBe("99");
  });

  it("prefers cursor over last_id when both provided", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", cursor: 5, last_id: 99 }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.cursor).toBe("5");
  });

  it("forwards event_type filter to backend", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha", event_type: "task:status_changed" }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.event_type).toBe("task:status_changed");
  });

  it("omits event_type param when not provided", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { events: [], next_cursor: null, has_more: false } });
    await handleGetRecentEvents({ project_id: "proj-alpha" }, testContext);
    const [, , params] = mockGet.mock.calls[0]!;
    expect(params.event_type).toBeUndefined();
  });
});

// ─── v1_subscribe_events ─────────────────────────────────────────────────────

describe("handleSubscribeEvents", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns events with subscription_hint", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: {
        events: [{ id: 7, event_type: "task_created", project_id: "proj-alpha", payload: {}, created_at: "2026-01-01T00:00:00Z" }],
        next_cursor: 7,
        has_more: false,
      },
    });
    const result = await handleSubscribeEvents({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.events).toHaveLength(1);
    expect(parsed.subscription_hint).toContain("v1_get_recent_events");
    expect(typeof parsed.next_cursor).toBe("number");
  });

  it("returns error when project_id missing", async () => {
    const result = await handleSubscribeEvents({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns scope_violation when project not in context", async () => {
    const result = await handleSubscribeEvents({ project_id: "proj-other" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("scope_violation");
  });

  it("computes next_cursor from last event id when backend omits next_cursor", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: {
        events: [
          { id: 3, event_type: "task_created", project_id: "proj-alpha", payload: {}, created_at: "" },
          { id: 5, event_type: "task_updated", project_id: "proj-alpha", payload: {}, created_at: "" },
        ],
        next_cursor: null,
        has_more: false,
      },
    });
    const result = await handleSubscribeEvents({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.next_cursor).toBe(5);
  });

  it("returns next_cursor 0 when no events and backend returns null", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { events: [], next_cursor: null, has_more: false },
    });
    const result = await handleSubscribeEvents({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.next_cursor).toBe(0);
  });
});

// ─── v1_get_attention_items ──────────────────────────────────────────────────

describe("handleGetAttentionItems", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns attention items", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: {
        escalated_reviews: [{ task_id: "task-1", title: "Fix Bug", status: "EscalatedReview", updated_at: "2026-01-01T00:00:00Z" }],
        failed_tasks: [],
        merge_conflicts: [],
      },
    });
    const result = await handleGetAttentionItems({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.escalated_reviews).toHaveLength(1);
    expect(parsed.failed_tasks).toHaveLength(0);
  });

  it("returns error when project_id missing", async () => {
    const result = await handleGetAttentionItems({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns scope_violation when project not in context", async () => {
    const result = await handleGetAttentionItems({ project_id: "proj-other" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("scope_violation");
  });

  it("returns empty arrays with note when endpoint returns 404", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(404, "Not Found"));
    const result = await handleGetAttentionItems({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.escalated_reviews).toEqual([]);
    expect(parsed.failed_tasks).toEqual([]);
    expect(parsed.merge_conflicts).toEqual([]);
    expect(parsed.note).toBeTruthy();
  });

  it("returns empty arrays with note when endpoint returns 501", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(501, "Not Implemented"));
    const result = await handleGetAttentionItems({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.escalated_reviews).toEqual([]);
  });

  it("returns backend_error on 500", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(500, "Server error"));
    const result = await handleGetAttentionItems({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
  });
});

// ─── v1_get_execution_capacity ───────────────────────────────────────────────

describe("handleGetExecutionCapacity", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns capacity info", async () => {
    mockGet.mockResolvedValueOnce({
      status: 200,
      body: { can_start: true, project_running: 2, project_queued: 1 },
    });
    const result = await handleGetExecutionCapacity({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.can_start).toBe(true);
    expect(parsed.project_running).toBe(2);
    expect(parsed.project_queued).toBe(1);
  });

  it("returns error when project_id missing", async () => {
    const result = await handleGetExecutionCapacity({}, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
  });

  it("returns scope_violation when project not in context", async () => {
    const result = await handleGetExecutionCapacity({ project_id: "proj-other" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("scope_violation");
  });

  it("returns default capacity with note when endpoint returns 404", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(404, "Not Found"));
    const result = await handleGetExecutionCapacity({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.can_start).toBe(true);
    expect(parsed.project_running).toBe(0);
    expect(parsed.project_queued).toBe(0);
    expect(parsed.note).toBeTruthy();
  });

  it("returns default capacity with note when endpoint returns 501", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(501, "Not Implemented"));
    const result = await handleGetExecutionCapacity({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.can_start).toBe(true);
  });

  it("returns backend_error on 500", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(500, "Server error"));
    const result = await handleGetExecutionCapacity({ project_id: "proj-alpha" }, testContext);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("backend_error");
  });

  it("calls correct endpoint path", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { can_start: false, project_running: 5, project_queued: 3 } });
    await handleGetExecutionCapacity({ project_id: "proj-alpha" }, testContext);
    const [path] = mockGet.mock.calls[0]!;
    expect(path).toContain("/api/external/execution_capacity/");
    expect(path).toContain("proj-alpha");
  });
});
