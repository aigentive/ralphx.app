import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CoordinatorPane } from "./CoordinatorPane";
import type { TeamMessage } from "@/stores/teamStore";

// Mock teamStore to avoid infinite re-render from factory selectors
// CoordinatorPane renders TeamOverviewHeader as child, which uses selectTeammates
const mockTeam = vi.fn();
const mockMessages = vi.fn();
const mockTeammates = vi.fn();

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (s: unknown) => unknown) => {
    return selector({});
  },
  selectActiveTeam: () => () => mockTeam(),
  selectTeamMessages: () => () => mockMessages(),
  selectTeammates: () => () => mockTeammates(),
}));

function makeMsg(from: string, to: string, content: string, idx = 0): TeamMessage {
  return {
    id: `msg-${idx}`,
    from,
    to,
    content,
    timestamp: new Date(2026, 0, 15, 10, idx).toISOString(),
  };
}

describe("CoordinatorPane", () => {
  beforeEach(() => {
    mockTeam.mockReturnValue(null);
    mockMessages.mockReturnValue([]);
    mockTeammates.mockReturnValue([]);
  });

  it("renders nothing when team does not exist", () => {
    const { container } = render(<CoordinatorPane contextKey="nonexistent" />);
    expect(container.firstChild).toBeNull();
  });

  it("shows 'No messages yet' when message list is empty", () => {
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    render(<CoordinatorPane contextKey="test" />);
    expect(screen.getByText("No messages yet")).toBeInTheDocument();
  });

  it("filters messages to only those involving the lead", () => {
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    mockMessages.mockReturnValue([
      makeMsg("team-lead", "worker-1", "Start working", 0),
      makeMsg("worker-1", "team-lead", "On it", 1),
      makeMsg("worker-1", "worker-2", "Need help", 2),
    ]);
    render(<CoordinatorPane contextKey="test" />);
    expect(screen.getByText("Start working")).toBeInTheDocument();
    expect(screen.getByText("On it")).toBeInTheDocument();
    expect(screen.queryByText("Need help")).not.toBeInTheDocument();
  });

  it("renders message sender name and content", () => {
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    mockMessages.mockReturnValue([
      makeMsg("team-lead", "worker-1", "Hello there", 0),
    ]);
    render(<CoordinatorPane contextKey="test" />);
    expect(screen.getByText("team-lead")).toBeInTheDocument();
    expect(screen.getByText("Hello there")).toBeInTheDocument();
  });

  it("shows placeholder input with lead name", () => {
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    render(<CoordinatorPane contextKey="test" />);
    expect(screen.getByPlaceholderText("Message team-lead...")).toBeInTheDocument();
  });

  it("calls onSendMessage when send button is clicked", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    render(<CoordinatorPane contextKey="test" onSendMessage={onSend} />);

    const input = screen.getByPlaceholderText("Message team-lead...");
    await user.type(input, "Test message");
    fireEvent.click(screen.getByText("Send"));

    expect(onSend).toHaveBeenCalledWith("Test message");
  });

  it("clears input after sending", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    render(<CoordinatorPane contextKey="test" onSendMessage={onSend} />);

    const input = screen.getByPlaceholderText("Message team-lead...") as HTMLInputElement;
    await user.type(input, "Test message");
    fireEvent.click(screen.getByText("Send"));

    expect(input.value).toBe("");
  });

  it("sends on Enter key press", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();
    mockTeam.mockReturnValue({ teamName: "T", leadName: "team-lead", totalEstimatedCostUsd: 0 });
    render(<CoordinatorPane contextKey="test" onSendMessage={onSend} />);

    const input = screen.getByPlaceholderText("Message team-lead...");
    await user.type(input, "Enter message{Enter}");

    expect(onSend).toHaveBeenCalledWith("Enter message");
  });
});
