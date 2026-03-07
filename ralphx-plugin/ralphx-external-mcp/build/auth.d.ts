/**
 * Bearer auth middleware for ralphx-external-mcp
 *
 * Validates rxk_live_ API keys against :3847/api/auth/validate-key with 30s TTL cache.
 * Cache entries are invalidated immediately on key rotation events.
 */
import type { IncomingMessage, ServerResponse } from "node:http";
import type { ApiKeyContext } from "./types.js";
export declare function configureAuth(options: {
    backendUrl: string;
}): void;
/** Invalidate a specific key from cache (call on rotation/revocation events) */
export declare function invalidateCacheEntry(rawKey: string): void;
/** Clear entire cache (for testing) */
export declare function clearAuthCache(): void;
/**
 * Extract Bearer token from Authorization header.
 * Returns undefined if header is missing or malformed.
 */
export declare function extractBearerToken(req: IncomingMessage): string | undefined;
/**
 * Validate an API key against the backend, using 30s TTL cache.
 * Returns ApiKeyContext on success, throws on failure.
 */
export declare function validateKey(rawKey: string): Promise<ApiKeyContext>;
/**
 * Full auth middleware: extract token, check format, validate key.
 * Returns ApiKeyContext on success, sends HTTP error response on failure.
 * Returns undefined if the request was rejected (caller should not proceed).
 */
export declare function authMiddleware(req: IncomingMessage, res: ServerResponse): Promise<ApiKeyContext | undefined>;
export declare class AuthError extends Error {
    readonly statusCode: number;
    constructor(statusCode: number, message: string);
}
//# sourceMappingURL=auth.d.ts.map