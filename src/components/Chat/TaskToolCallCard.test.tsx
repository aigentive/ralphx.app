/**
 * TaskToolCallCard tests
 *
 * Tests the static card for completed Task and Agent tool calls.
 * Agent tool calls share the same argument shape (description, subagent_type, model)
 * so the same component renders both.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TaskToolCallCard } from "./TaskToolCallCard";
import type { ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Test Data
// ============================================================================

function makeAgentToolCall(overrides?: Partial<ToolCall>): ToolCall {
  return {
    id: "agent-call-1",
    name: "Agent",
    arguments: {
      description: "Explore codebase structure",
      subagent_type: "Explore",
      model: "sonnet",
      prompt: "Find all TypeScript files",
    },
    ...overrides,
  };
}

function makeTaskToolCall(overrides?: Partial<ToolCall>): ToolCall {
  return {
    id: "task-call-1",
    name: "Task",
    arguments: {
      description: "Run tests",
      subagent_type: "general-purpose",
      model: "opus",
      prompt: "Execute the test suite",
    },
    ...overrides,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("TaskToolCallCard — Agent tool call arguments", () => {
  it("renders the card wrapper with data-testid", () => {
    render(<TaskToolCallCard toolCall={makeAgentToolCall()} />);
    expect(screen.getByTestId("task-tool-call-card")).toBeInTheDocument();
  });

  it("shows description from Agent args", () => {
    render(<TaskToolCallCard toolCall={makeAgentToolCall()} />);
    expect(screen.getByText("Explore codebase structure")).toBeInTheDocument();
  });

  it("shows subagent_type badge from Agent args", () => {
    render(<TaskToolCallCard toolCall={makeAgentToolCall()} />);
    expect(screen.getByText("Explore")).toBeInTheDocument();
  });

  it("shows model badge from Agent args", () => {
    render(<TaskToolCallCard toolCall={makeAgentToolCall()} />);
    expect(screen.getByText("sonnet")).toBeInTheDocument();
  });

  it("shows 'Plan' subagent type badge for Plan agent", () => {
    const tc = makeAgentToolCall({
      arguments: {
        description: "Plan the implementation",
        subagent_type: "Plan",
        model: "opus",
        prompt: "Create a plan",
      },
    });
    render(<TaskToolCallCard toolCall={tc} />);
    expect(screen.getByText("Plan")).toBeInTheDocument();
    expect(screen.getByText("opus")).toBeInTheDocument();
  });

  it("shows 'general-purpose' badge for general-purpose agent type", () => {
    const tc = makeAgentToolCall({
      arguments: {
        description: "Research the codebase",
        subagent_type: "general-purpose",
        model: "haiku",
        prompt: "Find patterns",
      },
    });
    render(<TaskToolCallCard toolCall={tc} />);
    expect(screen.getByText("general-purpose")).toBeInTheDocument();
  });

  it("falls back to 'agent' subagent type label when subagent_type is missing", () => {
    const tc = makeAgentToolCall({
      arguments: {
        description: "Do some work",
        model: "sonnet",
        prompt: "...",
      },
    });
    render(<TaskToolCallCard toolCall={tc} />);
    expect(screen.getByText("agent")).toBeInTheDocument();
  });

  it("falls back to 'Subagent task' description when description is missing", () => {
    const tc = makeAgentToolCall({
      arguments: {
        subagent_type: "Explore",
        model: "sonnet",
        prompt: "...",
      },
    });
    render(<TaskToolCallCard toolCall={tc} />);
    expect(screen.getByText("Subagent task")).toBeInTheDocument();
  });

  it("does not show model badge when model is missing", () => {
    const tc = makeAgentToolCall({
      arguments: {
        description: "Do something",
        subagent_type: "Explore",
        prompt: "...",
      },
    });
    render(<TaskToolCallCard toolCall={tc} />);
    // No sonnet/opus/haiku badge
    expect(screen.queryByText("sonnet")).not.toBeInTheDocument();
    expect(screen.queryByText("opus")).not.toBeInTheDocument();
    expect(screen.queryByText("haiku")).not.toBeInTheDocument();
  });

  it("handles null/invalid arguments gracefully", () => {
    const tc = makeAgentToolCall({ arguments: null });
    render(<TaskToolCallCard toolCall={tc} />);
    // Falls back to defaults — card should still render
    expect(screen.getByTestId("task-tool-call-card")).toBeInTheDocument();
    expect(screen.getByText("Subagent task")).toBeInTheDocument();
  });
});

describe("TaskToolCallCard — Task tool call (baseline, unchanged behavior)", () => {
  it("renders correctly with Task tool call arguments", () => {
    render(<TaskToolCallCard toolCall={makeTaskToolCall()} />);
    expect(screen.getByTestId("task-tool-call-card")).toBeInTheDocument();
    expect(screen.getByText("Run tests")).toBeInTheDocument();
    expect(screen.getByText("general-purpose")).toBeInTheDocument();
    expect(screen.getByText("opus")).toBeInTheDocument();
  });
});

describe("TaskToolCallCard — stats rendering", () => {
  it("shows duration, tokens, and tool count from result usage block", () => {
    const result = `
Agent output here.
agentId: abc123def
<usage>total_tokens: 5432
tool_uses: 12
duration_ms: 47000</usage>
    `.trim();
    const tc = makeAgentToolCall({ result });
    render(<TaskToolCallCard toolCall={tc} />);

    // Stats row should show formatted values
    expect(screen.getByText(/47s/)).toBeInTheDocument();
    expect(screen.getByText(/5,432 tokens/)).toBeInTheDocument();
    expect(screen.getByText(/12 tools/)).toBeInTheDocument();
  });

  it("shows no stats row when result has no usage block", () => {
    const tc = makeAgentToolCall({ result: undefined });
    const { container } = render(<TaskToolCallCard toolCall={tc} />);
    // No stats div with a middle dot separator
    expect(container.textContent).not.toContain("\u00B7");
  });
});

describe("TaskToolCallCard — expanded body", () => {
  it("shows expanded text output when clicked", async () => {
    const user = userEvent.setup();
    const result = `Agent found 42 TypeScript files.
agentId: abc123
<usage>total_tokens: 100
tool_uses: 3
duration_ms: 2000</usage>`;

    const tc = makeAgentToolCall({ result });
    render(<TaskToolCallCard toolCall={tc} />);

    // Click header to expand
    const header = screen.getByRole("button");
    await user.click(header);

    expect(screen.getByText("Agent found 42 TypeScript files.")).toBeInTheDocument();
  });

  it("shows child tool calls in expanded view when result has content blocks", async () => {
    const user = userEvent.setup();
    // Use a generic tool name (no widget registered) so generic ToolCallIndicator renders
    // and shows the tool name as text directly in the DOM.
    const result = [
      { type: "tool_use", id: "child-1", name: "inspect_code", input: { target: "src/" } },
      { type: "tool_result", tool_use_id: "child-1", content: "inspected" },
      { type: "text", text: "Found files." },
    ];

    const tc = makeAgentToolCall({ result });
    const { container } = render(<TaskToolCallCard toolCall={tc} />);

    // Click header to expand
    const header = screen.getByRole("button");
    await user.click(header);

    // Child tool call should render inside the expanded body (as ToolCallIndicator)
    expect(container.querySelector('[data-testid="tool-call-indicator"]')).toBeInTheDocument();
    // The generic ToolCallIndicator shows the tool name
    expect(screen.getByText("inspect_code")).toBeInTheDocument();
  });
});

describe("TaskToolCallCard — error state", () => {
  it("shows Failed badge when error is present", () => {
    const tc = makeAgentToolCall({ error: "Agent timed out" });
    render(<TaskToolCallCard toolCall={tc} />);
    expect(screen.getByText("Failed")).toBeInTheDocument();
  });

  it("shows error text in expanded view", async () => {
    const user = userEvent.setup();
    const tc = makeAgentToolCall({ error: "Connection refused to agent endpoint" });
    render(<TaskToolCallCard toolCall={tc} />);

    const header = screen.getByRole("button");
    await user.click(header);

    expect(screen.getByText("Connection refused to agent endpoint")).toBeInTheDocument();
  });
});
