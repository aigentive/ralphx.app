import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TranscriptImportSection } from "./TranscriptImportSection";
import { useChatAttributionBackfillSummary } from "@/hooks/useChatAttributionBackfillSummary";

vi.mock("@/hooks/useChatAttributionBackfillSummary", () => ({
  useChatAttributionBackfillSummary: vi.fn(),
}));

const mockUseChatAttributionBackfillSummary = useChatAttributionBackfillSummary as ReturnType<typeof vi.fn>;

function renderSection() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TranscriptImportSection />
    </QueryClientProvider>,
  );
}

describe("TranscriptImportSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders import counts and status", () => {
    mockUseChatAttributionBackfillSummary.mockReturnValue({
      data: {
        eligibleConversationCount: 12,
        pendingCount: 2,
        runningCount: 1,
        completedCount: 7,
        partialCount: 1,
        sessionNotFoundCount: 1,
        parseFailedCount: 0,
        remainingCount: 3,
        terminalCount: 9,
        attentionCount: 2,
        isIdle: false,
      },
      isLoading: false,
      error: null,
    });

    renderSection();

    expect(screen.getByText("Transcript Import")).toBeInTheDocument();
    expect(screen.getByText("Historical Claude transcript import is running in the background.")).toBeInTheDocument();
    expect(screen.getByText("Pending: 2 · Running: 1 · Partial: 1")).toBeInTheDocument();
    expect(screen.getByText("Not found: 1 · Parse failed: 0")).toBeInTheDocument();
  });

  it("renders loading state", () => {
    mockUseChatAttributionBackfillSummary.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    });

    renderSection();

    expect(screen.getByText("Loading transcript import status...")).toBeInTheDocument();
  });
});
