import { describe, it, expect } from "vitest";
import {
  safeArrayAccess,
  isNotNull,
  assertDefined,
  getStatusLabel,
  createConfig,
} from "./validation";

describe("validation utilities", () => {
  describe("safeArrayAccess", () => {
    it("returns element at valid index", () => {
      const arr = ["a", "b", "c"];
      const result = safeArrayAccess(arr, 1);
      // With noUncheckedIndexedAccess, result is T | undefined
      expect(result).toBe("b");
    });

    it("returns undefined for out of bounds index", () => {
      const arr = ["a", "b", "c"];
      const result = safeArrayAccess(arr, 10);
      expect(result).toBeUndefined();
    });

    it("returns undefined for negative index", () => {
      const arr = ["a", "b", "c"];
      const result = safeArrayAccess(arr, -1);
      expect(result).toBeUndefined();
    });
  });

  describe("isNotNull", () => {
    it("returns true for defined values", () => {
      expect(isNotNull("hello")).toBe(true);
      expect(isNotNull(0)).toBe(true);
      expect(isNotNull(false)).toBe(true);
      expect(isNotNull([])).toBe(true);
    });

    it("returns false for null", () => {
      expect(isNotNull(null)).toBe(false);
    });

    it("returns false for undefined", () => {
      expect(isNotNull(undefined)).toBe(false);
    });

    it("works as type guard", () => {
      const values: (string | null)[] = ["a", null, "b", null];
      const filtered = values.filter(isNotNull);
      // filtered should be string[], not (string | null)[]
      expect(filtered).toEqual(["a", "b"]);
    });
  });

  describe("assertDefined", () => {
    it("does not throw for defined values", () => {
      expect(() => assertDefined("test", "Should not throw")).not.toThrow();
      expect(() => assertDefined(0, "Should not throw")).not.toThrow();
      expect(() => assertDefined(false, "Should not throw")).not.toThrow();
    });

    it("throws for null", () => {
      expect(() => assertDefined(null, "Value is null")).toThrow("Value is null");
    });

    it("throws for undefined", () => {
      expect(() => assertDefined(undefined, "Value is undefined")).toThrow(
        "Value is undefined"
      );
    });
  });

  describe("getStatusLabel", () => {
    it("returns correct label for known statuses", () => {
      expect(getStatusLabel("backlog")).toBe("Backlog");
      expect(getStatusLabel("ready")).toBe("Ready");
      expect(getStatusLabel("executing")).toBe("Executing");
      expect(getStatusLabel("completed")).toBe("Completed");
    });

    it("returns Unknown for unrecognized status", () => {
      expect(getStatusLabel("invalid")).toBe("Unknown");
      expect(getStatusLabel("")).toBe("Unknown");
    });
  });

  describe("createConfig", () => {
    it("creates config with name only", () => {
      const config = createConfig("test");
      expect(config.name).toBe("test");
      expect(config.description).toBeUndefined();
    });

    it("creates config with name and description", () => {
      const config = createConfig("test", "A test config");
      expect(config.name).toBe("test");
      expect(config.description).toBe("A test config");
    });

    it("handles empty string description", () => {
      const config = createConfig("test", "");
      expect(config.name).toBe("test");
      expect(config.description).toBe("");
    });
  });
});
