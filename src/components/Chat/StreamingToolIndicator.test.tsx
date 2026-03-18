/**
 * StreamingToolIndicator tests
 *
 * Covers:
 * - Basic rendering (no tool calls, with tool calls)
 * - Elapsed timer renders for active bash tool call with toolCallStartTimes
 * - No elapsed timer for completed tool calls (with result)
 * - No elapsed timer when isActive=false
 * - Timer updates every second via setInterval
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import type { ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Helpers
// ============================================================================

function makeBashCall(id: string, withResult = false): ToolCall {
  const tc: ToolCall = {
    id,
    name: "Bash",
    arguments: { command: "cargo test --lib", description: "Run tests" },
  };
  if (withResult) {
    tc.result = "test output";
  }
  return tc;
}

function makeReadCall(id: string): ToolCall {
  return {
    id,
    name: "Read",
    arguments: { file_path: "src/main.rs" },
  };
}

/** Click the expand button to open the tool call list */
function expandIndicator() {
  fireEvent.click(screen.getByRole("button"));
}

// ============================================================================
// Basic rendering
// ============================================================================

describe("StreamingToolIndicator — basic", () => {
  it("returns null when toolCalls is empty", () => {
    const { container } = render(
      <StreamingToolIndicator toolCalls={[]} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders indicator with tool calls present", () => {
    render(
      <StreamingToolIndicator toolCalls={[makeBashCall("tc-1")]} />
    );
    expect(screen.getByTestId("streaming-tool-indicator")).toBeInTheDocument();
  });

  it("shows count of tool calls in header", () => {
    const calls = [makeBashCall("tc-1"), makeReadCall("tc-2")];
    render(<StreamingToolIndicator toolCalls={calls} />);
    expect(screen.getByText(/2 tool calls/)).toBeInTheDocument();
  });

  it("expands on click to show tool call list", () => {
    render(
      <StreamingToolIndicator toolCalls={[makeBashCall("tc-1")]} />
    );
    expandIndicator();
    expect(screen.getByText("Running")).toBeInTheDocument();
  });
});

// ============================================================================
// Elapsed timer — active tool calls
// ============================================================================

describe("StreamingToolIndicator — elapsed timer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows elapsed timer for active bash tool call when expanded", () => {
    const now = Date.now();
    const toolCallId = "tc-bash-1";
    const startedAt = now - 63_000; // 1m 3s ago

    render(
      <StreamingToolIndicator
        toolCalls={[makeBashCall(toolCallId)]}
        isActive={true}
        toolCallStartTimes={{ [toolCallId]: startedAt }}
      />
    );

    expandIndicator();

    expect(screen.getByTestId("elapsed-timer")).toBeInTheDocument();
    expect(screen.getByText(/Running 1m 3s/)).toBeInTheDocument();
  });

  it("elapsed timer updates after 1 second", () => {
    const now = Date.now();
    const toolCallId = "tc-bash-2";
    const startedAt = now - 10_000; // 10s ago

    render(
      <StreamingToolIndicator
        toolCalls={[makeBashCall(toolCallId)]}
        isActive={true}
        toolCallStartTimes={{ [toolCallId]: startedAt }}
      />
    );

    expandIndicator();
    expect(screen.getByText(/Running 10s/)).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(screen.getByText(/Running 11s/)).toBeInTheDocument();
  });

  it("does not show elapsed timer for tool call with result (completed)", () => {
    const now = Date.now();
    const toolCallId = "tc-bash-completed";

    render(
      <StreamingToolIndicator
        toolCalls={[makeBashCall(toolCallId, true)]}
        isActive={true}
        toolCallStartTimes={{ [toolCallId]: now - 30_000 }}
      />
    );

    expandIndicator();

    expect(screen.queryByTestId("elapsed-timer")).not.toBeInTheDocument();
  });

  it("does not show elapsed timer when isActive is false", () => {
    const now = Date.now();
    const toolCallId = "tc-bash-3";

    render(
      <StreamingToolIndicator
        toolCalls={[makeBashCall(toolCallId)]}
        isActive={false}
        toolCallStartTimes={{ [toolCallId]: now - 30_000 }}
      />
    );

    expandIndicator();

    expect(screen.queryByTestId("elapsed-timer")).not.toBeInTheDocument();
  });

  it("does not show elapsed timer when toolCallStartTimes is not provided", () => {
    render(
      <StreamingToolIndicator
        toolCalls={[makeBashCall("tc-bash-4")]}
        isActive={true}
      />
    );

    expandIndicator();

    expect(screen.queryByTestId("elapsed-timer")).not.toBeInTheDocument();
  });

  it("shows elapsed timer only for last active call when multiple calls present", () => {
    const now = Date.now();
    const completedId = "tc-bash-completed";
    const activeId = "tc-bash-active";

    const calls = [
      makeBashCall(completedId, true), // completed
      makeBashCall(activeId),           // active
    ];

    render(
      <StreamingToolIndicator
        toolCalls={calls}
        isActive={true}
        toolCallStartTimes={{
          [completedId]: now - 60_000,
          [activeId]: now - 5_000,
        }}
      />
    );

    expandIndicator();

    // Exactly one elapsed timer shown (for the active call only)
    const timers = screen.getAllByTestId("elapsed-timer");
    expect(timers).toHaveLength(1);
    expect(screen.getByText(/Running 5s/)).toBeInTheDocument();
  });
});
