import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MockEventBus } from "@/lib/event-bus";
import {
  CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT,
  useChatAttributionBackfillSummary,
} from "./useChatAttributionBackfillSummary";
import { getChatAttributionBackfillSummary } from "@/api/metrics";

let testEventBus: MockEventBus;

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => testEventBus,
}));

vi.mock("@/api/metrics", () => ({
  getChatAttributionBackfillSummary: vi.fn(),
}));

const mockGetChatAttributionBackfillSummary =
  getChatAttributionBackfillSummary as ReturnType<typeof vi.fn>;

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
        staleTime: 0,
      },
    },
  });

  return {
    queryClient,
    wrapper: ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
  };
}

describe("useChatAttributionBackfillSummary", () => {
  beforeEach(() => {
    testEventBus = new MockEventBus();
    vi.clearAllMocks();
  });

  it("updates cached summary when the backend emits progress", async () => {
    mockGetChatAttributionBackfillSummary.mockResolvedValue({
      eligibleConversationCount: 100,
      pendingCount: 99,
      runningCount: 1,
      completedCount: 0,
      partialCount: 0,
      sessionNotFoundCount: 0,
      parseFailedCount: 0,
      remainingCount: 100,
      terminalCount: 0,
      attentionCount: 0,
      isIdle: false,
    });

    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useChatAttributionBackfillSummary(), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.data?.completedCount).toBe(0);
    });

    act(() => {
      testEventBus.emit(CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT, {
        processedInBatch: 50,
        eligibleConversationCount: 100,
        pendingCount: 49,
        runningCount: 1,
        completedCount: 50,
        partialCount: 0,
        sessionNotFoundCount: 0,
        parseFailedCount: 0,
        remainingCount: 50,
        terminalCount: 50,
        attentionCount: 0,
        isIdle: false,
      });
    });

    await waitFor(() => {
      expect(result.current.data?.completedCount).toBe(50);
    });
    expect(result.current.data?.remainingCount).toBe(50);
  });
});
