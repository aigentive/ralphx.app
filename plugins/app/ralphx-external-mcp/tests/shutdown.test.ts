/**
 * Tests for graceful shutdown (SIGTERM/SIGINT handling) in index.ts.
 *
 * Strategy:
 *   - Mock process.exit so tests don't actually terminate
 *   - Start a real server on a free port
 *   - Call shutdown() directly (same code path as SIGTERM handler)
 *   - Verify: httpServer closes, activeTransports drained, process.exit(0) called
 */

import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
} from "vitest";
import { createServer } from "node:http";

// Mock auth and backend-client before importing index.ts
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

import { shutdown, startServer, getHttpServer } from "../src/index.js";

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

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("graceful shutdown", () => {
  let exitSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // Prevent process.exit from killing the test runner
    exitSpy = vi.spyOn(process, "exit").mockImplementation((_code?: number | string | null) => {
      // no-op
      return undefined as never;
    });
  });

  afterEach(() => {
    exitSpy.mockRestore();
    vi.clearAllMocks();
  });

  it("shutdown() calls process.exit(0)", async () => {
    const port = await getFreePort();
    await startServer({ port, host: "127.0.0.1", backendUrl: "http://127.0.0.1:3847" });

    await shutdown();

    expect(exitSpy).toHaveBeenCalledWith(0);
  });

  it("shutdown() closes the HTTP server (stops accepting new connections)", async () => {
    const port = await getFreePort();
    await startServer({ port, host: "127.0.0.1", backendUrl: "http://127.0.0.1:3847" });

    const server = getHttpServer();
    expect(server).toBeDefined();
    expect(server!.listening).toBe(true);

    await shutdown();

    expect(server!.listening).toBe(false);
  });

  it("getHttpServer() returns the server after startServer()", async () => {
    const port = await getFreePort();
    await startServer({ port, host: "127.0.0.1", backendUrl: "http://127.0.0.1:3847" });

    const server = getHttpServer();
    expect(server).toBeDefined();
    expect(server!.listening).toBe(true);

    // Cleanup
    await shutdown();
  });

  it("SIGTERM listener is registered and invokes shutdown", async () => {
    // Verify that a SIGTERM listener exists on process (registered at module load)
    const sigTermListeners = process.listeners("SIGTERM");
    const hasSigTermHandler = sigTermListeners.some(
      (fn) => fn.toString().includes("shutdown") || fn === shutdown
    );
    expect(sigTermListeners.length).toBeGreaterThan(0);
    // At least one listener should be our shutdown function
    expect(sigTermListeners).toContain(shutdown);
  });

  it("SIGINT listener is registered and invokes shutdown", async () => {
    const sigIntListeners = process.listeners("SIGINT");
    expect(sigIntListeners).toContain(shutdown);
  });
});
