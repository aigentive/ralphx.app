import { describe, it, expect } from "vitest";
import { transformNullableBool } from "./ideation.transforms";

describe("transformNullableBool", () => {
  it("returns null for null input", () => {
    expect(transformNullableBool(null)).toBeNull();
  });

  it("returns null for undefined input", () => {
    expect(transformNullableBool(undefined)).toBeNull();
  });

  it("returns false for 0", () => {
    expect(transformNullableBool(0)).toBe(false);
  });

  it("returns true for 1", () => {
    expect(transformNullableBool(1)).toBe(true);
  });

  it("returns true for any non-zero number", () => {
    expect(transformNullableBool(2)).toBe(true);
    expect(transformNullableBool(-1)).toBe(true);
  });
});
