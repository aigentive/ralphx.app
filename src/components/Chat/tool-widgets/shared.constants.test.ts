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
  it("returns empty string for empty input", () => {
    expect(normalizeDisplayPath("")).toBe("");
  });

  it("converts backslashes to forward slashes", () => {
    expect(normalizeDisplayPath("src\\components\\App.tsx")).toBe("src/components/App.tsx");
  });

  it("removes absolute prefix and anchors at known project segment", () => {
    expect(normalizeDisplayPath("/Users/dev/Code/project/src/app.ts")).toBe("src/app.ts");
  });

  it("anchors at src-tauri", () => {
    expect(normalizeDisplayPath("/home/user/project/src-tauri/src/main.rs")).toBe("src-tauri/src/main.rs");
  });

  it("anchors at tests", () => {
    expect(normalizeDisplayPath("/workspace/tests/unit/test.ts")).toBe("tests/unit/test.ts");
  });

  it("anchors at specs", () => {
    expect(normalizeDisplayPath("/workspace/specs/plan.md")).toBe("specs/plan.md");
  });

  it("anchors at .claude", () => {
    expect(normalizeDisplayPath("/workspace/.claude/rules/foo.md")).toBe(".claude/rules/foo.md");
  });

  it("removes .../  prefix artifacts", () => {
    expect(normalizeDisplayPath(".../src/app.ts")).toBe("src/app.ts");
  });

  it("removes /.../ prefix artifacts", () => {
    expect(normalizeDisplayPath("/.../src/app.ts")).toBe("src/app.ts");
  });

  it("returns relative path as-is when already relative", () => {
    expect(normalizeDisplayPath("src/components/App.tsx")).toBe("src/components/App.tsx");
  });

  it("falls back to filename for unknown absolute paths", () => {
    expect(normalizeDisplayPath("/unknown/path/file.txt")).toBe("file.txt");
  });

  it("prefers earliest anchor in path", () => {
    // "src" comes before "tests" in the segments
    expect(normalizeDisplayPath("/project/src/tests/foo.ts")).toBe("src/tests/foo.ts");
  });
});

describe("shortenPath", () => {
  it("returns path unchanged if under max length", () => {
    expect(shortenPath("src/app.ts", 50)).toBe("src/app.ts");
  });

  it("shortens long paths by keeping first and last two segments", () => {
    const long = "src/components/deeply/nested/path/Component.tsx";
    const result = shortenPath(long, 30);
    expect(result).toContain("Component.tsx");
    expect(result.length).toBeLessThanOrEqual(30);
  });

  it("returns just filename as last resort", () => {
    const result = shortenPath("a/b/c/d/e/f/VeryLongComponentName.tsx", 10);
    expect(result).toBe("VeryLongComponentName.tsx");
  });
});

describe("parseSearchResult", () => {
  it("returns empty for empty input", () => {
    const result = parseSearchResult("");
    expect(result.paths).toEqual([]);
    expect(result.isEmpty).toBe(true);
  });

  it("returns empty for null/undefined", () => {
    const result = parseSearchResult(null);
    expect(result.paths).toEqual([]);
    expect(result.isEmpty).toBe(true);
  });

  it("extracts plain file paths", () => {
    const result = parseSearchResult("src/app.ts\nsrc/utils.ts");
    expect(result.paths).toEqual(["src/app.ts", "src/utils.ts"]);
    expect(result.isEmpty).toBe(false);
  });

  it("extracts file path from path:line:content format", () => {
    const result = parseSearchResult("src/app.ts:10:import React\nsrc/app.ts:20:export default\nsrc/utils.ts:5:export function");
    expect(result.paths).toEqual(["src/app.ts", "src/utils.ts"]);
  });

  it("deduplicates paths", () => {
    const result = parseSearchResult("src/app.ts\nsrc/app.ts\nsrc/utils.ts");
    expect(result.paths).toEqual(["src/app.ts", "src/utils.ts"]);
  });

  it("skips metadata lines like 'Found N files'", () => {
    const result = parseSearchResult("Found 5 files\nsrc/app.ts\nsrc/utils.ts");
    expect(result.paths).toEqual(["src/app.ts", "src/utils.ts"]);
  });

  it("treats 'No matches found' as empty with note", () => {
    const result = parseSearchResult("No matches found");
    expect(result.isEmpty).toBe(true);
    expect(result.paths).toEqual([]);
    expect(result.note).toBe("No matches found");
  });

  it("treats 'No files found' as empty with note", () => {
    const result = parseSearchResult("No files found");
    expect(result.isEmpty).toBe(true);
    expect(result.note).toBe("No files found");
  });

  it("treats 'No files matched' as empty with note", () => {
    const result = parseSearchResult("No files matched");
    expect(result.isEmpty).toBe(true);
    expect(result.note).toBe("No files matched");
  });

  it("normalizes absolute paths to repo-relative", () => {
    const result = parseSearchResult("/Users/dev/project/src/app.ts\n/Users/dev/project/src/utils.ts");
    expect(result.paths).toEqual(["src/app.ts", "src/utils.ts"]);
  });

  it("handles MCP wrapper format", () => {
    const result = parseSearchResult([{ type: "text", text: "src/a.ts\nsrc/b.ts" }]);
    expect(result.paths).toEqual(["src/a.ts", "src/b.ts"]);
  });
});

describe("parseReadOutput", () => {
  it("returns empty for null/undefined", () => {
    const result = parseReadOutput(null);
    expect(result.lines).toEqual([]);
    expect(result.inferredStartLine).toBe(1);
  });

  it("strips line-number prefixes (N→ format)", () => {
    const raw = "     1→import React from 'react';\n     2→\n     3→export default function App() {";
    const result = parseReadOutput(raw);
    expect(result.lines).toEqual([
      "import React from 'react';",
      "",
      "export default function App() {",
    ]);
  });

  it("infers start line from first prefix when offset is missing", () => {
    const raw = "    50→function hello() {\n    51→  return 'world';\n    52→}";
    const result = parseReadOutput(raw);
    expect(result.inferredStartLine).toBe(50);
  });

  it("uses explicit offset over inferred start line", () => {
    const raw = "    50→function hello() {\n    51→  return 'world';";
    const result = parseReadOutput(raw, 50);
    expect(result.inferredStartLine).toBe(50);
  });

  it("extracts error from tool_use_error XML wrapper", () => {
    const raw = "<tool_use_error>File not found: src/missing.ts</tool_use_error>";
    const result = parseReadOutput(raw);
    expect(result.error).toBe("File not found: src/missing.ts");
    expect(result.lines).toEqual([]);
  });

  it("preserves code indentation after stripping prefix", () => {
    const raw = "     1→function foo() {\n     2→  const x = 1;\n     3→    return x;\n     4→}";
    const result = parseReadOutput(raw);
    expect(result.lines[1]).toBe("  const x = 1;");
    expect(result.lines[2]).toBe("    return x;");
  });

  it("handles MCP wrapper format", () => {
    const raw = [{ type: "text", text: "     1→const a = 1;\n     2→const b = 2;" }];
    const result = parseReadOutput(raw);
    expect(result.lines).toEqual(["const a = 1;", "const b = 2;"]);
  });

  it("passes through lines without prefix as-is", () => {
    const raw = "no prefix here\nalso no prefix";
    const result = parseReadOutput(raw);
    expect(result.lines).toEqual(["no prefix here", "also no prefix"]);
    expect(result.inferredStartLine).toBe(1);
  });

  it("defaults to startLine 1 when no prefix and no offset", () => {
    const result = parseReadOutput("plain text");
    expect(result.inferredStartLine).toBe(1);
  });
});
