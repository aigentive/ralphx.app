#!/usr/bin/env node

/**
 * ralphx-external-mcp — External MCP Server for RalphX Orchestration API
 *
 * Exposes 27 orchestration-level tools to external agents (reefbot.ai, etc.) via
 * MCP over HTTP+SSE (Streamable HTTP Transport).
 *
 * Architecture:
 *   External Agent → Bearer auth → Rate limit → MCP tools → :3847 backend
 *
 * Security:
 *   - Bearer API keys (rxk_live_ prefix) validated against :3847/api/auth/validate-key
 *   - TLS required for non-localhost binds
 *   - Token bucket rate limiting per key + IP-based auth throttle
 *   - X-RalphX-Project-Scope header injected for backend enforcement
 */

import { createServer as createHttpServer } from "node:http";
import { createServer as createHttpsServer } from "node:https";
import { randomUUID } from "node:crypto";

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import type { IncomingMessage, ServerResponse } from "node:http";

import { authMiddleware, configureAuth } from "./auth.js";
import { configureRateLimiter, getRateLimiter } from "./rate-limiter.js";
import { validateTlsConfig, buildTlsOptions } from "./tls.js";
import { configureBackendClient } from "./backend-client.js";
import { handleHealth, handleReady } from "./health.js";
import { registerTools } from "./tools/index.js";
import type { ExternalMcpConfig, ApiKeyContext } from "./types.js";

const DEFAULT_CONFIG: ExternalMcpConfig = {
  port: 3848,
  host: "127.0.0.1",
  backendUrl: "http://127.0.0.1:3847",
};

/** Active MCP transports keyed by session ID */
const activeTransports = new Map<string, StreamableHTTPServerTransport>();

/** Active connection count — tracked via httpServer connection/close events */
let _activeConnections = 0;

/** Returns the current number of active TCP connections (for testing). */
export function getActiveConnections(): number {
  return _activeConnections;
}

/**
 * Create and configure the MCP server instance.
 */
function createMcpServer(): Server {
  const server = new Server(
    {
      name: "ralphx-external-mcp",
      version: "1.0.0",
    },
    {
      capabilities: {
        tools: {},
      },
    }
  );

  return server;
}

/**
 * Main request handler — dispatches to /health, /ready, or MCP endpoint.
 */
async function handleRequest(
  req: IncomingMessage,
  res: ServerResponse,
  config: ExternalMcpConfig
): Promise<void> {
  const url = req.url ?? "/";

  // Health and readiness endpoints — no auth required
  if (url === "/health") {
    handleHealth(req, res);
    return;
  }

  if (url === "/ready") {
    await handleReady(req, res);
    return;
  }

  // Connection limit — checked before auth to shed load early
  const maxConnections = config.rateLimit?.maxConnections ?? 50;
  if (_activeConnections >= maxConnections) {
    sendError(res, 503, "Connection limit reached — try again later");
    return;
  }

  // All other requests require auth
  const ip = getClientIp(req);
  const rateLimiter = getRateLimiter();

  // Check IP-based auth throttle before attempting validation
  if (!rateLimiter.checkIpAuth(ip)) {
    sendError(res, 429, "Too many authentication failures — try again later");
    return;
  }

  // Validate bearer token
  const keyContext = await authMiddleware(req, res);
  if (!keyContext) {
    // authMiddleware already sent the error response
    rateLimiter.recordAuthFailure(ip);
    return;
  }

  // Auth succeeded — reset IP failure count
  rateLimiter.recordAuthSuccess(ip);

  // Apply per-key rate limit
  if (!rateLimiter.checkKey(keyContext.keyId)) {
    sendError(res, 429, "Rate limit exceeded — reduce request frequency");
    return;
  }

  // Route to MCP endpoint
  if (url === "/mcp" || url.startsWith("/mcp?")) {
    await handleMcpRequest(req, res, keyContext, config);
    return;
  }

  sendError(res, 404, "Not found");
}

/**
 * Handle MCP protocol requests using StreamableHTTPServerTransport.
 * Supports both stateful (session-based) and stateless modes.
 */
async function handleMcpRequest(
  req: IncomingMessage,
  res: ServerResponse,
  keyContext: ApiKeyContext,
  _config: ExternalMcpConfig
): Promise<void> {
  // Session management: look up existing transport or create new one
  const sessionId = req.headers["mcp-session-id"] as string | undefined;

  if (sessionId && activeTransports.has(sessionId)) {
    // Resume existing session
    const transport = activeTransports.get(sessionId)!;
    await transport.handleRequest(req, res);
    return;
  }

  // New session — only POST is valid for initialization
  if (req.method !== "POST") {
    sendError(res, 405, "Method not allowed — new sessions require POST");
    return;
  }

  // Create new MCP server + transport per session (stateful mode)
  const server = createMcpServer();

  // Register tools with key context provider
  let currentKeyContext: ApiKeyContext | undefined = keyContext;
  registerTools(server, () => currentKeyContext);

  const transport = new StreamableHTTPServerTransport({
    sessionIdGenerator: () => randomUUID(),
    onsessioninitialized: (sid) => {
      activeTransports.set(sid, transport);
      console.error(`[ralphx-external-mcp] Session initialized: ${sid}`);
    },
    onsessionclosed: (sid) => {
      activeTransports.delete(sid);
      currentKeyContext = undefined;
      console.error(`[ralphx-external-mcp] Session closed: ${sid}`);
    },
  });

  transport.onerror = (err) => {
    console.error("[ralphx-external-mcp] Transport error:", err);
  };

  await server.connect(transport);
  await transport.handleRequest(req, res);
}

/** Reset connection counter — used in tests to clean up between runs. */
export function resetActiveConnections(): void {
  _activeConnections = 0;
}

/**
 * Start the external MCP server.
 */
export async function startServer(
  config: Partial<ExternalMcpConfig> = {}
): Promise<void> {
  const cfg: ExternalMcpConfig = { ...DEFAULT_CONFIG, ...config };

  // Validate TLS requirements before starting
  validateTlsConfig(cfg.host, cfg.tls);

  // Configure singletons
  configureAuth({ backendUrl: cfg.backendUrl });
  configureBackendClient({ baseUrl: cfg.backendUrl });

  if (cfg.rateLimit) {
    configureRateLimiter({
      requestsPerSecond: cfg.rateLimit.requestsPerSecond,
      authFailuresBeforeLockout: cfg.rateLimit.authFailuresBeforeLockout,
      lockoutDurationMs: cfg.rateLimit.lockoutDurationSecs * 1000,
    });
  }

  // Create HTTP or HTTPS server based on TLS config
  const requestHandler = (req: IncomingMessage, res: ServerResponse): void => {
    handleRequest(req, res, cfg).catch((err) => {
      console.error("[ralphx-external-mcp] Unhandled request error:", err);
      if (!res.headersSent) {
        sendError(res, 500, "Internal server error");
      }
    });
  };

  const httpServer =
    cfg.tls
      ? createHttpsServer(buildTlsOptions(cfg.tls), requestHandler)
      : createHttpServer(requestHandler);

  // Track active TCP connections for max-connection enforcement.
  // Increment on new socket, decrement when that socket closes.
  httpServer.on("connection", (socket) => {
    _activeConnections += 1;
    socket.once("close", () => {
      _activeConnections -= 1;
    });
  });

  await new Promise<void>((resolve, reject) => {
    httpServer.listen(cfg.port, cfg.host, () => {
      const protocol = cfg.tls ? "https" : "http";
      console.error(
        `[ralphx-external-mcp] Server listening on ${protocol}://${cfg.host}:${cfg.port}`
      );
      console.error(
        `[ralphx-external-mcp] MCP endpoint: ${protocol}://${cfg.host}:${cfg.port}/mcp`
      );
      resolve();
    });
    httpServer.once("error", reject);
  });
}

/**
 * Returns the canonical client IP from the TCP socket.
 * X-Forwarded-For is intentionally ignored: without a validated trusted-proxy
 * list, a client could spoof any IP and bypass the auth-failure throttle.
 */
function getClientIp(req: IncomingMessage): string {
  return req.socket.remoteAddress ?? "unknown";
}

function sendError(res: ServerResponse, status: number, message: string): void {
  const body = JSON.stringify({ error: message });
  res.writeHead(status, {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(body);
}

// Entry point — parse config from environment and start
if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/"))) {
  const port = process.env.EXTERNAL_MCP_PORT
    ? parseInt(process.env.EXTERNAL_MCP_PORT, 10)
    : 3848;
  const host = process.env.EXTERNAL_MCP_HOST ?? "127.0.0.1";
  const backendUrl =
    process.env.RALPHX_BACKEND_URL ?? "http://127.0.0.1:3847";

  const tlsCert = process.env.EXTERNAL_MCP_TLS_CERT;
  const tlsKey = process.env.EXTERNAL_MCP_TLS_KEY;
  const tls =
    tlsCert && tlsKey ? { certPath: tlsCert, keyPath: tlsKey } : undefined;

  startServer({ port, host, backendUrl, tls }).catch((err) => {
    console.error("[ralphx-external-mcp] Fatal startup error:", err);
    process.exit(1);
  });
}
