const FETCH_FAILED_PATTERN = /fetch failed/i;
/**
 * Keep human-in-the-loop waits just under the observed MCP tool ceiling
 * so we can return structured timeout payloads instead of transport errors.
 */
export const HUMAN_WAIT_CLIENT_TIMEOUT_MS = 290 * 1000;
export function createHumanWaitAbortController(timeoutMs = HUMAN_WAIT_CLIENT_TIMEOUT_MS) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
    return { controller, timeoutId };
}
export function isHumanWaitTimeoutError(error, elapsedMs, timeoutMs = HUMAN_WAIT_CLIENT_TIMEOUT_MS) {
    if (!(error instanceof Error)) {
        return false;
    }
    if (error.name === "AbortError") {
        return true;
    }
    // Claude/MCP can terminate the underlying wait near the hard tool ceiling and
    // surface a generic fetch failure instead of an AbortError. Treat it as a
    // timeout only when it happens near our configured deadline.
    return elapsedMs >= timeoutMs - 15_000 && FETCH_FAILED_PATTERN.test(error.message);
}
//# sourceMappingURL=human-wait.js.map