/**
 * useApiKeys fetcher tests — verify Tauri invoke is called with correct args.
 *
 * Tests the exported fetcher functions directly (no React hooks / QueryClient needed).
 * The global Tauri mock is set up in src/test/setup.ts.
 */

import { describe, it, expect, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  fetchKeys,
  fetchAuditLog,
  createKey,
  revokeKey,
  rotateKey,
  updateKeyProjects,
  updateKeyPermissions,
} from "./useApiKeys";

// ============================================================================
// fetchKeys
// ============================================================================

describe("fetchKeys", () => {
  it("calls list_api_keys with no args and parses the result", async () => {
    const mockKeys = [
      {
        id: "key-1",
        name: "My Key",
        keyPrefix: "rxk_live_abc",
        permissions: 3,
        createdAt: "2024-01-01T00:00:00Z",
        revokedAt: null,
        lastUsedAt: null,
        projectIds: ["proj-1"],
      },
    ];
    vi.mocked(invoke).mockResolvedValue(mockKeys);

    const result = await fetchKeys();

    expect(invoke).toHaveBeenCalledWith("list_api_keys");
    expect(result).toHaveLength(1);
    expect(result[0]?.id).toBe("key-1");
    expect(result[0]?.keyPrefix).toBe("rxk_live_abc");
  });

  it("returns empty array when backend returns empty list", async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    const result = await fetchKeys();

    expect(invoke).toHaveBeenCalledWith("list_api_keys");
    expect(result).toEqual([]);
  });

  it("throws ZodError when backend returns invalid data shape", async () => {
    vi.mocked(invoke).mockResolvedValue([{ bad: "data" }]);

    await expect(fetchKeys()).rejects.toThrow();
  });

  it("defaults projectIds to empty array when field is missing", async () => {
    const mockKey = {
      id: "key-1",
      name: "My Key",
      keyPrefix: "rxk_live_abc",
      permissions: 1,
      createdAt: "2024-01-01T00:00:00Z",
      revokedAt: null,
      lastUsedAt: null,
      // projectIds omitted — schema default([]) should apply
    };
    vi.mocked(invoke).mockResolvedValue([mockKey]);

    const result = await fetchKeys();

    expect(result[0]?.projectIds).toEqual([]);
  });
});

// ============================================================================
// fetchAuditLog
// ============================================================================

describe("fetchAuditLog", () => {
  it("calls get_api_key_audit_log with wrapped input and parses result", async () => {
    const mockEntries = [
      {
        id: 1,
        api_key_id: "key-1",
        tool_name: "list_tasks",
        project_id: "proj-1",
        success: true,
        latency_ms: 42,
        created_at: "2024-01-01T00:00:00Z",
      },
    ];
    vi.mocked(invoke).mockResolvedValue(mockEntries);

    const result = await fetchAuditLog("key-1");

    expect(invoke).toHaveBeenCalledWith("get_api_key_audit_log", {
      input: { id: "key-1" },
    });
    expect(result).toHaveLength(1);
    expect(result[0]?.tool_name).toBe("list_tasks");
    expect(result[0]?.success).toBe(true);
  });

  it("handles nullable fields in audit log entries", async () => {
    const mockEntries = [
      {
        id: 2,
        api_key_id: "key-1",
        tool_name: "create_task",
        project_id: null,
        success: false,
        latency_ms: null,
        created_at: "2024-01-02T00:00:00Z",
      },
    ];
    vi.mocked(invoke).mockResolvedValue(mockEntries);

    const result = await fetchAuditLog("key-1");

    expect(result[0]?.project_id).toBeNull();
    expect(result[0]?.latency_ms).toBeNull();
    expect(result[0]?.success).toBe(false);
  });

  it("returns empty array when no audit log entries exist", async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    const result = await fetchAuditLog("key-1");

    expect(result).toEqual([]);
  });
});

// ============================================================================
// createKey
// ============================================================================

describe("createKey", () => {
  it("calls create_api_key with wrapped input containing all fields", async () => {
    const mockResponse = {
      id: "key-new",
      name: "New Key",
      rawKey: "rxk_live_secret_abc123",
      keyPrefix: "rxk_live_abc",
      permissions: 3,
    };
    vi.mocked(invoke).mockResolvedValue(mockResponse);

    const result = await createKey({
      name: "New Key",
      projectIds: ["proj-1", "proj-2"],
      permissions: 3,
    });

    expect(invoke).toHaveBeenCalledWith("create_api_key", {
      input: {
        name: "New Key",
        projectIds: ["proj-1", "proj-2"],
        permissions: 3,
      },
    });
    expect(result.rawKey).toBe("rxk_live_secret_abc123");
    expect(result.id).toBe("key-new");
  });

  it("calls create_api_key with undefined permissions when not provided", async () => {
    const mockResponse = {
      id: "key-new",
      name: "Read-only Key",
      rawKey: "rxk_live_secret_xyz",
      keyPrefix: "rxk_live_xyz",
      permissions: 1,
    };
    vi.mocked(invoke).mockResolvedValue(mockResponse);

    await createKey({ name: "Read-only Key", projectIds: [] });

    expect(invoke).toHaveBeenCalledWith("create_api_key", {
      input: {
        name: "Read-only Key",
        projectIds: [],
        permissions: undefined,
      },
    });
  });

  it("parses and returns ApiKeyCreatedResponse schema", async () => {
    vi.mocked(invoke).mockResolvedValue({
      id: "key-1",
      name: "Test",
      rawKey: "secret",
      keyPrefix: "rxk_",
      permissions: 7,
    });

    const result = await createKey({ name: "Test", projectIds: [] });

    expect(result.permissions).toBe(7);
  });

  it("throws ZodError when backend response is missing rawKey", async () => {
    vi.mocked(invoke).mockResolvedValue({
      id: "key-1",
      name: "Test",
      keyPrefix: "rxk_",
      permissions: 1,
      // rawKey missing
    });

    await expect(createKey({ name: "Test", projectIds: [] })).rejects.toThrow();
  });
});

// ============================================================================
// revokeKey
// ============================================================================

describe("revokeKey", () => {
  it("calls revoke_api_key with wrapped input id", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await revokeKey("key-to-revoke");

    expect(invoke).toHaveBeenCalledWith("revoke_api_key", {
      input: { id: "key-to-revoke" },
    });
  });

  it("resolves without error on success", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await expect(revokeKey("key-123")).resolves.toBeUndefined();
  });
});

// ============================================================================
// rotateKey
// ============================================================================

describe("rotateKey", () => {
  it("calls rotate_api_key with wrapped input id", async () => {
    const mockResponse = {
      id: "key-1",
      name: "Rotated Key",
      rawKey: "rxk_live_new_secret",
      keyPrefix: "rxk_live_new",
      permissions: 3,
    };
    vi.mocked(invoke).mockResolvedValue(mockResponse);

    const result = await rotateKey("key-1");

    expect(invoke).toHaveBeenCalledWith("rotate_api_key", {
      input: { id: "key-1" },
    });
    expect(result.rawKey).toBe("rxk_live_new_secret");
  });

  it("parses and returns new ApiKeyCreatedResponse", async () => {
    vi.mocked(invoke).mockResolvedValue({
      id: "key-1",
      name: "My Key",
      rawKey: "new-raw-key",
      keyPrefix: "rxk_",
      permissions: 5,
    });

    const result = await rotateKey("key-1");

    expect(result.id).toBe("key-1");
    expect(result.permissions).toBe(5);
  });

  it("throws ZodError when backend response is malformed", async () => {
    vi.mocked(invoke).mockResolvedValue({ bad: "response" });

    await expect(rotateKey("key-1")).rejects.toThrow();
  });
});

// ============================================================================
// updateKeyProjects
// ============================================================================

describe("updateKeyProjects", () => {
  it("calls update_api_key_projects with wrapped input containing id and projectIds", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await updateKeyProjects("key-1", ["proj-a", "proj-b"]);

    expect(invoke).toHaveBeenCalledWith("update_api_key_projects", {
      input: { id: "key-1", projectIds: ["proj-a", "proj-b"] },
    });
  });

  it("passes empty array when clearing all project associations", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await updateKeyProjects("key-1", []);

    expect(invoke).toHaveBeenCalledWith("update_api_key_projects", {
      input: { id: "key-1", projectIds: [] },
    });
  });

  it("resolves without error on success", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await expect(updateKeyProjects("key-1", ["proj-1"])).resolves.toBeUndefined();
  });
});

// ============================================================================
// updateKeyPermissions
// ============================================================================

describe("updateKeyPermissions", () => {
  it("calls update_api_key_permissions with wrapped input containing id and permissions", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await updateKeyPermissions("key-1", 7);

    expect(invoke).toHaveBeenCalledWith("update_api_key_permissions", {
      input: { id: "key-1", permissions: 7 },
    });
  });

  it("passes read-only permissions bitmask (1)", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await updateKeyPermissions("key-1", 1);

    expect(invoke).toHaveBeenCalledWith("update_api_key_permissions", {
      input: { id: "key-1", permissions: 1 },
    });
  });

  it("resolves without error on success", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await expect(updateKeyPermissions("key-1", 3)).resolves.toBeUndefined();
  });
});
