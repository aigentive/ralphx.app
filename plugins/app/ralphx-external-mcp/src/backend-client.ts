/**
 * Backend client for ralphx-external-mcp
 *
 * HTTP proxy to :3847 that:
 * - Injects X-RalphX-Project-Scope header from validated key's project list
 * - Forwards requests with proper error propagation
 */

import type { ApiKeyContext } from "./types.js";

/** Header name for project scope injection */
export const PROJECT_SCOPE_HEADER = "X-RalphX-Project-Scope";

/** Header used to mark requests coming from external MCP */
export const EXTERNAL_MCP_HEADER = "X-RalphX-External-MCP";

/** Header used for Tauri-owned local bypass calls through the external MCP process */
export const TAURI_MCP_HEADER = "X-RalphX-Tauri-MCP";

/** Header for propagating the API key ID to the backend (for permission enforcement) */
export const KEY_ID_HEADER = "X-RalphX-Key-Id";

export interface BackendClientOptions {
  baseUrl: string;
  /** Timeout in milliseconds (default: 30000) */
  timeoutMs?: number;
}

export interface BackendResponse<T = unknown> {
  status: number;
  body: T;
}

export class BackendClient {
  private readonly baseUrl: string;
  private readonly timeoutMs: number;

  constructor(options: BackendClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.timeoutMs = options.timeoutMs ?? 30_000;
  }

  /**
   * Make a GET request to the backend, injecting project scope header.
   */
  async get<T>(
    path: string,
    keyContext: ApiKeyContext,
    params?: Record<string, string>
  ): Promise<BackendResponse<T>> {
    const url = this.buildUrl(path, params);
    return this.request<T>("GET", url, keyContext, undefined);
  }

  /**
   * Make a POST request to the backend, injecting project scope header.
   */
  async post<T>(
    path: string,
    keyContext: ApiKeyContext,
    body?: unknown
  ): Promise<BackendResponse<T>> {
    const url = this.buildUrl(path);
    return this.request<T>("POST", url, keyContext, body);
  }

  /**
   * Make a DELETE request to the backend, injecting project scope header.
   */
  async delete<T>(
    path: string,
    keyContext: ApiKeyContext
  ): Promise<BackendResponse<T>> {
    const url = this.buildUrl(path);
    return this.request<T>("DELETE", url, keyContext, undefined);
  }

  private buildUrl(path: string, params?: Record<string, string>): string {
    const url = new URL(`${this.baseUrl}${path}`);
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        url.searchParams.set(key, value);
      }
    }
    return url.toString();
  }

  private async request<T>(
    method: string,
    url: string,
    keyContext: ApiKeyContext,
    body: unknown
  ): Promise<BackendResponse<T>> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      [EXTERNAL_MCP_HEADER]: "1",
    };

    // Inject key ID header — allows backend to identify the calling key for permission enforcement
    headers[KEY_ID_HEADER] = keyContext.keyId;

    if (keyContext.tauriOrigin === true) {
      headers[TAURI_MCP_HEADER] = "1";
    }

    // Inject project scope header — comma-separated list of project IDs
    if (keyContext.projectIds.length > 0) {
      headers[PROJECT_SCOPE_HEADER] = keyContext.projectIds.join(",");
    }

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.timeoutMs);

    let resp: Response;
    try {
      resp = await fetch(url, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });
    } catch (err) {
      if (err instanceof Error && err.name === "AbortError") {
        throw new BackendError(504, "Backend request timed out");
      }
      throw new BackendError(503, "Backend unreachable");
    } finally {
      clearTimeout(timeout);
    }

    let responseBody: T;
    const contentType = resp.headers.get("content-type") ?? "";
    if (contentType.includes("application/json")) {
      responseBody = (await resp.json()) as T;
    } else {
      responseBody = (await resp.text()) as unknown as T;
    }

    return { status: resp.status, body: responseBody };
  }

  /**
   * Check if the backend is reachable (for /ready endpoint).
   */
  async isReachable(): Promise<boolean> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);
    try {
      const resp = await fetch(`${this.baseUrl}/health`, {
        method: "GET",
        signal: controller.signal,
      });
      return resp.ok;
    } catch {
      return false;
    } finally {
      clearTimeout(timeout);
    }
  }
}

export class BackendError extends Error {
  constructor(
    public readonly statusCode: number,
    message: string
  ) {
    super(message);
    this.name = "BackendError";
  }
}

/** Singleton backend client */
let _client: BackendClient | null = null;

export function getBackendClient(): BackendClient {
  if (!_client) {
    _client = new BackendClient({ baseUrl: "http://127.0.0.1:3847" });
  }
  return _client;
}

export function configureBackendClient(options: BackendClientOptions): void {
  _client = new BackendClient(options);
}
