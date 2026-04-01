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

const { startIdeation } = await import(
  "../../src/composites/start-ideation.js"
);
const { BackendError } = await import("../../src/backend-client.js");

// ─── Test fixtures ────────────────────────────────────────────────────────────

const testContext: ApiKeyContext = {
  keyId: "key-test",
  projectIds: ["proj-alpha"],
  permissions: 3,
};

// ─── startIdeation ────────────────────────────────────────────────────────────

describe("startIdeation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("creates an ideation session and returns session_id + started status", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: {
        session_id: "sess-new-001",
        status: "ideating",
      },
    });

    const result = await startIdeation(
      { projectId: "proj-alpha", prompt: "Build auth system" },
      testContext
    );

    expect(result.sessionId).toBe("sess-new-001");
    expect(result.status).toBe("started");
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/start_ideation",
      testContext,
      {
        project_id: "proj-alpha",
        prompt: "Build auth system",
      }
    );
  });

  it("throws BackendError when backend returns non-2xx status", async () => {
    mockPost.mockResolvedValueOnce({
      status: 429,
      body: {},
    });

    await expect(
      startIdeation({ projectId: "proj-alpha", prompt: "test" }, testContext)
    ).rejects.toThrow(BackendError);
  });

  it("throws Error when backend returns no session_id", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { status: "ideating" }, // missing session_id
    });

    await expect(
      startIdeation({ projectId: "proj-alpha", prompt: "test" }, testContext)
    ).rejects.toThrow("no session_id");
  });

  it("propagates network errors from backend client", async () => {
    mockPost.mockRejectedValueOnce(new BackendError(503, "Backend unreachable"));

    await expect(
      startIdeation({ projectId: "proj-alpha", prompt: "test" }, testContext)
    ).rejects.toThrow(BackendError);
  });

  it("passes project scope from context to backend", async () => {
    mockPost.mockResolvedValueOnce({
      status: 200,
      body: { session_id: "sess-scoped", status: "ideating" },
    });

    const scopedContext: ApiKeyContext = {
      keyId: "key-scoped",
      projectIds: ["proj-alpha"],
      permissions: 3,
    };

    await startIdeation(
      { projectId: "proj-alpha", prompt: "scope test" },
      scopedContext
    );

    // The backend client (mocked here) receives the context — scope injection
    // is handled inside BackendClient.request() via the X-RalphX-Project-Scope header
    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/start_ideation",
      scopedContext,
      expect.objectContaining({ project_id: "proj-alpha" })
    );
  });
});
