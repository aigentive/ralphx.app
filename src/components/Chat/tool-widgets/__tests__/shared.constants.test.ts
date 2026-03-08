import { describe, it, expect } from "vitest";
import { parseMcpToolResult } from "../shared.constants";

describe("parseMcpToolResult", () => {
  it("unwraps MCP content array to parsed object", () => {
    const input = [{ type: "text", text: '{"title":"Hello"}' }];
    expect(parseMcpToolResult(input)).toEqual({ title: "Hello" });
  });

  it("passes through a plain object unchanged", () => {
    const input = { title: "Hello" };
    expect(parseMcpToolResult(input)).toEqual({ title: "Hello" });
  });

  it("returns empty object for empty array", () => {
    expect(parseMcpToolResult([])).toEqual({});
  });

  it("returns empty object for malformed JSON in text", () => {
    const input = [{ type: "text", text: "not json" }];
    expect(parseMcpToolResult(input)).toEqual({});
  });

  it("returns empty object for null", () => {
    expect(parseMcpToolResult(null)).toEqual({});
  });

  it("returns empty object for undefined", () => {
    expect(parseMcpToolResult(undefined)).toEqual({});
  });

  it("returns empty object for non-text content type", () => {
    const input = [{ type: "image", data: "base64data" }];
    expect(parseMcpToolResult(input)).toEqual({});
  });

  it("uses only the first content block when multiple exist", () => {
    const input = [
      { type: "text", text: '{"a":1}' },
      { type: "text", text: '{"b":2}' },
    ];
    expect(parseMcpToolResult(input)).toEqual({ a: 1 });
  });

  it("returns empty object for array of strings (non-MCP array)", () => {
    const input = ["hello", "world"];
    expect(parseMcpToolResult(input)).toEqual({});
  });

  it("returns empty object for array of numbers", () => {
    const input = [1, 2, 3];
    expect(parseMcpToolResult(input)).toEqual({});
  });
});
