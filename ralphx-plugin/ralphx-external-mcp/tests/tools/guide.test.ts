/**
 * Tests for v1_get_agent_guide tool handler and bidirectional sync validation.
 *
 * No backend mocks needed for guide-specific tests (pure static content).
 * backend-client mock is required for TOOL_CATEGORIES import via index.ts
 * (which imports all tool modules at module load time).
 *
 * Test coverage:
 *   1. No section arg → returns full guide containing all section headings
 *   2. Valid section → returns focused content only (not other sections)
 *   3. Invalid section → returns error JSON with error: "invalid_section"
 *   4. FULL_GUIDE mentions every tool name from ALL_TOOL_NAMES
 *   5. Each section is self-contained (heading + tool name + table row)
 *   6. Bidirectional sync: TOOL_CATEGORIES ↔ ALL_TOOL_NAMES ↔ FULL_GUIDE
 *   7. Integration dispatch: full path through real MCP server, mocked auth
 */

import { describe, it, expect, vi, beforeAll, afterEach } from "vitest";
import { createServer } from "node:http";
import type { ApiKeyContext } from "../../src/types.js";

// ─── Mocks (must be declared before imports) ─────────────────────────────────

// backend-client mock: required so that importing TOOL_CATEGORIES from index.ts
// (which imports all tool modules) does not fail at module load time.
vi.mock("../../src/backend-client.js", () => ({
  getBackendClient: () => ({ get: vi.fn(), post: vi.fn() }),
  configureBackendClient: vi.fn(),
  BackendError: class BackendError extends Error {
    statusCode: number;
    constructor(statusCode: number, message: string) {
      super(message);
      this.name = "BackendError";
      this.statusCode = statusCode;
    }
  },
}));

// auth mock: required for integration test server startup
vi.mock("../../src/auth.js", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../src/auth.js")>();
  return {
    ...original,
    authMiddleware: vi.fn(),
    configureAuth: vi.fn(),
  };
});

// ─── Imports (after mocks) ────────────────────────────────────────────────────

const { handleGetAgentGuide } = await import("../../src/tools/guide.js");
const { GUIDE_SECTIONS, FULL_GUIDE, VALID_SECTIONS, ALL_TOOL_NAMES } =
  await import("../../src/tools/guide-content.js");
const { TOOL_CATEGORIES } = await import("../../src/tools/index.js");
const { startServer } = await import("../../src/index.js");
const { authMiddleware, clearAuthCache } = await import("../../src/auth.js");
const { configureRateLimiter } = await import("../../src/rate-limiter.js");

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const testContext: ApiKeyContext = {
  keyId: "key-test",
  projectIds: [],
  permissions: 7,
};

// ─── Helper: allocate a free port ─────────────────────────────────────────────

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

// ─── Tests 1-3: Handler behavior ──────────────────────────────────────────────

describe("handleGetAgentGuide — handler behavior", () => {
  it("Test 1: no section returns the full guide containing all section first-lines", async () => {
    const result = await handleGetAgentGuide({}, testContext);
    expect(result).toBe(FULL_GUIDE);
    // Every section's first line must appear in the full guide
    for (const sectionContent of Object.values(GUIDE_SECTIONS)) {
      const firstLine = sectionContent.split("\n")[0];
      expect(result).toContain(firstLine);
    }
  });

  it("Test 2: valid section 'ideation' returns only its content and not other section headings", async () => {
    const result = await handleGetAgentGuide({ section: "ideation" }, testContext);
    expect(result).toBe(GUIDE_SECTIONS.ideation);
    expect(result).toContain("v1_start_ideation");
    // Must NOT contain discovery or pipeline section headings
    const discoveryHeading = GUIDE_SECTIONS.discovery.split("\n")[0];
    const pipelineHeading = GUIDE_SECTIONS.pipeline.split("\n")[0];
    expect(result).not.toContain(discoveryHeading);
    expect(result).not.toContain(pipelineHeading);
  });

  it("Test 3: invalid section returns error JSON with error: invalid_section and valid_sections array", async () => {
    const result = await handleGetAgentGuide({ section: "nonexistent" }, testContext);
    const parsed = JSON.parse(result) as Record<string, unknown>;
    expect(parsed.error).toBe("invalid_section");
    expect(Array.isArray(parsed.valid_sections)).toBe(true);
    expect((parsed.valid_sections as string[]).sort()).toEqual([...VALID_SECTIONS].sort());
    expect(typeof parsed.message).toBe("string");
    expect(parsed.message as string).toContain("nonexistent");
  });
});

// ─── Tests 4-5: Guide content completeness ────────────────────────────────────

describe("guide content completeness", () => {
  it("Test 4: FULL_GUIDE mentions every tool name from ALL_TOOL_NAMES", () => {
    for (const toolName of ALL_TOOL_NAMES) {
      expect(FULL_GUIDE, `'${toolName}' not found in FULL_GUIDE`).toContain(toolName);
    }
  });

  it("Test 5: each section is self-contained — has a heading, at least one tool name, and a table row", () => {
    for (const [sectionKey, sectionContent] of Object.entries(GUIDE_SECTIONS)) {
      // (a) starts with a markdown heading (## or #)
      expect(
        sectionContent.trimStart(),
        `Section '${sectionKey}' does not start with a markdown heading`
      ).toMatch(/^##? /);

      // (b) contains at least one tool name from ALL_TOOL_NAMES
      const hasTool = ALL_TOOL_NAMES.some((name) => sectionContent.includes(name));
      expect(hasTool, `Section '${sectionKey}' contains no tool names from ALL_TOOL_NAMES`).toBe(
        true
      );

      // (c) contains at least one pipe character (table row indicator)
      expect(sectionContent, `Section '${sectionKey}' contains no table rows (|)`).toContain("|");
    }
  });
});

// ─── Test 6: Bidirectional sync ───────────────────────────────────────────────

describe("bidirectional sync: TOOL_CATEGORIES ↔ ALL_TOOL_NAMES ↔ FULL_GUIDE", () => {
  it("Test 6: every tool in TOOL_CATEGORIES is in ALL_TOOL_NAMES, every tool in ALL_TOOL_NAMES is in TOOL_CATEGORIES, and every tool in ALL_TOOL_NAMES appears in FULL_GUIDE", () => {
    const categorizedTools = Object.values(TOOL_CATEGORIES).flat() as string[];

    // Direction A: TOOL_CATEGORIES → ALL_TOOL_NAMES
    // Catches tools added to index.ts but not added to guide-content.ts canonical list
    for (const toolName of categorizedTools) {
      expect(
        ALL_TOOL_NAMES,
        `'${toolName}' is in TOOL_CATEGORIES but missing from ALL_TOOL_NAMES`
      ).toContain(toolName);
    }

    // Direction B: ALL_TOOL_NAMES → TOOL_CATEGORIES
    // Catches tools in canonical list but not registered in index.ts
    for (const toolName of ALL_TOOL_NAMES) {
      expect(
        categorizedTools,
        `'${toolName}' is in ALL_TOOL_NAMES but missing from TOOL_CATEGORIES`
      ).toContain(toolName);
    }

    // Direction C: ALL_TOOL_NAMES → FULL_GUIDE (content coverage)
    // Catches tools that are listed but not documented in guide sections
    for (const toolName of ALL_TOOL_NAMES) {
      expect(
        FULL_GUIDE,
        `'${toolName}' is in ALL_TOOL_NAMES but not mentioned in FULL_GUIDE`
      ).toContain(toolName);
    }
  });
});

// ─── Test 7: Integration dispatch through real MCP server ────────────────────

describe("integration: full dispatch path through MCP server", () => {
  let integrationPort: number;
  const mockAuth = authMiddleware as ReturnType<typeof vi.fn>;

  beforeAll(async () => {
    integrationPort = await getFreePort();
    configureRateLimiter({
      requestsPerSecond: 100,
      authFailuresBeforeLockout: 100,
      lockoutDurationMs: 60_000,
    });
    await startServer({
      port: integrationPort,
      host: "127.0.0.1",
      backendUrl: "http://127.0.0.1:3847",
      rateLimit: {
        requestsPerSecond: 100,
        maxConnections: 50,
        authFailuresBeforeLockout: 100,
        lockoutDurationSecs: 60,
        maxExternalIdeationSessions: 1,
      },
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
    clearAuthCache();
  });

  it("Test 7: v1_get_agent_guide dispatches correctly — response contains guide content, not a not_implemented error", async () => {
    mockAuth.mockImplementation(async () => testContext);

    const MCP_HEADERS = {
      Authorization: "Bearer rxk_live_testkey",
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream",
    };

    // Step 1: initialize — creates the stateful session and returns Mcp-Session-Id
    const initRes = await fetch(`http://127.0.0.1:${integrationPort}/mcp`, {
      method: "POST",
      headers: MCP_HEADERS,
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        id: 1,
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "guide-test", version: "1" },
        },
      }),
    });
    expect(initRes.status).toBe(200);
    const sessionId = initRes.headers.get("mcp-session-id");
    expect(sessionId).toBeTruthy();

    // Step 2: tools/call — dispatches to v1_get_agent_guide handler
    const callRes = await fetch(`http://127.0.0.1:${integrationPort}/mcp`, {
      method: "POST",
      headers: {
        ...MCP_HEADERS,
        "Mcp-Session-Id": sessionId!,
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "tools/call",
        id: 2,
        params: {
          name: "v1_get_agent_guide",
          arguments: {},
        },
      }),
    });

    expect(callRes.status).toBe(200);
    const text = await callRes.text();
    // Guide heading is the first line of FULL_GUIDE — must be present in the SSE response body
    expect(text).toContain("RalphX Agent Guide");
    // A typo in the switch case would produce "not_implemented" — must not appear
    expect(text).not.toContain("not_implemented");
  });
});
