/**
 * Tests for secret redaction — mirrors the Rust secret_redactor test patterns.
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, it, expect } from "vitest";
import { getTraceLogPath, redactSecrets, resetTraceLogPathForTests, safeError, safeTrace, } from "../redact.js";
afterEach(() => {
    delete process.env.RALPHX_MCP_TRACE_DIR;
    delete process.env.RALPHX_AGENT_TYPE;
    delete process.env.RALPHX_CONTEXT_TYPE;
    delete process.env.RALPHX_CONTEXT_ID;
    delete process.env.RALPHX_TASK_ID;
    delete process.env.RALPHX_PROJECT_ID;
    resetTraceLogPathForTests();
});
describe("redactSecrets — pattern matching", () => {
    // Pattern 1: Anthropic API keys
    it("redacts Anthropic API key (sk-ant-)", () => {
        const input = "key=sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456";
        expect(redactSecrets(input)).toBe("key=sk-ant-***REDACTED***");
    });
    // Pattern 2: OpenRouter keys
    it("redacts OpenRouter key (sk-or-v1-)", () => {
        const input = "token: sk-or-v1-abcdefghijklmnopqrstuvwxyz1234";
        expect(redactSecrets(input)).toBe("token: sk-or-v1-***REDACTED***");
    });
    // Pattern 3: RalphX API keys
    it("redacts RalphX API key (rxk_live_)", () => {
        const input = "key=rxk_live_abcdefghijklmnopqrstuvwxyz1234";
        expect(redactSecrets(input)).toBe("key=rxk_live_***REDACTED***");
    });
    // Pattern 4: Generic OpenAI-style keys (catch-all)
    it("redacts generic OpenAI-style key (sk-)", () => {
        const input = "OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz1234";
        expect(redactSecrets(input)).toBe("OPENAI_API_KEY=sk-***REDACTED***");
    });
    // Pattern 5: Bearer tokens
    it("redacts Bearer tokens", () => {
        const input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abc";
        expect(redactSecrets(input)).toBe("Authorization: Bearer ***REDACTED***");
    });
    // Pattern 6: ANTHROPIC_AUTH_TOKEN in JSON
    it("redacts ANTHROPIC_AUTH_TOKEN in JSON", () => {
        const input = '{"ANTHROPIC_AUTH_TOKEN": "sk-ant-secret-key-value-here"}';
        expect(redactSecrets(input)).toBe('{"ANTHROPIC_AUTH_TOKEN":"***REDACTED***"}');
    });
    // Pattern 7: ANTHROPIC_API_KEY in JSON
    it("redacts ANTHROPIC_API_KEY in JSON", () => {
        const input = '{"ANTHROPIC_API_KEY": "sk-ant-api-key-here-longer-value"}';
        expect(redactSecrets(input)).toBe('{"ANTHROPIC_API_KEY":"***REDACTED***"}');
    });
    // Pattern 8: GitHub PATs
    it("redacts GitHub PAT (ghp_)", () => {
        const input = "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz1234";
        expect(redactSecrets(input)).toBe("GITHUB_TOKEN=ghp_***REDACTED***");
    });
    // Pattern 9: GitHub OAuth tokens
    it("redacts GitHub OAuth token (gho_)", () => {
        const input = "oauth_token=gho_abcdefghijklmnopqrstuvwxyz1234";
        expect(redactSecrets(input)).toBe("oauth_token=gho_***REDACTED***");
    });
});
describe("redactSecrets — non-secrets pass through", () => {
    it("preserves plain log messages", () => {
        const input = "[RalphX MCP] Starting server...";
        expect(redactSecrets(input)).toBe(input);
    });
    it("preserves short sk- prefixes that are not secrets", () => {
        // Less than 20 chars after sk-
        const input = "sk-short123";
        expect(redactSecrets(input)).toBe(input);
    });
    it("preserves non-secret environment variable names", () => {
        const input = "TAURI_API_URL=http://127.0.0.1:3847";
        expect(redactSecrets(input)).toBe(input);
    });
    it("preserves empty string", () => {
        expect(redactSecrets("")).toBe("");
    });
    it("preserves short Bearer values", () => {
        // Less than 20 chars after Bearer
        const input = "Bearer shorttoken";
        expect(redactSecrets(input)).toBe(input);
    });
});
describe("redactSecrets — ordering (specific before generic)", () => {
    it("redacts sk-ant- before the generic sk- catch-all (no double-redaction)", () => {
        const input = "key=sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456";
        const result = redactSecrets(input);
        expect(result).toBe("key=sk-ant-***REDACTED***");
        expect(result).not.toContain("sk-***REDACTED***");
    });
    it("redacts sk-or-v1- before the generic sk- catch-all", () => {
        const input = "token=sk-or-v1-abcdefghijklmnopqrstuvwxyz1234";
        const result = redactSecrets(input);
        expect(result).toBe("token=sk-or-v1-***REDACTED***");
        expect(result).not.toContain("sk-***REDACTED***");
    });
});
describe("redactSecrets — multi-secret lines", () => {
    it("redacts multiple secrets on the same line", () => {
        const input = "key1=sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456 key2=ghp_abcdefghijklmnopqrstuvwxyz1234";
        const result = redactSecrets(input);
        expect(result).toBe("key1=sk-ant-***REDACTED*** key2=ghp_***REDACTED***");
    });
    it("redacts secrets in JSON settings string", () => {
        const input = '{"ANTHROPIC_AUTH_TOKEN": "sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456", "OTHER": "value"}';
        const result = redactSecrets(input);
        expect(result).toContain('"ANTHROPIC_AUTH_TOKEN":"***REDACTED***"');
        expect(result).not.toContain("sk-ant-api03");
    });
});
describe("redactSecrets — edge cases", () => {
    it("handles partial pattern matches without redacting", () => {
        // 'sk-' with exactly 19 chars (one short of 20 minimum)
        const input = "sk-1234567890123456789"; // 19 chars after sk-
        expect(redactSecrets(input)).toBe(input);
    });
    it("handles rxk_live_ with exactly 20 char suffix", () => {
        const input = "rxk_live_12345678901234567890"; // exactly 20 chars
        expect(redactSecrets(input)).toBe("rxk_live_***REDACTED***");
    });
});
describe("safeError — integration", () => {
    it("is callable without throwing", () => {
        expect(() => safeError("[RalphX MCP] test message", { key: "value" })).not.toThrow();
    });
    it("accepts Error objects", () => {
        const err = new Error("sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456");
        expect(() => safeError("Error:", err)).not.toThrow();
    });
});
describe("safeTrace — file logging", () => {
    it("writes redacted trace records under the safe trace root", () => {
        const expectedRoot = path.resolve(process.cwd(), ".artifacts/logs/mcp-proxy");
        process.env.RALPHX_AGENT_TYPE = "ralphx-ideation";
        process.env.RALPHX_CONTEXT_TYPE = "ideation";
        process.env.RALPHX_CONTEXT_ID = "session-123";
        safeTrace("tool.request", {
            api_key: "sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456",
        });
        const logPath = getTraceLogPath();
        const contents = fs.readFileSync(logPath, "utf8");
        expect(logPath.startsWith(expectedRoot)).toBe(true);
        expect(contents).toContain("\"event\":\"tool.request\"");
        expect(contents).toContain("sk-ant-***REDACTED***");
        expect(contents).not.toContain("abcdefghijklmnopqrstuvwxyz123456");
    });
    it("ignores trace dir overrides and keeps traces under the safe root", () => {
        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "ralphx-mcp-trace-"));
        const expectedRoot = path.resolve(process.cwd(), ".artifacts/logs/mcp-proxy");
        process.env.RALPHX_MCP_TRACE_DIR = tempDir;
        safeTrace("tool.request", {
            api_key: "sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456",
        });
        const logPath = getTraceLogPath();
        expect(logPath.startsWith(expectedRoot)).toBe(true);
        expect(logPath.startsWith(tempDir)).toBe(false);
    });
});
//# sourceMappingURL=redact.test.js.map