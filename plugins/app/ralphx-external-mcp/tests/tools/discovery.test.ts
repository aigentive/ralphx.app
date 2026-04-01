import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ApiKeyContext } from "../../src/types.js";

// ─── Mock BackendClient ──────────────────────────────────────────────────────

const mockGet = vi.fn();
const mockPost = vi.fn();

vi.mock("../../src/backend-client.js", () => ({
  getBackendClient: () => ({
    get: mockGet,
    post: mockPost,
  }),
  BackendError: class BackendError extends Error {
    statusCode: number;
    constructor(statusCode: number, message: string) {
      super(message);
      this.name = "BackendError";
      this.statusCode = statusCode;
    }
  },
}));

// Import after mocks are set up
const { handleListProjects, handleGetProjectStatus, handleGetPipelineOverview } =
  await import("../../src/tools/discovery.js");

// ─── Test fixtures ────────────────────────────────────────────────────────────

const testContext: ApiKeyContext = {
  keyId: "key-test",
  projectIds: ["proj-alpha", "proj-beta"],
  permissions: 3,
};

const unrestrictedContext: ApiKeyContext = {
  keyId: "key-admin",
  projectIds: [],
  permissions: 7,
};

// ─── v1_list_projects ─────────────────────────────────────────────────────────

describe("handleListProjects", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns formatted project list from backend", async () => {
    const backendPayload = {
      projects: [
        { id: "proj-alpha", name: "Alpha", task_count: 5, created_at: "2024-01-01T00:00:00Z" },
        { id: "proj-beta", name: "Beta", task_count: 2, created_at: "2024-01-02T00:00:00Z" },
      ],
    };
    mockGet.mockResolvedValueOnce({ status: 200, body: backendPayload });

    const result = await handleListProjects({}, testContext);
    const parsed = JSON.parse(result);

    expect(parsed.projects).toHaveLength(2);
    expect(parsed.projects[0].id).toBe("proj-alpha");
    expect(parsed.projects[0].name).toBe("Alpha");
    expect(mockGet).toHaveBeenCalledWith("/api/external/projects", testContext);
  });

  it("handles backend error gracefully", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(500, "Internal Server Error"));

    const result = await handleListProjects({}, testContext);
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(500);
  });

  it("handles unexpected errors gracefully", async () => {
    mockGet.mockRejectedValueOnce(new Error("Network failure"));

    const result = await handleListProjects({}, testContext);
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("unexpected_error");
    expect(parsed.message).toContain("Network failure");
  });

  it("passes context to backend client", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: { projects: [] } });

    await handleListProjects({}, unrestrictedContext);

    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/projects",
      unrestrictedContext
    );
  });
});

// ─── v1_get_project_status ────────────────────────────────────────────────────

describe("handleGetProjectStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns project status with task counts", async () => {
    const backendPayload = {
      project: { id: "proj-alpha", name: "Alpha" },
      task_counts: {
        total: 10,
        backlog: 3,
        ready: 2,
        executing: 1,
        reviewing: 1,
        merging: 0,
        merged: 2,
        cancelled: 1,
        stopped: 0,
        blocked: 0,
        pending_review: 0,
        pending_merge: 0,
        other: 0,
      },
      running_agents: 1,
    };
    mockGet.mockResolvedValueOnce({ status: 200, body: backendPayload });

    const result = await handleGetProjectStatus(
      { project_id: "proj-alpha" },
      testContext
    );
    const parsed = JSON.parse(result);

    expect(parsed.project.id).toBe("proj-alpha");
    expect(parsed.task_counts.total).toBe(10);
    expect(parsed.running_agents).toBe(1);
    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/project/proj-alpha/status",
      testContext
    );
  });

  it("returns missing_argument error when project_id not provided", async () => {
    const result = await handleGetProjectStatus({}, testContext);
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("missing_argument");
    expect(parsed.message).toContain("project_id");
    expect(mockGet).not.toHaveBeenCalled();
  });

  it("handles 403 scope violation from backend", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(403, "Forbidden"));

    const result = await handleGetProjectStatus(
      { project_id: "proj-secret" },
      testContext
    );
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(403);
  });

  it("URL-encodes project_id in path", async () => {
    mockGet.mockResolvedValueOnce({ status: 200, body: {} });

    await handleGetProjectStatus(
      { project_id: "proj with spaces" },
      testContext
    );

    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/project/proj%20with%20spaces/status",
      testContext
    );
  });
});

// ─── v1_get_pipeline_overview ─────────────────────────────────────────────────

describe("handleGetPipelineOverview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns pipeline stage counts", async () => {
    const backendPayload = {
      project_id: "proj-alpha",
      stages: {
        pending: 5,
        executing: 2,
        reviewing: 1,
        pending_merge: 0,
        merging: 0,
        merged: 10,
        blocked: 0,
        cancelled: 1,
        stopped: 0,
      },
    };
    mockGet.mockResolvedValueOnce({ status: 200, body: backendPayload });

    const result = await handleGetPipelineOverview(
      { project_id: "proj-alpha" },
      testContext
    );
    const parsed = JSON.parse(result);

    expect(parsed.project_id).toBe("proj-alpha");
    expect(parsed.stages.pending).toBe(5);
    expect(parsed.stages.merged).toBe(10);
    expect(mockGet).toHaveBeenCalledWith(
      "/api/external/pipeline/proj-alpha",
      testContext
    );
  });

  it("returns missing_argument error when project_id not provided", async () => {
    const result = await handleGetPipelineOverview({}, testContext);
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("missing_argument");
    expect(mockGet).not.toHaveBeenCalled();
  });

  it("handles backend error", async () => {
    const { BackendError } = await import("../../src/backend-client.js");
    mockGet.mockRejectedValueOnce(new BackendError(404, "Not found"));

    const result = await handleGetPipelineOverview(
      { project_id: "proj-gone" },
      testContext
    );
    const parsed = JSON.parse(result);

    expect(parsed.error).toBe("backend_error");
    expect(parsed.status).toBe(404);
  });
});
