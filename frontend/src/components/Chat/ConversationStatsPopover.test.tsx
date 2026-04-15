import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { ConversationStatsPopover } from "./ConversationStatsPopover";
import type { ConversationStatsResponse } from "@/api/chat";

const mockUseConversationStats = vi.fn();

vi.mock("@/hooks/useConversationStats", () => ({
  useConversationStats: (...args: unknown[]) => mockUseConversationStats(...args),
}));

function makeStats(
  overrides: Partial<ConversationStatsResponse> = {},
): ConversationStatsResponse {
  return {
    conversationId: "conv-1",
    contextType: "ideation",
    contextId: "session-1",
    providerHarness: "codex",
    upstreamProvider: "openai",
    providerProfile: null,
    messageUsageTotals: {
      inputTokens: 76286,
      outputTokens: 12148,
      cacheCreationTokens: 12000,
      cacheReadTokens: 37920,
      estimatedUsd: null,
    },
    runUsageTotals: {
      inputTokens: 0,
      outputTokens: 0,
      cacheCreationTokens: 0,
      cacheReadTokens: 0,
      estimatedUsd: null,
    },
    effectiveUsageTotals: {
      inputTokens: 76286,
      outputTokens: 12148,
      cacheCreationTokens: 12000,
      cacheReadTokens: 37920,
      estimatedUsd: null,
    },
    usageCoverage: {
      providerMessageCount: 1,
      providerMessagesWithUsage: 1,
      runCount: 0,
      runsWithUsage: 0,
      effectiveTotalsSource: "messages",
    },
    attributionCoverage: {
      providerMessageCount: 1,
      providerMessagesWithAttribution: 1,
      runCount: 0,
      runsWithAttribution: 0,
    },
    byHarness: [],
    byUpstreamProvider: [{ key: "openai", count: 1, usage: {
      inputTokens: 76286,
      outputTokens: 12148,
      cacheCreationTokens: 12000,
      cacheReadTokens: 37920,
      estimatedUsd: null,
    } }],
    byModel: [{ key: "gpt-5.4", count: 1, usage: {
      inputTokens: 76286,
      outputTokens: 12148,
      cacheCreationTokens: 12000,
      cacheReadTokens: 37920,
      estimatedUsd: null,
    } }],
    byEffort: [{ key: "xhigh", count: 1, usage: {
      inputTokens: 76286,
      outputTokens: 12148,
      cacheCreationTokens: 12000,
      cacheReadTokens: 37920,
      estimatedUsd: null,
    } }],
    ...overrides,
  };
}

describe("ConversationStatsPopover", () => {
  beforeEach(() => {
    mockUseConversationStats.mockReset();
  });

  it("renders compact token totals for large conversations", async () => {
    mockUseConversationStats.mockReturnValue({
      data: makeStats(),
      isLoading: false,
    });

    render(
      <ConversationStatsPopover
        conversationId="conv-1"
        fallbackConversation={null}
        fallbackMessages={null}
      />,
    );

    fireEvent.click(screen.getByTestId("chat-session-stats-button"));

    expect(await screen.findByText("Conversation stats")).toBeInTheDocument();
    expect(screen.getByText("76.3k")).toBeInTheDocument();
    expect(screen.getByText("12.1k")).toBeInTheDocument();
    expect(screen.getByText("49.9k")).toBeInTheDocument();
  });

  it("hides run coverage rows when no run aggregates exist", async () => {
    mockUseConversationStats.mockReturnValue({
      data: makeStats(),
      isLoading: false,
    });

    render(
      <ConversationStatsPopover
        conversationId="conv-1"
        fallbackConversation={null}
        fallbackMessages={null}
      />,
    );

    fireEvent.click(screen.getByTestId("chat-session-stats-button"));

    expect(await screen.findByText("Coverage")).toBeInTheDocument();
    expect(screen.getByText("Usage captured on all provider turns")).toBeInTheDocument();
    expect(screen.getByText("Attribution captured on all provider turns")).toBeInTheDocument();
    expect(screen.queryByText(/Runs:/)).not.toBeInTheDocument();
  });

  it("shows pending usage copy during an active turn when provider totals have not arrived yet", async () => {
    mockUseConversationStats.mockReturnValue({
      data: makeStats({
        effectiveUsageTotals: {
          inputTokens: 0,
          outputTokens: 0,
          cacheCreationTokens: 0,
          cacheReadTokens: 0,
          estimatedUsd: null,
        },
        usageCoverage: {
          providerMessageCount: 1,
          providerMessagesWithUsage: 0,
          runCount: 0,
          runsWithUsage: 0,
          effectiveTotalsSource: "none",
        },
      }),
      isLoading: false,
    });

    render(
      <ConversationStatsPopover
        conversationId="conv-1"
        fallbackConversation={null}
        fallbackMessages={null}
        isLiveTurnActive={true}
      />,
    );

    fireEvent.click(screen.getByTestId("chat-session-stats-button"));

    expect(await screen.findByText("Usage totals are pending until the provider reports the current turn.")).toBeInTheDocument();
    expect(screen.getAllByText("Pending")).toHaveLength(4);
  });
});
