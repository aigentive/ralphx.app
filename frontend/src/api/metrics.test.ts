import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getChatAttributionBackfillSummary,
  type AttributionBackfillSummary,
} from "./metrics";

describe("metrics API", () => {
  it("loads chat attribution backfill summary", async () => {
    const raw: AttributionBackfillSummary = {
      eligibleConversationCount: 12,
      pendingCount: 3,
      runningCount: 1,
      completedCount: 6,
      partialCount: 1,
      sessionNotFoundCount: 1,
      parseFailedCount: 0,
      remainingCount: 4,
      terminalCount: 8,
      attentionCount: 2,
      isIdle: false,
    };
    vi.mocked(invoke).mockResolvedValueOnce(raw);

    const result = await getChatAttributionBackfillSummary();

    expect(invoke).toHaveBeenCalledWith("get_chat_attribution_backfill_summary", {});
    expect(result).toEqual(raw);
  });
});
