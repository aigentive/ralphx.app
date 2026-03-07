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
import type { ExternalMcpConfig } from "./types.js";
/** Returns the current number of active TCP connections (for testing). */
export declare function getActiveConnections(): number;
/** Reset connection counter — used in tests to clean up between runs. */
export declare function resetActiveConnections(): void;
/**
 * Start the external MCP server.
 */
export declare function startServer(config?: Partial<ExternalMcpConfig>): Promise<void>;
//# sourceMappingURL=index.d.ts.map