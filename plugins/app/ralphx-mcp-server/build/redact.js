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
import path from "node:path";
const TRACE_ROOT_DIR = path.resolve(process.cwd(), ".artifacts/logs/mcp-proxy");
function isWithinDir(rootDir, candidatePath) {
    const root = path.resolve(rootDir);
    const candidate = path.resolve(candidatePath);
    return candidate === root || candidate.startsWith(`${root}${path.sep}`);
}
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
function slugify(input) {
    const slug = input
        .trim()
        .replace(/[^a-zA-Z0-9._-]+/g, "-")
        .replace(/-+/g, "-")
        .replace(/^-|-$/g, "");
    return slug.length > 0 ? slug : "unknown";
}
function buildTraceFilename() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    const agentType = slugify(process.env.RALPHX_AGENT_TYPE ?? "unknown-agent");
    const contextType = slugify(process.env.RALPHX_CONTEXT_TYPE ?? "unknown-context");
    const contextId = slugify((process.env.RALPHX_CONTEXT_ID ?? "no-context-id").slice(0, 32));
    return `${timestamp}-${process.pid}-${agentType}-${contextType}-${contextId}.jsonl`;
}
function resolveTraceDir() {
    const override = process.env.RALPHX_MCP_TRACE_DIR?.trim();
    if (!override) {
        return TRACE_ROOT_DIR;
    }
    if (path.isAbsolute(override)) {
        return TRACE_ROOT_DIR;
    }
    const resolved = path.resolve(TRACE_ROOT_DIR, override);
    if (!isWithinDir(TRACE_ROOT_DIR, resolved)) {
        return TRACE_ROOT_DIR;
    }
    return resolved;
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
export function safeTrace(event, payload) {
    const record = {
        ts: new Date().toISOString(),
        pid: process.pid,
        event,
        agent_type: process.env.RALPHX_AGENT_TYPE ?? "unknown",
        task_id: process.env.RALPHX_TASK_ID ?? null,
        project_id: process.env.RALPHX_PROJECT_ID ?? null,
        context_type: process.env.RALPHX_CONTEXT_TYPE ?? null,
        context_id: process.env.RALPHX_CONTEXT_ID ?? null,
    };
    if (payload !== undefined) {
        record.payload = redactSecrets(stringify(payload));
    }
    try {
        fs.appendFileSync(getTraceLogPath(), `${JSON.stringify(record)}\n`, "utf8");
    }
    catch (error) {
        safeError("[RalphX MCP] Failed to append MCP trace log:", error);
    }
}
//# sourceMappingURL=redact.js.map