/**
 * Integration tests for ralphx-external-mcp
 *
 * Tests end-to-end request routing through startServer:
 * - /health and /ready endpoints (no auth)
 * - Auth failures (missing / invalid key → 401)
 * - IP lockout (5 failures from same IP → 429)
 * - Rate limit (token bucket exhausted → 429)
 * - Connection limit (active connections at cap → 503)
 * - Successful MCP routing (valid key → reaches MCP handler)
 *
 * Mock strategy:
 *   We need to mock the auth module's `validateKey` and the backend-client's
 *   `isReachable` without stubbing global `fetch` (which would also intercept
 *   the test's own HTTP requests to the local server).
 */

import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
  beforeAll,
} from "vitest";
import { createServer } from "node:http";

// Mock these modules before importing the server so the mocks are in place
// when startServer is called.
vi.mock("../src/auth.js", async (importOriginal) => {
  const original = await importOriginal<typeof import("../src/auth.js")>();
  return {
    ...original,
    // We'll override authMiddleware per-test via the mock below
    authMiddleware: vi.fn(),
    configureAuth: vi.fn(),
  };
});

vi.mock("../src/backend-client.js", async (importOriginal) => {
  const original = await importOriginal<typeof import("../src/backend-client.js")>();
  return {
    ...original,
    configureBackendClient: vi.fn(),
    getBackendClient: vi.fn(() => ({
      isReachable: vi.fn().mockResolvedValue(true),
    })),
  };
});

import {
  getActiveConnections,
  resetActiveConnections,
  startServer,
} from "../src/index.js";
import { authMiddleware, clearAuthCache } from "../src/auth.js";
import { configureRateLimiter, getRateLimiter } from "../src/rate-limiter.js";
import { getBackendClient } from "../src/backend-client.js";
import type { ApiKeyContext } from "../src/types.js";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Allocate a free port by binding to :0 then closing */
async function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = createServer();
    srv.listen(0, "127.0.0.1", () => {
      const addr = srv.address();
      if (!addr || typeof addr === "string") {
        reject(new Error("Could not get port"));
        return;
      }
      const port = addr.port;
      srv.close(() => resolve(port));
    });
    srv.once("error", reject);
  });
}

/** HTTP request to local test server — uses the REAL fetch (not mocked) */
async function httpRequest(
  port: number,
  path: string,
  options: {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
  } = {}
): Promise<{ status: number; body: Record<string, unknown> }> {
  const url = `http://127.0.0.1:${port}${path}`;
  // Use real undici/native fetch — not affected by vi.mock above
  const res = await fetch(url, {
    method: options.method ?? "GET",
    headers: options.headers,
    body: options.body,
  });
  let body: Record<string, unknown> = {};
  try {
    body = (await res.json()) as Record<string, unknown>;
  } catch {
    // non-JSON body — leave as {}
  }
  return { status: res.status, body };
}

const VALID_KEY_CONTEXT: ApiKeyContext = {
  keyId: "key-test-001",
  projectIds: ["proj-abc"],
  permissions: 3,
};

// Cast to the mocked type for per-test control
const mockAuthMiddleware = authMiddleware as ReturnType<typeof vi.fn>;
const mockGetBackendClient = getBackendClient as ReturnType<typeof vi.fn>;

// ─── Test setup ───────────────────────────────────────────────────────────────

let mainPort: number;

beforeAll(async () => {
  mainPort = await getFreePort();

  // Start the main test server (shared across tests in this suite)
  await startServer({
    port: mainPort,
    host: "127.0.0.1",
    backendUrl: "http://127.0.0.1:3847",
    rateLimit: {
      requestsPerSecond: 100,
      maxConnections: 50,
      authFailuresBeforeLockout: 5,
      lockoutDurationSecs: 30,
      maxExternalIdeationSessions: 1,
    },
  });
});

beforeEach(() => {
  clearAuthCache();

  // Default: auth succeeds
  mockAuthMiddleware.mockImplementation(
    async (_req: unknown, _res: unknown) => VALID_KEY_CONTEXT
  );

  // Default: backend is reachable
  mockGetBackendClient.mockReturnValue({
    isReachable: vi.fn().mockResolvedValue(true),
  });

  // Reset rate limiter to generous defaults
  configureRateLimiter({
    requestsPerSecond: 100,
    authFailuresBeforeLockout: 5,
    lockoutDurationMs: 30_000,
  });
});

afterEach(() => {
  vi.clearAllMocks();
  clearAuthCache();
});

// ─── Health / ready ───────────────────────────────────────────────────────────

describe("health and readiness endpoints", () => {
  it("GET /health returns 200 without auth", async () => {
    const { status, body } = await httpRequest(mainPort, "/health");
    expect(status).toBe(200);
    expect(body.status).toBe("ok");
  });

  it("GET /ready returns 200 when backend is reachable", async () => {
    mockGetBackendClient.mockReturnValue({
      isReachable: vi.fn().mockResolvedValue(true),
    });
    const { status, body } = await httpRequest(mainPort, "/ready");
    expect(status).toBe(200);
    expect(body.status).toBe("ready");
  });

  it("GET /ready returns 503 when backend is unreachable", async () => {
    mockGetBackendClient.mockReturnValue({
      isReachable: vi.fn().mockResolvedValue(false),
    });
    const { status, body } = await httpRequest(mainPort, "/ready");
    expect(status).toBe(503);
    expect(body.status).toBe("not_ready");
  });
});

// ─── Auth failures ────────────────────────────────────────────────────────────

describe("authentication", () => {
  it("returns 401 when Authorization header is missing", async () => {
    // authMiddleware must return undefined (no context) to trigger 401
    mockAuthMiddleware.mockImplementation(
      async (_req: unknown, res: { writeHead: (s: number, h: Record<string, string>) => void; end: (b: string) => void }) => {
        const body = JSON.stringify({ error: "Missing or malformed Authorization header" });
        res.writeHead(401, {
          "Content-Type": "application/json",
          "Content-Length": String(Buffer.byteLength(body)),
        });
        res.end(body);
        return undefined;
      }
    );

    const { status } = await httpRequest(mainPort, "/mcp", { method: "POST" });
    expect(status).toBe(401);
  });

  it("returns 401 when API key is invalid (backend rejects)", async () => {
    mockAuthMiddleware.mockImplementation(
      async (_req: unknown, res: { writeHead: (s: number, h: Record<string, string>) => void; end: (b: string) => void }) => {
        const body = JSON.stringify({ error: "Invalid or revoked API key" });
        res.writeHead(401, {
          "Content-Type": "application/json",
          "Content-Length": String(Buffer.byteLength(body)),
        });
        res.end(body);
        return undefined;
      }
    );

    const { status } = await httpRequest(mainPort, "/mcp", {
      method: "POST",
      headers: { Authorization: "Bearer rxk_live_badkey" },
    });
    expect(status).toBe(401);
  });
});

// ─── IP lockout ───────────────────────────────────────────────────────────────

describe("IP lockout after repeated auth failures", () => {
  it("returns 429 after 5 auth failures from same IP", async () => {
    // Configure rate limiter with threshold 5
    configureRateLimiter({
      requestsPerSecond: 100,
      authFailuresBeforeLockout: 5,
      lockoutDurationMs: 30_000,
    });

    // Each call to authMiddleware returns undefined (failure) and records a failure
    let callCount = 0;
    mockAuthMiddleware.mockImplementation(
      async (_req: unknown, res: { writeHead: (s: number, h: Record<string, string>) => void; end: (b: string) => void }) => {
        callCount++;
        const body = JSON.stringify({ error: "Unauthorized" });
        res.writeHead(401, {
          "Content-Type": "application/json",
          "Content-Length": String(Buffer.byteLength(body)),
        });
        res.end(body);
        return undefined;
      }
    );

    // First 5 requests — auth fails, failures accumulate
    for (let i = 0; i < 5; i++) {
      await httpRequest(mainPort, "/mcp", {
        method: "POST",
        headers: { Authorization: "Bearer rxk_live_wrongkey" },
      });
    }

    // 6th request — should be blocked by IP lockout BEFORE auth middleware
    mockAuthMiddleware.mockClear(); // reset so we can check it's not called
    const { status, body } = await httpRequest(mainPort, "/mcp", {
      method: "POST",
      headers: { Authorization: "Bearer rxk_live_wrongkey" },
    });

    expect(status).toBe(429);
    expect((body.error as string).toLowerCase()).toContain("authentication");
    // authMiddleware should NOT be called — lockout happens before auth
    expect(mockAuthMiddleware).not.toHaveBeenCalled();
  });
});

// ─── Rate limit ───────────────────────────────────────────────────────────────

describe("per-key rate limiting", () => {
  it("returns 429 when token bucket is exhausted", async () => {
    // Fresh server on dedicated port so rate limiter state is clean
    const rlPort = await getFreePort();

    await startServer({
      port: rlPort,
      host: "127.0.0.1",
      backendUrl: "http://127.0.0.1:3847",
      rateLimit: {
        requestsPerSecond: 2, // tiny bucket
        maxConnections: 50,
        authFailuresBeforeLockout: 100,
        lockoutDurationSecs: 30,
        maxExternalIdeationSessions: 1,
      },
    });

    // Auth always succeeds
    mockAuthMiddleware.mockImplementation(
      async () => VALID_KEY_CONTEXT
    );

    const results: number[] = [];
    // Send 3 requests — first 2 should pass (2 token bucket), 3rd should be 429
    for (let i = 0; i < 3; i++) {
      const { status } = await httpRequest(rlPort, "/mcp", {
        method: "POST",
        headers: {
          Authorization: "Bearer rxk_live_ratekey",
          "Content-Type": "application/json",
        },
        body: "{}",
      });
      results.push(status);
    }

    expect(results).toContain(429);
  });
});

// ─── Connection limit ─────────────────────────────────────────────────────────

describe("connection limit enforcement", () => {
  it("returns 503 when maxConnections is 0 (all requests over limit)", async () => {
    const connPort = await getFreePort();

    await startServer({
      port: connPort,
      host: "127.0.0.1",
      backendUrl: "http://127.0.0.1:3847",
      rateLimit: {
        requestsPerSecond: 100,
        maxConnections: 0, // every non-health request is over limit
        authFailuresBeforeLockout: 100,
        lockoutDurationSecs: 30,
        maxExternalIdeationSessions: 1,
      },
    });

    const { status, body } = await httpRequest(connPort, "/mcp", {
      method: "POST",
      headers: { Authorization: "Bearer rxk_live_goodkey" },
    });

    expect(status).toBe(503);
    expect((body.error as string).toLowerCase()).toContain("connection");
  });
});

// ─── getActiveConnections / resetActiveConnections ────────────────────────────

describe("connection tracking exports", () => {
  it("getActiveConnections returns a non-negative number", () => {
    const count = getActiveConnections();
    expect(typeof count).toBe("number");
    expect(count).toBeGreaterThanOrEqual(0);
  });

  it("resetActiveConnections resets the counter to 0", () => {
    resetActiveConnections();
    expect(getActiveConnections()).toBe(0);
  });
});

// ─── Successful MCP routing ───────────────────────────────────────────────────

describe("successful MCP routing", () => {
  it("valid key reaches MCP handler (not 401/429/503)", async () => {
    // Reset rate limiter to generous limits
    configureRateLimiter({
      requestsPerSecond: 100,
      authFailuresBeforeLockout: 100,
      lockoutDurationMs: 30_000,
    });

    mockAuthMiddleware.mockImplementation(async () => VALID_KEY_CONTEXT);

    const { status } = await httpRequest(mainPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_goodkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        id: 1,
        params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "test", version: "1" } },
      }),
    });

    // Should not be auth/rate/connection error
    expect(status).not.toBe(401);
    expect(status).not.toBe(429);
    expect(status).not.toBe(503);
  });
});
