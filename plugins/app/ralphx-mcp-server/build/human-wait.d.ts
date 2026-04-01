/**
 * Keep human-in-the-loop waits just under the observed MCP tool ceiling
 * so we can return structured timeout payloads instead of transport errors.
 */
export declare const HUMAN_WAIT_CLIENT_TIMEOUT_MS: number;
export declare function createHumanWaitAbortController(timeoutMs?: number): {
    controller: AbortController;
    timeoutId: ReturnType<typeof setTimeout>;
};
export declare function isHumanWaitTimeoutError(error: unknown, elapsedMs: number, timeoutMs?: number): boolean;
//# sourceMappingURL=human-wait.d.ts.map