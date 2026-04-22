/**
 * Secret redaction for MCP server logs.
 *
 * Mirrors the Rust secret_redactor patterns as JS regexps.
 * Apply to all console.error calls that log variable data to prevent
 * API keys, tokens, and bearer credentials from appearing in server logs.
 *
 * Pattern application order matters: more-specific prefixes (sk-ant-, sk-or-v1-)
 * MUST match before the generic sk- catch-all to prevent double-redaction.
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
const TRACE_SUBDIR = "mcp-proxy";
/**
 * Ordered list of secret patterns with their replacements.
 * Patterns are applied in order — specific before generic.
 */
const PATTERNS = [
    // 1. Anthropic API keys (most specific sk- variant first)
    { regex: /sk-ant-[a-zA-Z0-9_-]{20,}/g, replacement: "sk-ant-***REDACTED***" },
    // 2. OpenRouter keys
    { regex: /sk-or-v1-[a-zA-Z0-9]{20,}/g, replacement: "sk-or-v1-***REDACTED***" },
    // 3. RalphX API keys
    { regex: /rxk_live_[a-zA-Z0-9]{20,}/g, replacement: "rxk_live_***REDACTED***" },
    // 4. Generic OpenAI-style keys (catch-all after specific sk- patterns)
    { regex: /sk-[a-zA-Z0-9]{20,}/g, replacement: "sk-***REDACTED***" },
    // 5. Bearer tokens
    { regex: /Bearer [a-zA-Z0-9_.+-]{20,}/g, replacement: "Bearer ***REDACTED***" },
    // 6. ANTHROPIC_AUTH_TOKEN in JSON
    { regex: /"ANTHROPIC_AUTH_TOKEN"\s*:\s*"[^"]+"/g, replacement: '"ANTHROPIC_AUTH_TOKEN":"***REDACTED***"' },
    // 7. ANTHROPIC_API_KEY in JSON
    { regex: /"ANTHROPIC_API_KEY"\s*:\s*"[^"]+"/g, replacement: '"ANTHROPIC_API_KEY":"***REDACTED***"' },
    // 8. GitHub PATs
    { regex: /ghp_[a-zA-Z0-9]{20,}/g, replacement: "ghp_***REDACTED***" },
    // 9. GitHub OAuth tokens
    { regex: /gho_[a-zA-Z0-9]{20,}/g, replacement: "gho_***REDACTED***" },
];
/**
 * Apply all redaction patterns to a string.
 * Non-secret strings pass through unchanged.
 */
export function redactSecrets(input) {
    let result = input;
    for (const { regex, replacement } of PATTERNS) {
        regex.lastIndex = 0; // reset stateful regex
        result = result.replace(regex, replacement);
    }
    return result;
}
/**
 * Stringify an unknown value for redaction.
 * Objects are JSON-serialized; primitives are coerced to string.
 */
function stringify(arg) {
    if (typeof arg === "string")
        return arg;
    if (arg instanceof Error)
        return arg.stack ?? arg.message;
    try {
        return JSON.stringify(arg) ?? String(arg);
    }
    catch {
        return String(arg);
    }
}
/**
 * Safe drop-in replacement for console.error that redacts secrets from all arguments.
 * Use this instead of console.error wherever variable data (errors, objects, env values) is logged.
 *
 * Usage: safeError("[RalphX MCP] Error calling", name, error)
 */
export function safeError(...args) {
    const redacted = args.map((arg) => redactSecrets(stringify(arg)));
    console.error(...redacted);
}
let traceLogPath = null;
const SAFE_TRACE_EVENTS = new Set([
    "backend.error",
    "backend.request",
    "backend.response",
    "server.ready",
    "server.start",
    "tool.denied",
    "tool.dispatch",
    "tool.error",
    "tool.request",
    "tool.success",
    "tools.list",
]);
function buildTraceFilename() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    return `${timestamp}-${process.pid}.jsonl`;
}
function isInsidePath(candidate, parent) {
    const relative = path.relative(path.resolve(parent), path.resolve(candidate));
    return (relative === ""
        || (!!relative && !relative.startsWith("..") && !path.isAbsolute(relative)));
}
function platformTraceFallbackDir() {
    if (process.platform === "darwin" && process.env.HOME) {
        return path.join(process.env.HOME, "Library/Application Support/com.ralphx.app/logs", TRACE_SUBDIR);
    }
    if (process.platform === "win32" && process.env.APPDATA) {
        return path.join(process.env.APPDATA, "RalphX", "logs", TRACE_SUBDIR);
    }
    const stateRoot = process.env.XDG_STATE_HOME
        ?? (process.env.HOME ? path.join(process.env.HOME, ".local/state") : os.tmpdir());
    return path.join(stateRoot, "ralphx", "logs", TRACE_SUBDIR);
}
function isSafeTraceDir(candidate) {
    if (!candidate || !path.isAbsolute(candidate)) {
        return false;
    }
    const workingDirectory = process.env.RALPHX_WORKING_DIRECTORY;
    if (workingDirectory && isInsidePath(candidate, workingDirectory)) {
        return false;
    }
    return true;
}
function resolveTraceDir() {
    if (isSafeTraceDir(process.env.RALPHX_MCP_TRACE_DIR)) {
        return process.env.RALPHX_MCP_TRACE_DIR;
    }
    return platformTraceFallbackDir();
}
export function getTraceLogPath() {
    if (traceLogPath) {
        return traceLogPath;
    }
    const traceDir = resolveTraceDir();
    fs.mkdirSync(traceDir, { recursive: true });
    traceLogPath = path.join(traceDir, buildTraceFilename());
    return traceLogPath;
}
export function resetTraceLogPathForTests() {
    traceLogPath = null;
}
function normalizeTraceEvent(event) {
    return SAFE_TRACE_EVENTS.has(event) ? event : "unknown";
}
export function safeTrace(event, _payload) {
    const record = {
        ts: new Date().toISOString(),
        pid: process.pid,
        event: normalizeTraceEvent(event),
    };
    try {
        fs.appendFileSync(getTraceLogPath(), `${JSON.stringify(record)}\n`, "utf8");
    }
    catch (error) {
        safeError("[RalphX MCP] Failed to append MCP trace log:", error);
    }
}
//# sourceMappingURL=redact.js.map