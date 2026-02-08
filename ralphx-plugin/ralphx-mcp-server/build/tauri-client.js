/**
 * HTTP client for calling RalphX Tauri backend
 * All MCP tools forward to these endpoints (proxy pattern)
 */
const TAURI_API_URL = process.env.TAURI_API_URL || "http://127.0.0.1:3847";
export class TauriClientError extends Error {
    statusCode;
    details;
    constructor(message, statusCode, details) {
        super(message);
        this.statusCode = statusCode;
        this.details = details;
        this.name = "TauriClientError";
    }
}
/**
 * Call a Tauri backend endpoint via HTTP POST
 * @param endpoint - Endpoint path (e.g., "create_task_proposal")
 * @param args - Request body (JSON)
 * @returns Response data
 * @throws TauriClientError on HTTP errors
 */
export async function callTauri(endpoint, args) {
    const url = `${TAURI_API_URL}/api/${endpoint}`;
    try {
        const response = await fetch(url, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(args),
        });
        if (!response.ok) {
            let errorMessage = `Tauri API error: ${response.statusText}`;
            let details;
            try {
                const errorData = (await response.json());
                if (errorData.error) {
                    errorMessage = errorData.error;
                    details = errorData.details;
                }
            }
            catch {
                // JSON parse failed — try plain text response
                try {
                    const text = await response.text();
                    if (text) {
                        errorMessage = text;
                    }
                }
                catch {
                    // Both JSON and text failed, use status text
                }
            }
            throw new TauriClientError(errorMessage, response.status, details);
        }
        return await response.json();
    }
    catch (error) {
        if (error instanceof TauriClientError) {
            throw error;
        }
        // Network or other fetch errors
        throw new TauriClientError(`Failed to connect to Tauri backend at ${url}: ${error instanceof Error ? error.message : String(error)}`, 0);
    }
}
/**
 * Call a Tauri backend endpoint via HTTP GET
 * @param endpoint - Endpoint path (e.g., "task_context/task-123")
 * @returns Response data
 * @throws TauriClientError on HTTP errors
 */
export async function callTauriGet(endpoint) {
    const url = `${TAURI_API_URL}/api/${endpoint}`;
    try {
        const response = await fetch(url, {
            method: "GET",
            headers: {
                "Content-Type": "application/json",
            },
        });
        if (!response.ok) {
            let errorMessage = `Tauri API error: ${response.statusText}`;
            let details;
            try {
                const errorData = (await response.json());
                if (errorData.error) {
                    errorMessage = errorData.error;
                    details = errorData.details;
                }
            }
            catch {
                // JSON parse failed — try plain text response
                try {
                    const text = await response.text();
                    if (text) {
                        errorMessage = text;
                    }
                }
                catch {
                    // Both JSON and text failed, use status text
                }
            }
            throw new TauriClientError(errorMessage, response.status, details);
        }
        return await response.json();
    }
    catch (error) {
        if (error instanceof TauriClientError) {
            throw error;
        }
        // Network or other fetch errors
        throw new TauriClientError(`Failed to connect to Tauri backend at ${url}: ${error instanceof Error ? error.message : String(error)}`, 0);
    }
}
//# sourceMappingURL=tauri-client.js.map