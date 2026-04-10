import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { AttributionBackfillSummary } from "@/api/metrics";
import { AttributionBackfillCard } from "./AttributionBackfillCard";

const runningSummary: AttributionBackfillSummary = {
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

describe("AttributionBackfillCard", () => {
  it("renders running import progress and breakdown counts", () => {
    render(<AttributionBackfillCard summary={runningSummary} />);

    expect(screen.getByText("Historical Transcript Import")).toBeInTheDocument();
    expect(screen.getByText("Historical Claude transcript import is running in the background.")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    expect(screen.getByText("6")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("Pending: 3 · Running: 1")).toBeInTheDocument();
    expect(screen.getByText("Not found: 1 · Parse failed: 0")).toBeInTheDocument();
  });

  it("renders the completed state when nothing remains", () => {
    render(
      <AttributionBackfillCard
        summary={{
          ...runningSummary,
          pendingCount: 0,
          runningCount: 0,
          completedCount: 10,
          partialCount: 0,
          sessionNotFoundCount: 0,
          remainingCount: 0,
          attentionCount: 0,
          terminalCount: 10,
          isIdle: true,
        }}
      />,
    );

    expect(screen.getByText("Historical Claude transcript import is complete.")).toBeInTheDocument();
    expect(screen.getByText("Idle: yes · Terminal states: 10")).toBeInTheDocument();
  });
});
