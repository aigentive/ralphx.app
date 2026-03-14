/**
 * API Key types and Zod schema validation tests.
 */

import { describe, it, expect } from "vitest";
import {
  PERM_READ,
  PERM_WRITE,
  PERM_ADMIN,
  hasPermission,
  togglePermission,
  ApiKeySchema,
  AuditLogEntrySchema,
  ApiKeyCreatedResponseSchema,
  parseApiKey,
  safeParseApiKey,
  type ApiKey,
  type AuditLogEntry,
  type ApiKeyCreatedResponse,
} from "./api-key";

// ============================================================================
// Permission Bitmask Constants
// ============================================================================

describe("permission constants", () => {
  it("PERM_READ is 1", () => {
    expect(PERM_READ).toBe(1);
  });

  it("PERM_WRITE is 2", () => {
    expect(PERM_WRITE).toBe(2);
  });

  it("PERM_ADMIN is 4", () => {
    expect(PERM_ADMIN).toBe(4);
  });

  it("combined permissions sum correctly", () => {
    expect(PERM_READ | PERM_WRITE | PERM_ADMIN).toBe(7);
  });
});

// ============================================================================
// hasPermission
// ============================================================================

describe("hasPermission", () => {
  it("returns true when bit is set", () => {
    expect(hasPermission(3, PERM_READ)).toBe(true);
    expect(hasPermission(3, PERM_WRITE)).toBe(true);
  });

  it("returns false when bit is not set", () => {
    expect(hasPermission(3, PERM_ADMIN)).toBe(false);
  });

  it("returns true for full permissions (7) on every bit", () => {
    expect(hasPermission(7, PERM_READ)).toBe(true);
    expect(hasPermission(7, PERM_WRITE)).toBe(true);
    expect(hasPermission(7, PERM_ADMIN)).toBe(true);
  });

  it("returns false for no permissions (0)", () => {
    expect(hasPermission(0, PERM_READ)).toBe(false);
    expect(hasPermission(0, PERM_WRITE)).toBe(false);
    expect(hasPermission(0, PERM_ADMIN)).toBe(false);
  });
});

// ============================================================================
// togglePermission
// ============================================================================

describe("togglePermission", () => {
  it("sets a bit that is not set", () => {
    expect(togglePermission(0, PERM_READ)).toBe(1);
    expect(togglePermission(2, PERM_READ)).toBe(3);
  });

  it("clears a bit that is already set", () => {
    expect(togglePermission(7, PERM_ADMIN)).toBe(3);
    expect(togglePermission(3, PERM_WRITE)).toBe(1);
  });

  it("is idempotent when toggled twice", () => {
    const original = 5;
    const toggled = togglePermission(original, PERM_WRITE);
    const restored = togglePermission(toggled, PERM_WRITE);
    expect(restored).toBe(original);
  });
});

// ============================================================================
// ApiKeySchema
// ============================================================================

describe("ApiKeySchema", () => {
  const validKey = {
    id: "key-abc-123",
    name: "My Production Key",
    keyPrefix: "rxk_live_a3f2",
    permissions: 3,
    createdAt: "2024-01-01T00:00:00Z",
    revokedAt: null,
    lastUsedAt: null,
    projectIds: ["proj-1", "proj-2"],
  };

  it("validates a complete valid API key", () => {
    const result = ApiKeySchema.safeParse(validKey);
    expect(result.success).toBe(true);
  });

  it("validates key with all optional dates set", () => {
    const key = {
      ...validKey,
      revokedAt: "2024-06-01T00:00:00Z",
      lastUsedAt: "2024-05-31T12:00:00Z",
    };
    const result = ApiKeySchema.safeParse(key);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.revokedAt).toBe("2024-06-01T00:00:00Z");
      expect(result.data.lastUsedAt).toBe("2024-05-31T12:00:00Z");
    }
  });

  it("defaults projectIds to empty array when omitted", () => {
    const { projectIds: _projectIds, ...keyWithoutProjects } = validKey;
    const result = ApiKeySchema.safeParse(keyWithoutProjects);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.projectIds).toEqual([]);
    }
  });

  it("rejects key with empty id", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, id: "" });
    expect(result.success).toBe(false);
  });

  it("rejects key with empty name", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, name: "" });
    expect(result.success).toBe(false);
  });

  it("rejects key with empty keyPrefix", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, keyPrefix: "" });
    expect(result.success).toBe(false);
  });

  it("rejects key with permissions below 0", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, permissions: -1 });
    expect(result.success).toBe(false);
  });

  it("rejects key with permissions above 15", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, permissions: 16 });
    expect(result.success).toBe(false);
  });

  it("accepts all valid permission values (0-15)", () => {
    for (let perms = 0; perms <= 15; perms++) {
      const result = ApiKeySchema.safeParse({ ...validKey, permissions: perms });
      expect(result.success).toBe(true);
    }
  });

  it("rejects key with non-integer permissions", () => {
    const result = ApiKeySchema.safeParse({ ...validKey, permissions: 1.5 });
    expect(result.success).toBe(false);
  });

  it("rejects key missing required fields", () => {
    const { id: _id, ...keyWithoutId } = validKey;
    const result = ApiKeySchema.safeParse(keyWithoutId);
    expect(result.success).toBe(false);
  });
});

// ============================================================================
// AuditLogEntrySchema
// ============================================================================

describe("AuditLogEntrySchema", () => {
  const validEntry = {
    id: 1,
    api_key_id: "key-abc-123",
    tool_name: "list_tasks",
    project_id: "proj-1",
    success: true,
    latency_ms: 42,
    created_at: "2024-01-01T00:00:00Z",
  };

  it("validates a complete audit log entry", () => {
    const result = AuditLogEntrySchema.safeParse(validEntry);
    expect(result.success).toBe(true);
  });

  it("validates entry with null project_id", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, project_id: null });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.project_id).toBeNull();
    }
  });

  it("validates entry with null latency_ms", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, latency_ms: null });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.latency_ms).toBeNull();
    }
  });

  it("validates entry with success=false", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, success: false });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.success).toBe(false);
    }
  });

  it("uses snake_case field names (not camelCase)", () => {
    const result = AuditLogEntrySchema.safeParse(validEntry);
    expect(result.success).toBe(true);
    if (result.success) {
      // Confirm snake_case fields are present
      expect(result.data.api_key_id).toBe("key-abc-123");
      expect(result.data.tool_name).toBe("list_tasks");
      expect(result.data.project_id).toBe("proj-1");
      expect(result.data.latency_ms).toBe(42);
      expect(result.data.created_at).toBe("2024-01-01T00:00:00Z");
    }
  });

  it("rejects entry with empty api_key_id", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, api_key_id: "" });
    expect(result.success).toBe(false);
  });

  it("rejects entry with empty tool_name", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, tool_name: "" });
    expect(result.success).toBe(false);
  });

  it("rejects entry missing required id field", () => {
    const { id: _id, ...entryWithoutId } = validEntry;
    const result = AuditLogEntrySchema.safeParse(entryWithoutId);
    expect(result.success).toBe(false);
  });

  it("rejects entry with non-integer id", () => {
    const result = AuditLogEntrySchema.safeParse({ ...validEntry, id: 1.5 });
    expect(result.success).toBe(false);
  });
});

// ============================================================================
// ApiKeyCreatedResponseSchema
// ============================================================================

describe("ApiKeyCreatedResponseSchema", () => {
  const validResponse = {
    id: "key-new-123",
    name: "New API Key",
    rawKey: "rxk_live_supersecretkey_abc123xyz",
    keyPrefix: "rxk_live_abc",
    permissions: 3,
  };

  it("validates a complete created response", () => {
    const result = ApiKeyCreatedResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
  });

  it("exposes rawKey field (only available at creation/rotation time)", () => {
    const result = ApiKeyCreatedResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.rawKey).toBe("rxk_live_supersecretkey_abc123xyz");
    }
  });

  it("uses camelCase field names (rawKey, keyPrefix)", () => {
    const result = ApiKeyCreatedResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.rawKey).toBeDefined();
      expect(result.data.keyPrefix).toBeDefined();
    }
  });

  it("rejects response without rawKey", () => {
    const { rawKey: _rawKey, ...withoutRawKey } = validResponse;
    const result = ApiKeyCreatedResponseSchema.safeParse(withoutRawKey);
    expect(result.success).toBe(false);
  });

  it("rejects response without id", () => {
    const { id: _id, ...withoutId } = validResponse;
    const result = ApiKeyCreatedResponseSchema.safeParse(withoutId);
    expect(result.success).toBe(false);
  });
});

// ============================================================================
// Parsing Utilities
// ============================================================================

describe("parseApiKey", () => {
  it("returns parsed ApiKey on valid input", () => {
    const data = {
      id: "key-1",
      name: "Test Key",
      keyPrefix: "rxk_live_tst",
      permissions: 1,
      createdAt: "2024-01-01T00:00:00Z",
      revokedAt: null,
      lastUsedAt: null,
      projectIds: [],
    };
    const result = parseApiKey(data);
    expect(result.id).toBe("key-1");
    expect(result.permissions).toBe(1);
  });

  it("throws ZodError on invalid input", () => {
    expect(() => parseApiKey({ invalid: "data" })).toThrow();
  });
});

describe("safeParseApiKey", () => {
  it("returns parsed ApiKey on valid input", () => {
    const data = {
      id: "key-1",
      name: "Test Key",
      keyPrefix: "rxk_live_tst",
      permissions: 1,
      createdAt: "2024-01-01T00:00:00Z",
      revokedAt: null,
      lastUsedAt: null,
      projectIds: [],
    };
    const result = safeParseApiKey(data);
    expect(result).not.toBeNull();
    expect(result?.id).toBe("key-1");
  });

  it("returns null on invalid input instead of throwing", () => {
    const result = safeParseApiKey({ invalid: "data" });
    expect(result).toBeNull();
  });

  it("returns null on empty object", () => {
    const result = safeParseApiKey({});
    expect(result).toBeNull();
  });
});

// ============================================================================
// Type Inference Tests
// ============================================================================

describe("type inference", () => {
  it("correctly infers ApiKey type", () => {
    const key: ApiKey = {
      id: "key-1",
      name: "My Key",
      keyPrefix: "rxk_live_abc",
      permissions: 3,
      createdAt: "2024-01-01T00:00:00Z",
      revokedAt: null,
      lastUsedAt: "2024-01-02T00:00:00Z",
      projectIds: ["proj-1"],
    };
    expect(key.id).toBe("key-1");
    expect(key.projectIds).toContain("proj-1");
  });

  it("correctly infers AuditLogEntry type", () => {
    const entry: AuditLogEntry = {
      id: 1,
      api_key_id: "key-1",
      tool_name: "list_tasks",
      project_id: null,
      success: true,
      latency_ms: 10,
      created_at: "2024-01-01T00:00:00Z",
    };
    expect(entry.tool_name).toBe("list_tasks");
  });

  it("correctly infers ApiKeyCreatedResponse type", () => {
    const response: ApiKeyCreatedResponse = {
      id: "key-1",
      name: "New Key",
      rawKey: "the-secret",
      keyPrefix: "rxk_",
      permissions: 7,
    };
    expect(response.rawKey).toBe("the-secret");
  });
});
