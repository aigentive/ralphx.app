import { describe, it, expect } from "vitest";
import { parseToolResultAsLines } from "./shared.constants";

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
