import { describe, it, expect } from "vitest";
import {
  parseToolResultAsLines,
  normalizeDisplayPath,
  shortenPath,
  parseSearchResult,
  parseReadOutput,
} from "./shared.constants";

describe("parseToolResultAsLines", () => {
  it("returns empty array for null/undefined", () => {
    expect(parseToolResultAsLines(null)).toEqual([]);
    expect(parseToolResultAsLines(undefined)).toEqual([]);
  });

  it("returns empty array for empty string", () => {
    expect(parseToolResultAsLines("")).toEqual([]);
  });

  it("splits plain string by newlines", () => {
    expect(parseToolResultAsLines("a.ts\nb.ts\nc.ts")).toEqual([
      "a.ts",
      "b.ts",
      "c.ts",
    ]);
  });

  it("trims whitespace from each line", () => {
    expect(parseToolResultAsLines("  a.ts  \n  b.ts  ")).toEqual([
      "a.ts",
      "b.ts",
    ]);
  });

  it("filters empty lines", () => {
    expect(parseToolResultAsLines("a.ts\n\n\nb.ts\n")).toEqual([
      "a.ts",
      "b.ts",
    ]);
  });

  it("parses MCP wrapper [{type: 'text', text: '...'}]", () => {
    const result = [{ type: "text", text: "mcp1.ts\nmcp2.ts" }];
    expect(parseToolResultAsLines(result)).toEqual(["mcp1.ts", "mcp2.ts"]);
  });

  it("parses object with text property", () => {
    const result = { text: "obj1.ts\nobj2.ts" };
    expect(parseToolResultAsLines(result)).toEqual(["obj1.ts", "obj2.ts"]);
  });

  it("parses string array directly", () => {
    expect(parseToolResultAsLines(["a.ts", "b.ts"])).toEqual(["a.ts", "b.ts"]);
  });

  it("filters non-string items from arrays", () => {
    expect(parseToolResultAsLines(["a.ts", 42, "b.ts", null])).toEqual([
      "a.ts",
      "b.ts",
    ]);
  });

  it("returns empty array for empty MCP text", () => {
    expect(parseToolResultAsLines([{ type: "text", text: "" }])).toEqual([]);
  });
});

describe("normalizeDisplayPath", () => {
  it("converts backslashes to forward slashes", () => {
    expect(normalizeDisplayPath("src\\components\\Chat.tsx")).toBe(
      "src/components/Chat.tsx",
    );
  });

  it("removes absolute prefix for known project segments", () => {
    expect(
      normalizeDisplayPath(
        "/Users/test/ralphx-worktrees/ralphx/task-abc/src/components/Chat.tsx",
      ),
    ).toBe("src/components/Chat.tsx");
  });

  it("handles src-tauri prefix", () => {
    expect(
      normalizeDisplayPath("/some/path/src-tauri/src/main.rs"),
    ).toBe("src-tauri/src/main.rs");
  });

  it("handles tests prefix", () => {
    expect(normalizeDisplayPath("/workspace/tests/unit/test.ts")).toBe(
      "tests/unit/test.ts",
    );
  });

  it("handles specs prefix", () => {
    expect(normalizeDisplayPath("/Users/me/project/specs/plan.md")).toBe(
      "specs/plan.md",
    );
  });

  it("handles scripts prefix", () => {
    expect(normalizeDisplayPath("/root/scripts/build.sh")).toBe(
      "scripts/build.sh",
    );
  });

  it("handles docs prefix", () => {
    expect(normalizeDisplayPath("/project/docs/api.md")).toBe("docs/api.md");
  });

  it("handles mockups prefix", () => {
    expect(normalizeDisplayPath("/work/mockups/design.html")).toBe(
      "mockups/design.html",
    );
  });

  it("handles assets prefix", () => {
    expect(normalizeDisplayPath("/app/assets/logo.png")).toBe(
      "assets/logo.png",
    );
  });

  it("handles public prefix", () => {
    expect(normalizeDisplayPath("/site/public/index.html")).toBe(
      "public/index.html",
    );
  });

  it("removes leading /.../ artifacts", () => {
    expect(normalizeDisplayPath("/.../src/main.ts")).toBe("src/main.ts");
  });

  it("removes leading .../ artifacts", () => {
    expect(normalizeDisplayPath(".../src/main.ts")).toBe("src/main.ts");
  });

  it("falls back to basename when no anchor found", () => {
    expect(normalizeDisplayPath("/unknown/path/to/file.txt")).toBe(
      "file.txt",
    );
  });

  it("returns path as-is if already repo-relative", () => {
    expect(normalizeDisplayPath("src/components/Chat.tsx")).toBe(
      "src/components/Chat.tsx",
    );
  });

  it("handles empty path", () => {
    expect(normalizeDisplayPath("")).toBe("");
  });
});

describe("shortenPath", () => {
  it("returns path unchanged if under maxLength", () => {
    expect(shortenPath("src/components/Chat.tsx", 100)).toBe(
      "src/components/Chat.tsx",
    );
  });

  it("collapses middle directories with .../", () => {
    expect(shortenPath("src/components/nested/deep/Chat.tsx", 25)).toBe(
      "src/.../deep/Chat.tsx",
    );
  });

  it("shows only filename if still too long", () => {
    expect(shortenPath("src/components/Chat.tsx", 10)).toBe(".../Chat.tsx");
  });

  it("never produces /.../", () => {
    const result = shortenPath("src/very/deep/nested/path/file.tsx", 20);
    expect(result).not.toContain("/.../");
    expect(result).toMatch(/^(src\/\.\.\/|\.\.\.\/)/);
  });

  it("handles paths with only 2 parts", () => {
    expect(shortenPath("src/main.ts", 5)).toBe("src/main.ts");
  });

  it("handles single-part paths", () => {
    expect(shortenPath("file.txt", 5)).toBe("file.txt");
  });
});

describe("parseSearchResult", () => {
  it("parses grep output with path:line:match format", () => {
    const result = "src/main.ts:42:const foo = bar;\nsrc/utils.ts:100:export";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/main.ts", "src/utils.ts"]);
    expect(parsed.isEmpty).toBe(false);
    expect(parsed.note).toBeUndefined();
  });

  it("filters out metadata lines", () => {
    const result =
      "Found 3 files matching pattern\nsrc/a.ts:1:import\nsrc/b.ts:2:export\nPage 1 of 1";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/a.ts", "src/b.ts"]);
  });

  it("detects no-match states", () => {
    const result = "No matches found";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual([]);
    expect(parsed.isEmpty).toBe(true);
    expect(parsed.note).toBe("No matches found");
  });

  it("detects no files found", () => {
    const result = "No files found";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual([]);
    expect(parsed.isEmpty).toBe(true);
    expect(parsed.note).toBe("No files found");
  });

  it("detects no files matched", () => {
    const result = "No files matched the pattern";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual([]);
    expect(parsed.isEmpty).toBe(true);
    expect(parsed.note).toContain("No files matched");
  });

  it("deduplicates paths", () => {
    const result = "src/main.ts:1:a\nsrc/main.ts:2:b\nsrc/main.ts:3:c";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/main.ts"]);
  });

  it("normalizes extracted paths", () => {
    const result =
      "/Users/me/project/src/main.ts:1:code\n/Users/me/project/src-tauri/src/lib.rs:5:rust";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/main.ts", "src-tauri/src/lib.rs"]);
  });

  it("handles MCP wrapper format", () => {
    const result = [{ type: "text", text: "src/a.ts:1:code\nsrc/b.ts:2:more" }];
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/a.ts", "src/b.ts"]);
  });

  it("handles empty result", () => {
    const parsed = parseSearchResult("");
    expect(parsed.paths).toEqual([]);
    expect(parsed.isEmpty).toBe(false);
    expect(parsed.note).toBeUndefined();
  });

  it("handles plain file list (glob output)", () => {
    const result = "src/a.ts\nsrc/b.ts\nsrc/c.ts";
    const parsed = parseSearchResult(result);
    expect(parsed.paths).toEqual(["src/a.ts", "src/b.ts", "src/c.ts"]);
  });
});

describe("parseReadOutput", () => {
  it("strips line-number prefixes like '   500→'", () => {
    const result = "   500→const foo = 'bar';\n   501→const baz = 'qux';";
    const parsed = parseReadOutput(result);
    expect(parsed.lines).toEqual([
      "const foo = 'bar';",
      "const baz = 'qux';",
    ]);
    expect(parsed.inferredStartLine).toBe(500);
  });

  it("preserves code indentation after arrow", () => {
    const result =
      "     1→function test() {\n     2→  return true;\n     3→}";
    const parsed = parseReadOutput(result);
    expect(parsed.lines).toEqual([
      "function test() {",
      "  return true;",
      "}",
    ]);
  });

  it("infers start line from first prefix", () => {
    const result = "    42→first line\n    43→second line";
    const parsed = parseReadOutput(result);
    expect(parsed.inferredStartLine).toBe(42);
  });

  it("uses provided offset when available", () => {
    const result = "     1→line one\n     2→line two";
    const parsed = parseReadOutput(result, 100);
    expect(parsed.inferredStartLine).toBe(100);
  });

  it("extracts error from <tool_use_error> tags", () => {
    const result = "<tool_use_error>File not found: missing.txt</tool_use_error>";
    const parsed = parseReadOutput(result);
    expect(parsed.error).toBe("File not found: missing.txt");
    expect(parsed.lines).toEqual([]);
  });

  it("handles plain text without prefixes", () => {
    const result = "plain line 1\nplain line 2";
    const parsed = parseReadOutput(result);
    expect(parsed.lines).toEqual(["plain line 1", "plain line 2"]);
    expect(parsed.inferredStartLine).toBe(1);
  });

  it("handles MCP wrapper format", () => {
    const result = [
      { type: "text", text: "     1→const x = 1;\n     2→const y = 2;" },
    ];
    const parsed = parseReadOutput(result);
    expect(parsed.lines).toEqual(["const x = 1;", "const y = 2;"]);
    expect(parsed.inferredStartLine).toBe(1);
  });

  it("handles empty result", () => {
    const parsed = parseReadOutput("");
    expect(parsed.lines).toEqual([]);
    expect(parsed.inferredStartLine).toBe(1);
    expect("error" in parsed).toBe(false);
  });

  it("filters empty lines from output", () => {
    const result = "     1→line one\n     2→\n     3→line three";
    const parsed = parseReadOutput(result);
    expect(parsed.lines).toEqual(["line one", "", "line three"]);
  });
});
