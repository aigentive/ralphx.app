/**
 * Bearer auth middleware for ralphx-external-mcp
 *
 * Validates rxk_live_ API keys against :3847/api/auth/validate-key with 30s TTL cache.
 * Cache entries are invalidated immediately on key rotation events.
 */

import type { IncomingMessage, ServerResponse } from "node:http";
import type { ApiKeyContext, ValidateKeyResponse } from "./types.js";

const CACHE_TTL_MS = 30_000;
const BEARER_PREFIX = "Bearer ";
const KEY_PREFIX = "rxk_live_";

interface CacheEntry {
  context: ApiKeyContext;
  expiresAt: number;
}

/** In-memory auth cache with TTL */
const cache = new Map<string, CacheEntry>();

let backendUrl = "http://127.0.0.1:3847";

export function configureAuth(options: { backendUrl: string }): void {
  backendUrl = options.backendUrl;
}

/** Invalidate a specific key from cache (call on rotation/revocation events) */
export function invalidateCacheEntry(rawKey: string): void {
  cache.delete(rawKey);
}

/** Clear entire cache (for testing) */
export function clearAuthCache(): void {
  cache.clear();
}

/**
 * Extract Bearer token from Authorization header.
 * Returns undefined if header is missing or malformed.
 */
export function extractBearerToken(req: IncomingMessage): string | undefined {
  const authHeader = req.headers["authorization"];
  if (!authHeader || !authHeader.startsWith(BEARER_PREFIX)) {
    return undefined;
  }
  return authHeader.slice(BEARER_PREFIX.length).trim();
}

/**
 * Validate an API key against the backend, using 30s TTL cache.
 * Returns ApiKeyContext on success, throws on failure.
 */
export async function validateKey(rawKey: string): Promise<ApiKeyContext> {
  // Check cache first
  const cached = cache.get(rawKey);
  if (cached && cached.expiresAt > Date.now()) {
    return cached.context;
  }

  // Live validation against backend
  const url = `${backendUrl}/api/auth/validate-key`;
  let resp: Response;
  try {
    resp = await fetch(url, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${rawKey}`,
      },
    });
  } catch (err) {
    throw new AuthError(503, "Backend unavailable for key validation");
  }

  if (resp.status === 401 || resp.status === 403) {
    // Ensure stale cache entry is removed
    cache.delete(rawKey);
    throw new AuthError(401, "Invalid or revoked API key");
  }

  if (!resp.ok) {
    throw new AuthError(502, `Backend validation error: ${resp.status}`);
  }

  const body = (await resp.json()) as ValidateKeyResponse;
  const context: ApiKeyContext = {
    keyId: body.key_id,
    projectIds: body.project_ids,
    permissions: body.permissions,
  };

  // Populate cache
  cache.set(rawKey, {
    context,
    expiresAt: Date.now() + CACHE_TTL_MS,
  });

  return context;
}

/**
 * Full auth middleware: extract token, check format, validate key.
 * Returns ApiKeyContext on success, sends HTTP error response on failure.
 * Returns undefined if the request was rejected (caller should not proceed).
 */
export async function authMiddleware(
  req: IncomingMessage,
  res: ServerResponse
): Promise<ApiKeyContext | undefined> {
  const token = extractBearerToken(req);
  if (!token) {
    sendError(res, 401, "Missing Authorization header");
    return undefined;
  }

  if (!token.startsWith(KEY_PREFIX)) {
    sendError(res, 401, "Invalid API key format — expected rxk_live_ prefix");
    return undefined;
  }

  try {
    return await validateKey(token);
  } catch (err) {
    if (err instanceof AuthError) {
      sendError(res, err.statusCode, err.message);
    } else {
      sendError(res, 500, "Internal auth error");
    }
    return undefined;
  }
}

export class AuthError extends Error {
  constructor(
    public readonly statusCode: number,
    message: string
  ) {
    super(message);
    this.name = "AuthError";
  }
}

function sendError(res: ServerResponse, status: number, message: string): void {
  const body = JSON.stringify({ error: message });
  res.writeHead(status, {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(body);
}
