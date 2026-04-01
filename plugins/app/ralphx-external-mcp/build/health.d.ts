/**
 * Health and readiness endpoints for ralphx-external-mcp
 *
 * - GET /health — always 200 if the process is alive
 * - GET /ready  — 200 only if :3847 is reachable (cache warm check is best-effort)
 */
import type { IncomingMessage, ServerResponse } from "node:http";
/** Always 200 — signals the process is alive */
export declare function handleHealth(_req: IncomingMessage, res: ServerResponse): void;
/** 200 if backend is reachable, 503 otherwise */
export declare function handleReady(_req: IncomingMessage, res: ServerResponse): Promise<void>;
//# sourceMappingURL=health.d.ts.map