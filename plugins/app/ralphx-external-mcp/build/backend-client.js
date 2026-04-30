/**
 * Backend client for ralphx-external-mcp
 *
 * HTTP proxy to :3847 that:
 * - Injects X-RalphX-Project-Scope header from validated key's project list
 * - Forwards requests with proper error propagation
 */
/** Header name for project scope injection */
export const PROJECT_SCOPE_HEADER = "X-RalphX-Project-Scope";
/** Header used to mark requests coming from external MCP */
export const EXTERNAL_MCP_HEADER = "X-RalphX-External-MCP";
/** Header used for Tauri-owned local bypass calls through the external MCP process */
export const TAURI_MCP_HEADER = "X-RalphX-Tauri-MCP";
/** Header for propagating the API key ID to the backend (for permission enforcement) */
export const KEY_ID_HEADER = "X-RalphX-Key-Id";
/** Header for backend-owned parent conversation/workspace binding */
export const PARENT_CONVERSATION_HEADER = "X-RalphX-Parent-Conversation-Id";
export class BackendClient {
    baseUrl;
    timeoutMs;
    constructor(options) {
        this.baseUrl = options.baseUrl.replace(/\/$/, "");
        this.timeoutMs = options.timeoutMs ?? 30_000;
    }
    /**
     * Make a GET request to the backend, injecting project scope header.
     */
    async get(path, keyContext, params) {
        const url = this.buildUrl(path, params);
        return this.request("GET", url, keyContext, undefined);
    }
    /**
     * Make a POST request to the backend, injecting project scope header.
     */
    async post(path, keyContext, body) {
        const url = this.buildUrl(path);
        return this.request("POST", url, keyContext, body);
    }
    /**
     * Make a DELETE request to the backend, injecting project scope header.
     */
    async delete(path, keyContext) {
        const url = this.buildUrl(path);
        return this.request("DELETE", url, keyContext, undefined);
    }
    buildUrl(path, params) {
        const url = new URL(`${this.baseUrl}${path}`);
        if (params) {
            for (const [key, value] of Object.entries(params)) {
                url.searchParams.set(key, value);
            }
        }
        return url.toString();
    }
    async request(method, url, keyContext, body) {
        const headers = {
            "Content-Type": "application/json",
            [EXTERNAL_MCP_HEADER]: "1",
        };
        // Inject key ID header — allows backend to identify the calling key for permission enforcement
        headers[KEY_ID_HEADER] = keyContext.keyId;
        if (keyContext.tauriOrigin === true) {
            headers[TAURI_MCP_HEADER] = "1";
            const parentConversationId = keyContext.runtime?.parentConversationId;
            if (parentConversationId) {
                headers[PARENT_CONVERSATION_HEADER] = parentConversationId;
            }
        }
        // Inject project scope header — comma-separated list of project IDs
        if (keyContext.projectIds.length > 0) {
            headers[PROJECT_SCOPE_HEADER] = keyContext.projectIds.join(",");
        }
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), this.timeoutMs);
        let resp;
        try {
            resp = await fetch(url, {
                method,
                headers,
                body: body !== undefined ? JSON.stringify(body) : undefined,
                signal: controller.signal,
            });
        }
        catch (err) {
            if (err instanceof Error && err.name === "AbortError") {
                throw new BackendError(504, "Backend request timed out");
            }
            throw new BackendError(503, "Backend unreachable");
        }
        finally {
            clearTimeout(timeout);
        }
        let responseBody;
        const contentType = resp.headers.get("content-type") ?? "";
        if (contentType.includes("application/json")) {
            responseBody = (await resp.json());
        }
        else {
            responseBody = (await resp.text());
        }
        return { status: resp.status, body: responseBody };
    }
    /**
     * Check if the backend is reachable (for /ready endpoint).
     */
    async isReachable() {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 5_000);
        try {
            const resp = await fetch(`${this.baseUrl}/health`, {
                method: "GET",
                signal: controller.signal,
            });
            return resp.ok;
        }
        catch {
            return false;
        }
        finally {
            clearTimeout(timeout);
        }
    }
}
export class BackendError extends Error {
    statusCode;
    constructor(statusCode, message) {
        super(message);
        this.statusCode = statusCode;
        this.name = "BackendError";
    }
}
/** Singleton backend client */
let _client = null;
export function getBackendClient() {
    if (!_client) {
        _client = new BackendClient({ baseUrl: "http://127.0.0.1:3847" });
    }
    return _client;
}
export function configureBackendClient(options) {
    _client = new BackendClient(options);
}
//# sourceMappingURL=backend-client.js.map