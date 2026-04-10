import { describe, expect, it } from "vitest";
import { buildFallbackConversationStats } from "./useConversationStats";
import type { ChatConversation } from "@/types/chat-conversation";
import type { ChatMessageResponse } from "@/api/chat";

const conversation: ChatConversation = {
  id: "conv-1",
  contextType: "ideation",
  contextId: "session-1",
  claudeSessionId: null,
  providerSessionId: "provider-session-1",
  providerHarness: "codex",
  upstreamProvider: "openai",
  providerProfile: null,
  title: null,
  messageCount: 3,
  lastMessageAt: "2026-04-10T10:05:00.000Z",
  createdAt: "2026-04-10T10:00:00.000Z",
  updatedAt: "2026-04-10T10:05:00.000Z",
};

function message(overrides: Partial<ChatMessageResponse>): ChatMessageResponse {
  return {
    id: "msg",
    sessionId: null,
    projectId: null,
    taskId: null,
    role: "orchestrator",
    content: "",
    metadata: null,
    parentMessageId: null,
    conversationId: "conv-1",
    toolCalls: null,
    contentBlocks: null,
    sender: null,
    attributionSource: "native",
    providerHarness: "codex",
    providerSessionId: "provider-session-1",
    upstreamProvider: "openai",
    providerProfile: null,
    logicalModel: "gpt-5.4",
    effectiveModelId: "gpt-5.4",
    logicalEffort: "high",
    effectiveEffort: "xhigh",
    inputTokens: null,
    outputTokens: null,
    cacheCreationTokens: null,
    cacheReadTokens: null,
    estimatedUsd: null,
    createdAt: "2026-04-10T10:00:00.000Z",
    ...overrides,
  };
}

describe("buildFallbackConversationStats", () => {
  it("aggregates message-level usage and attribution when backend stats are unavailable", () => {
    const stats = buildFallbackConversationStats(conversation, [
      message({ role: "user", content: "hello" }),
      message({
        id: "assistant-1",
        inputTokens: 100,
        outputTokens: 20,
        cacheReadTokens: 50,
      }),
      message({
        id: "assistant-2",
        inputTokens: 25,
        outputTokens: 5,
        cacheCreationTokens: 10,
      }),
    ]);

    expect(stats).not.toBeNull();
    expect(stats?.effectiveUsageTotals).toEqual({
      inputTokens: 125,
      outputTokens: 25,
      cacheCreationTokens: 10,
      cacheReadTokens: 50,
      estimatedUsd: null,
    });
    expect(stats?.usageCoverage).toEqual({
      providerMessageCount: 2,
      providerMessagesWithUsage: 2,
      runCount: 0,
      runsWithUsage: 0,
      effectiveTotalsSource: "messages",
    });
    expect(stats?.attributionCoverage).toEqual({
      providerMessageCount: 2,
      providerMessagesWithAttribution: 2,
      runCount: 0,
      runsWithAttribution: 0,
    });
    expect(stats?.byHarness[0]?.key).toBe("codex");
    expect(stats?.byModel[0]?.key).toBe("gpt-5.4");
    expect(stats?.byEffort[0]?.key).toBe("xhigh");
    expect(stats?.byUpstreamProvider[0]?.key).toBe("openai");
  });

  it("returns zeroed stats for a selected conversation with no provider usage yet", () => {
    const stats = buildFallbackConversationStats(conversation, [
      message({ id: "user-1", role: "user", content: "hello" }),
    ]);

    expect(stats).not.toBeNull();
    expect(stats?.effectiveUsageTotals).toEqual({
      inputTokens: 0,
      outputTokens: 0,
      cacheCreationTokens: 0,
      cacheReadTokens: 0,
      estimatedUsd: null,
    });
    expect(stats?.usageCoverage.effectiveTotalsSource).toBe("none");
    expect(stats?.usageCoverage.providerMessageCount).toBe(0);
  });
});
