import { describe, expect, it } from "vitest";
import { buildTaskCardSummaryParts, getTaskCardKindLabel } from "./TaskCardShared.utils";

describe("TaskCardShared", () => {
  it("classifies task card kind labels across delegate, agent, and task names", () => {
    expect(getTaskCardKindLabel("delegate_start")).toBe("Delegate");
    expect(getTaskCardKindLabel("ralphx::delegate_start")).toBe("Delegate");
    expect(getTaskCardKindLabel("Agent")).toBe("Agent");
    expect(getTaskCardKindLabel("Task")).toBe("Task");
  });

  it("builds summary parts from duration, usage, tool count, and cost", () => {
    expect(
      buildTaskCardSummaryParts({
        totalDurationMs: 6200,
        totalTokens: 1532,
        totalToolUseCount: 3,
        estimatedUsd: 0.43,
      }),
    ).toEqual(["6s", "1,532 tokens", "3 tools", "$0.43"]);
  });

  it("omits absent summary parts cleanly", () => {
    expect(
      buildTaskCardSummaryParts({
        totalTokens: 12,
      }),
    ).toEqual(["12 tokens"]);
  });
});
