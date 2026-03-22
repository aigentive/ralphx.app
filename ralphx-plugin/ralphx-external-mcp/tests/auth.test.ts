import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  extractBearerToken,
  validateKey,
  authMiddleware,
  clearAuthCache,
  invalidateCacheEntry,
  invalidateCacheByKeyId,
  configureAuth,
  AuthError,
} from "../src/auth.js";
import type { IncomingMessage, ServerResponse } from "node:http";

// ─── Helpers ─────────────────────────────────────────────────────────────────

function makeReq(headers: Record<string, string> = {}): IncomingMessage {
  return { headers } as unknown as IncomingMessage;
}

function makeRes(): {
  res: ServerResponse;
  state: { statusCode: number | undefined; body: string | undefined };
} {
  const state: { statusCode: number | undefined; body: string | undefined } = {
    statusCode: undefined,
    body: undefined,
  };
  const res = {
    writeHead: (code: number) => {
      state.statusCode = code;
    },
    end: (b: string) => {
      state.body = b;
    },
    headersSent: false,
  } as unknown as ServerResponse;
  return { res, state };
}

// ─── extractBearerToken ───────────────────────────────────────────────────────

describe("extractBearerToken", () => {
  it("returns token from valid Authorization header", () => {
    const req = makeReq({ authorization: "Bearer rxk_live_abc123" });
    expect(extractBearerToken(req)).toBe("rxk_live_abc123");
  });

  it("returns undefined when Authorization header is missing", () => {
    const req = makeReq({});
    expect(extractBearerToken(req)).toBeUndefined();
  });

  it("returns undefined when header does not start with Bearer", () => {
    const req = makeReq({ authorization: "Basic abc123" });
    expect(extractBearerToken(req)).toBeUndefined();
  });

  it("trims whitespace from token", () => {
    const req = makeReq({ authorization: "Bearer   rxk_live_trimmed  " });
    expect(extractBearerToken(req)).toBe("rxk_live_trimmed");
  });
});

// ─── validateKey ─────────────────────────────────────────────────────────────

describe("validateKey", () => {
  beforeEach(() => {
    clearAuthCache();
    configureAuth({ backendUrl: "http://127.0.0.1:3847" });
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    clearAuthCache();
  });

  it("validates key against backend and caches result", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        key_id: "key-001",
        project_ids: ["proj-abc"],
        permissions: 3,
      }),
    });
    vi.stubGlobal("fetch", mockFetch);

    const result = await validateKey("rxk_live_testkey");
    expect(result.keyId).toBe("key-001");
    expect(result.projectIds).toEqual(["proj-abc"]);
    expect(result.permissions).toBe(3);

    // Second call should use cache (fetch called only once)
    await validateKey("rxk_live_testkey");
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it("throws AuthError on 401 from backend", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 401 })
    );

    await expect(validateKey("rxk_live_invalid")).rejects.toThrow(AuthError);
  });

  it("throws AuthError on 403 from backend", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 403 })
    );

    await expect(validateKey("rxk_live_revoked")).rejects.toThrow(AuthError);
  });

  it("throws AuthError on network failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("ECONNREFUSED"))
    );

    await expect(validateKey("rxk_live_anyk")).rejects.toThrow(AuthError);
  });

  it("invalidateCacheEntry removes cached entry", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        key_id: "key-002",
        project_ids: ["proj-xyz"],
        permissions: 1,
      }),
    });
    vi.stubGlobal("fetch", mockFetch);

    await validateKey("rxk_live_torevoke");
    expect(mockFetch).toHaveBeenCalledTimes(1);

    invalidateCacheEntry("rxk_live_torevoke");

    await validateKey("rxk_live_torevoke");
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});

// ─── authMiddleware ───────────────────────────────────────────────────────────

describe("authMiddleware", () => {
  beforeEach(() => {
    clearAuthCache();
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    clearAuthCache();
  });

  it("returns undefined and sends 401 when no Authorization header", async () => {
    const req = makeReq({});
    const { res, state } = makeRes();

    const result = await authMiddleware(req, res);
    expect(result).toBeUndefined();
    expect(state.statusCode).toBe(401);
  });

  it("returns undefined and sends 401 when token lacks rxk_live_ prefix", async () => {
    const req = makeReq({ authorization: "Bearer bad_format_token" });
    const { res, state } = makeRes();

    const result = await authMiddleware(req, res);
    expect(result).toBeUndefined();
    expect(state.statusCode).toBe(401);
  });

  it("returns ApiKeyContext on valid key", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({
          key_id: "key-valid",
          project_ids: ["proj-1"],
          permissions: 3,
        }),
      })
    );

    const req = makeReq({ authorization: "Bearer rxk_live_validkey" });
    const { res } = makeRes();

    const result = await authMiddleware(req, res);
    expect(result).not.toBeUndefined();
    expect(result?.keyId).toBe("key-valid");
  });
});

// ─── invalidateCacheByKeyId ───────────────────────────────────────────────────

describe("invalidateCacheByKeyId", () => {
  beforeEach(() => {
    clearAuthCache();
    configureAuth({ backendUrl: "http://127.0.0.1:3847" });
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    clearAuthCache();
  });

  it("removes cache entries matching keyId", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        key_id: "key-to-invalidate",
        project_ids: ["proj-1"],
        permissions: 3,
      }),
    });
    vi.stubGlobal("fetch", mockFetch);

    await validateKey("rxk_live_somekey");
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // Invalidate by key_id
    invalidateCacheByKeyId("key-to-invalidate");

    // Next call should go to backend again (cache cleared)
    await validateKey("rxk_live_somekey");
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("is a no-op for empty keyId", () => {
    // Should not throw
    expect(() => invalidateCacheByKeyId("")).not.toThrow();
  });
});
