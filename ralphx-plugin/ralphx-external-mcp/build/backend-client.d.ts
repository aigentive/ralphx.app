/**
 * Backend client for ralphx-external-mcp
 *
 * HTTP proxy to :3847 that:
 * - Injects X-RalphX-Project-Scope header from validated key's project list
 * - Forwards requests with proper error propagation
 */
import type { ApiKeyContext } from "./types.js";
/** Header name for project scope injection */
export declare const PROJECT_SCOPE_HEADER = "X-RalphX-Project-Scope";
/** Header used to mark requests coming from external MCP */
export declare const EXTERNAL_MCP_HEADER = "X-RalphX-External-MCP";
/** Header for propagating the API key ID to the backend (for permission enforcement) */
export declare const KEY_ID_HEADER = "X-RalphX-Key-Id";
export interface BackendClientOptions {
    baseUrl: string;
    /** Timeout in milliseconds (default: 30000) */
    timeoutMs?: number;
}
export interface BackendResponse<T = unknown> {
    status: number;
    body: T;
}
export declare class BackendClient {
    private readonly baseUrl;
    private readonly timeoutMs;
    constructor(options: BackendClientOptions);
    /**
     * Make a GET request to the backend, injecting project scope header.
     */
    get<T>(path: string, keyContext: ApiKeyContext, params?: Record<string, string>): Promise<BackendResponse<T>>;
    /**
     * Make a POST request to the backend, injecting project scope header.
     */
    post<T>(path: string, keyContext: ApiKeyContext, body?: unknown): Promise<BackendResponse<T>>;
    /**
     * Make a DELETE request to the backend, injecting project scope header.
     */
    delete<T>(path: string, keyContext: ApiKeyContext): Promise<BackendResponse<T>>;
    private buildUrl;
    private request;
    /**
     * Check if the backend is reachable (for /ready endpoint).
     */
    isReachable(): Promise<boolean>;
}
export declare class BackendError extends Error {
    readonly statusCode: number;
    constructor(statusCode: number, message: string);
}
export declare function getBackendClient(): BackendClient;
export declare function configureBackendClient(options: BackendClientOptions): void;
//# sourceMappingURL=backend-client.d.ts.map