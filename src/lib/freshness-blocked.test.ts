import { describe, it, expect } from "vitest";
import { parseFreshnessBlockedReason, FRESHNESS_BLOCKED_PREFIX } from "./freshness-blocked";

describe("parseFreshnessBlockedReason", () => {
  it("parses a valid freshness blocked reason", () => {
    const reason = "FRESHNESS_BLOCKED|3|12|src/foo.ts,src/bar.ts|Branch is behind main";
    const result = parseFreshnessBlockedReason(reason);
    expect(result).toEqual({
      totalAttempts: 3,
      elapsedMinutes: 12,
      conflictFiles: ["src/foo.ts", "src/bar.ts"],
      message: "Branch is behind main",
    });
  });

  it("returns null for non-FRESHNESS_BLOCKED prefix", () => {
    expect(parseFreshnessBlockedReason("SOME_OTHER|1|2|file.ts|msg")).toBeNull();
    expect(parseFreshnessBlockedReason("")).toBeNull();
    expect(parseFreshnessBlockedReason("plain error message")).toBeNull();
  });

  it("returns null when missing separators (only prefix)", () => {
    expect(parseFreshnessBlockedReason(FRESHNESS_BLOCKED_PREFIX)).toBeNull();
  });

  it("returns null when missing second separator", () => {
    expect(parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3")).toBeNull();
  });

  it("returns null when missing third separator", () => {
    expect(parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3|12")).toBeNull();
  });

  it("returns null when missing fourth separator", () => {
    expect(parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3|12|file.ts")).toBeNull();
  });

  it("defaults totalAttempts to 0 on NaN", () => {
    const result = parseFreshnessBlockedReason("FRESHNESS_BLOCKED|abc|12|file.ts|msg");
    expect(result).not.toBeNull();
    expect(result!.totalAttempts).toBe(0);
  });

  it("defaults elapsedMinutes to 0 on NaN", () => {
    const result = parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3|xyz|file.ts|msg");
    expect(result).not.toBeNull();
    expect(result!.elapsedMinutes).toBe(0);
  });

  it("returns empty conflictFiles array when files field is empty string", () => {
    const result = parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3|12||msg");
    expect(result).not.toBeNull();
    expect(result!.conflictFiles).toEqual([]);
  });

  it("captures message correctly when message contains pipe characters", () => {
    const result = parseFreshnessBlockedReason(
      "FRESHNESS_BLOCKED|3|12|file.ts|message with | pipe | chars"
    );
    expect(result).not.toBeNull();
    expect(result!.message).toBe("message with | pipe | chars");
  });

  it("handles single conflict file", () => {
    const result = parseFreshnessBlockedReason("FRESHNESS_BLOCKED|1|5|src/only.ts|error");
    expect(result).not.toBeNull();
    expect(result!.conflictFiles).toEqual(["src/only.ts"]);
  });

  it("handles message that is empty string", () => {
    const result = parseFreshnessBlockedReason("FRESHNESS_BLOCKED|3|12|file.ts|");
    expect(result).not.toBeNull();
    expect(result!.message).toBe("");
  });

  it("handles many conflict files", () => {
    const files = ["a.ts", "b.ts", "c.ts", "d.rs", "e.tsx"];
    const result = parseFreshnessBlockedReason(
      `FRESHNESS_BLOCKED|5|30|${files.join(",")}|some message`
    );
    expect(result).not.toBeNull();
    expect(result!.conflictFiles).toEqual(files);
  });
});
