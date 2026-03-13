/**
 * Tests for project setup tool handler (v1_register_project) and
 * auth cache invalidation by key ID.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ApiKeyContext } from "../../src/types.js";
import { Permission } from "../../src/types.js";

// ============================================================================
// Mock: backend-client
// ============================================================================

const mockPost = vi.fn();

vi.mock("../../src/backend-client.js", () => ({
  getBackendClient: () => ({ post: mockPost }),
  BackendError: class BackendError extends Error {
    statusCode: number;
    constructor(statusCode: number, message: string) {
      super(message);
      this.statusCode = statusCode;
    }
  },
}));

// ============================================================================
// invalidateCacheByKeyId — auth.ts
// ============================================================================

// Import auth module (no mocking needed for this unit)
import {
  invalidateCacheByKeyId,
  clearAuthCache,
} from "../../src/auth.js";

// Access internal cache through validateKey + a workaround: seed via clearAuthCache + manual injection.
// Since we can't easily seed the cache from outside, we test the exported API surface:
// - clearAuthCache clears all entries
// - invalidateCacheByKeyId with a keyId that has no cache entries is a no-op (no throw)
// - invalidateCacheByKeyId with empty string is a no-op (null guard)

describe("invalidateCacheByKeyId", () => {
  beforeEach(() => {
    clearAuthCache();
  });

  it("does not throw when called with an empty string", () => {
    expect(() => invalidateCacheByKeyId("")).not.toThrow();
  });

  it("does not throw when called on an empty cache", () => {
    expect(() => invalidateCacheByKeyId("key-does-not-exist")).not.toThrow();
  });

  it("returns undefined (void function)", () => {
    const result = invalidateCacheByKeyId("key-abc");
    expect(result).toBeUndefined();
  });
});

// ============================================================================
// handleRegisterProject — tools/projects.ts
// ============================================================================

// Mock auth module so cache invalidation is observable
const mockInvalidateCacheByKeyId = vi.fn();
vi.mock("../../src/auth.js", () => ({
  invalidateCacheByKeyId: (keyId: string) => mockInvalidateCacheByKeyId(keyId),
  clearAuthCache: vi.fn(),
}));

import { handleRegisterProject } from "../../src/tools/projects.js";

const contextWithPermission: ApiKeyContext = {
  keyId: "key-test-123",
  projectIds: ["proj-alpha"],
  permissions: Permission.CREATE_PROJECT, // 8
};

const contextWithoutPermission: ApiKeyContext = {
  keyId: "key-test-no-perm",
  projectIds: [],
  permissions: Permission.READ, // 1
};

describe("handleRegisterProject", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns permission_denied when CREATE_PROJECT permission is missing", async () => {
    const result = await handleRegisterProject(
      { working_directory: "/home/user/myproject" },
      contextWithoutPermission
    );
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("permission_denied");
    expect(mockPost).not.toHaveBeenCalled();
  });

  it("returns missing_argument when working_directory is absent", async () => {
    const result = await handleRegisterProject({}, contextWithPermission);
    const parsed = JSON.parse(result);
    expect(parsed.error).toBe("missing_argument");
    expect(mockPost).not.toHaveBeenCalled();
  });

  it("calls backend POST with working_directory and name on success", async () => {
    const backendResponse = {
      status: 200,
      body: { id: "proj-new-1", name: "myproject", working_directory: "/home/user/myproject", created_at: "2026-01-01T00:00:00Z" },
    };
    mockPost.mockResolvedValueOnce(backendResponse);

    await handleRegisterProject(
      { working_directory: "/home/user/myproject", name: "myproject" },
      contextWithPermission
    );

    expect(mockPost).toHaveBeenCalledWith(
      "/api/external/projects",
      contextWithPermission,
      { working_directory: "/home/user/myproject", name: "myproject" }
    );
  });

  it("invalidates cache by keyId on successful registration (id in response)", async () => {
    const backendResponse = {
      status: 200,
      body: { id: "proj-new-2", name: "proj", working_directory: "/home/user/proj", created_at: "2026-01-01T00:00:00Z" },
    };
    mockPost.mockResolvedValueOnce(backendResponse);

    await handleRegisterProject(
      { working_directory: "/home/user/proj" },
      contextWithPermission
    );

    expect(mockInvalidateCacheByKeyId).toHaveBeenCalledWith("key-test-123");
  });

  it("does not invalidate cache when response has no id", async () => {
    const backendResponse = { status: 409, body: { error: "conflict" } };
    mockPost.mockResolvedValueOnce(backendResponse);

    await handleRegisterProject(
      { working_directory: "/home/user/proj" },
      contextWithPermission
    );

    expect(mockInvalidateCacheByKeyId).not.toHaveBeenCalled();
  });

  it("returns JSON-serialized response body", async () => {
    const body = { id: "proj-x", name: "X", working_directory: "/tmp/x", created_at: "2026-01-01T00:00:00Z" };
    mockPost.mockResolvedValueOnce({ status: 200, body });

    const result = await handleRegisterProject(
      { working_directory: "/tmp/x" },
      contextWithPermission
    );

    expect(JSON.parse(result)).toEqual(body);
  });
});
