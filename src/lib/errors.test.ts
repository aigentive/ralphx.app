import { describe, it, expect } from "vitest";
import { extractErrorMessage } from "./errors";

const FALLBACK = "Something went wrong";

describe("extractErrorMessage", () => {
  describe("precedence 1: Error objects", () => {
    it("should extract message from Error instance", () => {
      const err = new Error("connection refused");
      expect(extractErrorMessage(err, FALLBACK)).toBe("connection refused");
    });

    it("should extract message from TypeError", () => {
      const err = new TypeError("cannot read property");
      expect(extractErrorMessage(err, FALLBACK)).toBe("cannot read property");
    });

    it("should trim whitespace from Error message", () => {
      const err = new Error("  spaced message  ");
      expect(extractErrorMessage(err, FALLBACK)).toBe("spaced message");
    });

    it("should return fallback for Error with empty message", () => {
      const err = new Error("");
      expect(extractErrorMessage(err, FALLBACK)).toBe(FALLBACK);
    });

    it("should return fallback for Error with whitespace-only message", () => {
      const err = new Error("   ");
      expect(extractErrorMessage(err, FALLBACK)).toBe(FALLBACK);
    });
  });

  describe("precedence 2: string values", () => {
    it("should return string error directly", () => {
      expect(extractErrorMessage("Git merge failed", FALLBACK)).toBe(
        "Git merge failed",
      );
    });

    it("should trim whitespace from string error", () => {
      expect(extractErrorMessage("  trimmed  ", FALLBACK)).toBe("trimmed");
    });

    it("should return fallback for empty string", () => {
      expect(extractErrorMessage("", FALLBACK)).toBe(FALLBACK);
    });

    it("should return fallback for whitespace-only string", () => {
      expect(extractErrorMessage("   ", FALLBACK)).toBe(FALLBACK);
    });
  });

  describe("precedence 3: plain objects with known fields", () => {
    it("should extract .message from plain object", () => {
      expect(
        extractErrorMessage({ message: "conflict in src/main.rs" }, FALLBACK),
      ).toBe("conflict in src/main.rs");
    });

    it("should extract .error from plain object", () => {
      expect(
        extractErrorMessage({ error: "permission denied" }, FALLBACK),
      ).toBe("permission denied");
    });

    it("should extract .cause.message from plain object", () => {
      expect(
        extractErrorMessage(
          { cause: { message: "nested cause" } },
          FALLBACK,
        ),
      ).toBe("nested cause");
    });

    it("should prefer .message over .error", () => {
      expect(
        extractErrorMessage(
          { message: "primary", error: "secondary" },
          FALLBACK,
        ),
      ).toBe("primary");
    });

    it("should prefer .error over .cause.message", () => {
      expect(
        extractErrorMessage(
          { error: "direct", cause: { message: "nested" } },
          FALLBACK,
        ),
      ).toBe("direct");
    });

    it("should skip empty .message and try .error", () => {
      expect(
        extractErrorMessage({ message: "", error: "fallback field" }, FALLBACK),
      ).toBe("fallback field");
    });

    it("should skip whitespace .message and try .error", () => {
      expect(
        extractErrorMessage(
          { message: "  ", error: "next field" },
          FALLBACK,
        ),
      ).toBe("next field");
    });

    it("should trim values from plain object fields", () => {
      expect(
        extractErrorMessage({ message: "  padded  " }, FALLBACK),
      ).toBe("padded");
    });
  });

  describe("precedence 4: JSON serializable objects", () => {
    it("should JSON.stringify unknown object shapes", () => {
      const obj = { code: 500, detail: "internal" };
      expect(extractErrorMessage(obj, FALLBACK)).toBe(
        JSON.stringify(obj),
      );
    });

    it("should JSON.stringify arrays", () => {
      const arr = ["error1", "error2"];
      expect(extractErrorMessage(arr, FALLBACK)).toBe(
        JSON.stringify(arr),
      );
    });

    it("should return fallback for empty object", () => {
      expect(extractErrorMessage({}, FALLBACK)).toBe(FALLBACK);
    });
  });

  describe("precedence 5: fallback", () => {
    it("should return fallback for null", () => {
      expect(extractErrorMessage(null, FALLBACK)).toBe(FALLBACK);
    });

    it("should return fallback for undefined", () => {
      expect(extractErrorMessage(undefined, FALLBACK)).toBe(FALLBACK);
    });

    it("should return fallback for number", () => {
      expect(extractErrorMessage(42, FALLBACK)).toBe(FALLBACK);
    });

    it("should return fallback for boolean", () => {
      expect(extractErrorMessage(true, FALLBACK)).toBe(FALLBACK);
    });
  });

  describe("edge cases", () => {
    it("should handle circular object references gracefully", () => {
      const circular: Record<string, unknown> = { a: 1 };
      circular.self = circular;
      expect(extractErrorMessage(circular, FALLBACK)).toBe(FALLBACK);
    });

    it("should handle object with non-string .message", () => {
      expect(extractErrorMessage({ message: 123 }, FALLBACK)).toBe(
        JSON.stringify({ message: 123 }),
      );
    });

    it("should handle object with non-string .error", () => {
      expect(extractErrorMessage({ error: true }, FALLBACK)).toBe(
        JSON.stringify({ error: true }),
      );
    });

    it("should use custom fallback string", () => {
      expect(extractErrorMessage(null, "Custom fallback")).toBe(
        "Custom fallback",
      );
    });

    it("should handle object with null .cause", () => {
      expect(
        extractErrorMessage({ cause: null }, FALLBACK),
      ).toBe(JSON.stringify({ cause: null }));
    });

    it("should handle object with .cause but no .message on cause", () => {
      expect(
        extractErrorMessage({ cause: { code: 500 } }, FALLBACK),
      ).toBe(JSON.stringify({ cause: { code: 500 } }));
    });
  });
});
