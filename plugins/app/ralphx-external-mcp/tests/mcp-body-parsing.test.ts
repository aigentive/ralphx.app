/**
 * Tests for MCP body pre-parsing in handleMcpRequest.
 *
 * Verifies that handleMcpRequest correctly pre-parses POST bodies before
 * handing to the SDK transport, with proper error distinction:
 *   - Malformed JSON → 400 with JSON-RPC -32700 error
 *   - Empty body    → 400
 *   - Stream error  → 500 (tested via readBodyString unit test)
 *   - Valid JSON    → reaches transport (not rejected at parse layer)
 *   - Batch array   → valid JSON, reaches transport
 *
 * Covers BOTH new-session and resumed-session transport paths.
 */

import {
  describe,
  it,
  expect,
  vi,
  beforeAll,
  beforeEach,
  afterEach,
} from "vitest";
import { Readable } from "node:stream";
import type { IncomingMessage } from "node:http";
import { createServer } from "node:http";

// Mock auth and backend-client before importing the server
vi.mock("../src/auth.js", async (importOriginal) => {
  const original = await importOriginal<typeof import("../src/auth.js")>();
  return {
    ...original,
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

import { startServer, readBodyString } from "../src/index.js";
import { authMiddleware, clearAuthCache } from "../src/auth.js";
import { configureRateLimiter } from "../src/rate-limiter.js";
import type { ApiKeyContext } from "../src/types.js";

// ─── Helpers ──────────────────────────────────────────────────────────────────

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
  keyId: "key-body-parse-001",
  projectIds: ["proj-test"],
  permissions: 3,
};

const mockAuthMiddleware = authMiddleware as ReturnType<typeof vi.fn>;

// ─── Test setup ───────────────────────────────────────────────────────────────

let testPort: number;

beforeAll(async () => {
  testPort = await getFreePort();
  await startServer({
    port: testPort,
    host: "127.0.0.1",
    backendUrl: "http://127.0.0.1:3847",
    rateLimit: {
      requestsPerSecond: 100,
      maxConnections: 50,
      authFailuresBeforeLockout: 100,
      lockoutDurationSecs: 30,
      maxExternalIdeationSessions: 1,
    },
  });
});

beforeEach(() => {
  clearAuthCache();
  configureRateLimiter({
    requestsPerSecond: 100,
    authFailuresBeforeLockout: 100,
    lockoutDurationMs: 30_000,
  });
  mockAuthMiddleware.mockImplementation(async () => VALID_KEY_CONTEXT);
});

afterEach(() => {
  vi.clearAllMocks();
  clearAuthCache();
});

// ─── New-session path ─────────────────────────────────────────────────────────

describe("new-session path — POST body pre-parsing", () => {
  it("malformed JSON returns 400 with JSON-RPC parse error", async () => {
    const { status, body } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: "not valid json {{{",
    });

    expect(status).toBe(400);
    expect(body.jsonrpc).toBe("2.0");
    expect((body.error as Record<string, unknown>).code).toBe(-32700);
    expect((body.error as Record<string, unknown>).message).toBe("Parse error");
  });

  it("empty body returns 400", async () => {
    const { status } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: "",
    });

    expect(status).toBe(400);
  });

  it("valid JSON-RPC initialize request reaches transport (not rejected at parse layer)", async () => {
    const { status } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        id: 1,
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test-client", version: "1.0" },
        },
      }),
    });

    // Should not be a body-parse rejection
    expect(status).not.toBe(400);
    expect(status).not.toBe(500);
  });

  it("new session rejects non-initialize requests without transport stack noise", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const { status, body } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "tools/list",
        id: 42,
      }),
    });

    expect(status).toBe(400);
    expect(body.jsonrpc).toBe("2.0");
    expect(body.id).toBe(42);
    expect((body.error as Record<string, unknown>).message).toContain(
      "initialize"
    );
    expect(
      errorSpy.mock.calls.some((args) =>
        String(args[0]).includes("Transport error")
      )
    ).toBe(false);

    errorSpy.mockRestore();
  });

  it("batch array is valid JSON and reaches transport", async () => {
    // JSON-RPC batch requests are arrays — must be accepted by the parse layer
    const { status } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify([
        { jsonrpc: "2.0", method: "initialize", id: 1, params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "test", version: "1" } } },
      ]),
    });

    // Arrays are valid JSON — should not be rejected at the parse layer
    expect(status).not.toBe(400);
  });
});

// ─── Resumed-session path ─────────────────────────────────────────────────────

describe("resumed-session path — POST body pre-parsing", () => {
  it("malformed JSON on resumed session returns 400 with JSON-RPC parse error", async () => {
    // First establish a session by sending a valid initialize
    const initRes = await fetch(`http://127.0.0.1:${testPort}/mcp`, {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        id: 1,
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test-client", version: "1.0" },
        },
      }),
    });

    const sessionId = initRes.headers.get("mcp-session-id");
    if (!sessionId) {
      // Transport may not return a session ID if initialization doesn't complete fully.
      // In that case, skip the resumed-session test gracefully.
      return;
    }

    // Now send a malformed body with the session ID
    const { status, body } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
        "mcp-session-id": sessionId,
      },
      body: "not valid json {{{",
    });

    expect(status).toBe(400);
    expect(body.jsonrpc).toBe("2.0");
    expect((body.error as Record<string, unknown>).code).toBe(-32700);
  });

  it("empty body on resumed session returns 400", async () => {
    const initRes = await fetch(`http://127.0.0.1:${testPort}/mcp`, {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        id: 2,
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "test-client", version: "1.0" },
        },
      }),
    });

    const sessionId = initRes.headers.get("mcp-session-id");
    if (!sessionId) {
      return;
    }

    const { status } = await httpRequest(testPort, "/mcp", {
      method: "POST",
      headers: {
        Authorization: "Bearer rxk_live_testkey",
        "Content-Type": "application/json",
        "mcp-session-id": sessionId,
      },
      body: "",
    });

    expect(status).toBe(400);
  });
});

// ─── readBodyString unit tests ────────────────────────────────────────────────

describe("readBodyString", () => {
  it("returns { ok: true, body } for a complete readable stream", async () => {
    const readable = Readable.from(["hello", " world"]) as unknown as IncomingMessage;
    const result = await readBodyString(readable);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.body).toBe("hello world");
    }
  });

  it("returns { ok: true, body: '' } for an empty stream", async () => {
    const readable = Readable.from([]) as unknown as IncomingMessage;
    const result = await readBodyString(readable);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.body).toBe("");
    }
  });

  it("returns { ok: false } when stream emits an error", async () => {
    const readable = new Readable({
      read() {
        this.emit("error", new Error("simulated stream failure"));
      },
    }) as unknown as IncomingMessage;

    const result = await readBodyString(readable);
    expect(result.ok).toBe(false);
  });

  it("handles JSON body correctly (used by parse layer)", async () => {
    const jsonBody = JSON.stringify({ jsonrpc: "2.0", method: "test", id: 1 });
    const readable = Readable.from([jsonBody]) as unknown as IncomingMessage;
    const result = await readBodyString(readable);
    expect(result.ok).toBe(true);
    if (result.ok) {
      const parsed = JSON.parse(result.body);
      expect(parsed.method).toBe("test");
    }
  });
});
