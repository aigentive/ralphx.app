import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { PaneStream } from "./PaneStream";
import type { TeammateState } from "@/stores/teamStore";

// Mock teamStore to avoid infinite re-render from factory selectors
const mockMate = vi.fn();

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    return selector({});
  },
  selectTeammateByName: () => () => mockMate(),
}));

function makeMate(streamingText = ""): TeammateState {
  return {
    name: "worker-1",
    color: "#4ade80",
    model: "sonnet",
    roleDescription: "coder",
    status: "running",
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText,
  };
}

describe("PaneStream", () => {
  beforeEach(() => {
    mockMate.mockReturnValue(null);
  });

  it("shows waiting message when no streaming text", () => {
    mockMate.mockReturnValue(makeMate(""));
    render(<PaneStream contextKey="test" teammateName="worker-1" />);
    expect(screen.getByText("Waiting for output...")).toBeInTheDocument();
  });

  it("renders plain streaming text", () => {
    mockMate.mockReturnValue(makeMate("Hello world, analyzing code..."));
    render(<PaneStream contextKey="test" teammateName="worker-1" />);
    expect(screen.getByText(/Hello world, analyzing code/)).toBeInTheDocument();
  });

  it("renders tool call badges for recognized tools", () => {
    mockMate.mockReturnValue(makeMate("Looking at [Read src/main.ts] now"));
    render(<PaneStream contextKey="test" teammateName="worker-1" />);
    expect(screen.getByText("[Read src/main.ts]")).toBeInTheDocument();
  });

  it("renders badges for all 8 recognized tool types", () => {
    const tools = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebFetch", "WebSearch"];
    const text = tools.map((t) => `[${t} file.ts]`).join(" ");
    mockMate.mockReturnValue(makeMate(text));
    render(<PaneStream contextKey="test" teammateName="worker-1" />);
    for (const tool of tools) {
      expect(screen.getByText(`[${tool} file.ts]`)).toBeInTheDocument();
    }
  });

  it("shows waiting text when teammate is not found", () => {
    render(<PaneStream contextKey="test" teammateName="nonexistent" />);
    expect(screen.getByText("Waiting for output...")).toBeInTheDocument();
  });
});
