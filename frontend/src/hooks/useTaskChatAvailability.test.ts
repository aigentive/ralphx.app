import { describe, expect, it } from "vitest";
import { shouldShowTaskChatForState } from "./useTaskChatAvailability";
import type { InternalStatus } from "@/types/status";

describe("shouldShowTaskChatForState", () => {
  it.each([
    "executing",
    "re_executing",
    "qa_refining",
    "qa_testing",
  ] as InternalStatus[])("shows live execution chat for %s", (status) => {
    expect(shouldShowTaskChatForState({ status })).toBe(true);
  });

  it.each(["qa_passed", "qa_failed"] as InternalStatus[])(
    "shows %s only when an execution conversation exists",
    (status) => {
      expect(shouldShowTaskChatForState({ status })).toBe(false);
      expect(
        shouldShowTaskChatForState({
          status,
          executionConversationCount: 1,
        })
      ).toBe(true);
    }
  );

  it.each(["pending_review", "reviewing"] as InternalStatus[])(
    "shows live review chat for %s",
    (status) => {
      expect(shouldShowTaskChatForState({ status })).toBe(true);
    }
  );

  it.each(["review_passed", "escalated", "approved"] as InternalStatus[])(
    "shows %s only when a review conversation exists",
    (status) => {
      expect(shouldShowTaskChatForState({ status })).toBe(false);
      expect(
        shouldShowTaskChatForState({
          status,
          reviewConversationCount: 1,
        })
      ).toBe(true);
    }
  );

  it.each([
    "pending_merge",
    "merging",
    "waiting_on_pr",
    "merge_incomplete",
    "merge_conflict",
    "merged",
  ] as InternalStatus[])("shows merge chat for %s only when a merge conversation exists", (status) => {
    expect(shouldShowTaskChatForState({ status })).toBe(false);
    expect(
      shouldShowTaskChatForState({
        status,
        mergeConversationCount: 1,
      })
    ).toBe(true);
  });

  it("shows live chat when the matching agent is running", () => {
    expect(shouldShowTaskChatForState({ executionAgentRunning: true })).toBe(true);
    expect(shouldShowTaskChatForState({ reviewAgentRunning: true })).toBe(true);
    expect(shouldShowTaskChatForState({ mergeAgentRunning: true })).toBe(true);
  });

  it("shows historical chat only when the timeline state carries a conversation id", () => {
    expect(
      shouldShowTaskChatForState({
        status: "merged",
        isHistoryMode: true,
        hasHistoryConversation: false,
        mergeConversationCount: 1,
      })
    ).toBe(false);
    expect(
      shouldShowTaskChatForState({
        status: "merged",
        isHistoryMode: true,
        hasHistoryConversation: true,
      })
    ).toBe(true);
  });

  it("does not show chat for idle task states", () => {
    expect(shouldShowTaskChatForState({ status: "backlog" })).toBe(false);
    expect(shouldShowTaskChatForState({ status: "ready" })).toBe(false);
    expect(shouldShowTaskChatForState({ status: "blocked" })).toBe(false);
  });
});
