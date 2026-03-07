/**
 * Health and readiness endpoints for ralphx-external-mcp
 *
 * - GET /health — always 200 if the process is alive
 * - GET /ready  — 200 only if :3847 is reachable (cache warm check is best-effort)
 */

import type { IncomingMessage, ServerResponse } from "node:http";
import { getBackendClient } from "./backend-client.js";

/** Always 200 — signals the process is alive */
export function handleHealth(_req: IncomingMessage, res: ServerResponse): void {
  sendJson(res, 200, { status: "ok" });
}

/** 200 if backend is reachable, 503 otherwise */
export async function handleReady(
  _req: IncomingMessage,
  res: ServerResponse
): Promise<void> {
  const client = getBackendClient();
  const reachable = await client.isReachable();

  if (reachable) {
    sendJson(res, 200, { status: "ready", backend: "reachable" });
  } else {
    sendJson(res, 503, { status: "not_ready", backend: "unreachable" });
  }
}

function sendJson(res: ServerResponse, status: number, body: object): void {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(payload),
  });
  res.end(payload);
}
