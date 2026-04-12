import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { TaskSubagentCard } from "./TaskSubagentCard";
import type { StreamingTask } from "@/types/streaming-task";

function makeStreamingTask(overrides?: Partial<StreamingTask>): StreamingTask {
  return {
    toolUseId: "toolu-task-1",
    toolName: "Task",
    description: "Inspect repository layout",
    subagentType: "Explore",
    model: "sonnet",
    status: "completed",
    startedAt: Date.now() - 6_200,
    completedAt: Date.now(),
    totalDurationMs: 6_200,
    totalTokens: 1_532,
    totalToolUseCount: 3,
    estimatedUsd: 0.43,
    childToolCalls: [],
    ...overrides,
  };
}

describe("TaskSubagentCard", () => {
  it("renders delegated streaming cards with shared provider chrome", () => {
    render(
      <TaskSubagentCard
        task={makeStreamingTask({
          toolName: "delegate_start",
          description: "Review delegated patch",
          subagentType: "delegated",
          model: "gpt-5.4",
          providerHarness: "codex",
          providerSessionId: "thread-1234567890",
          upstreamProvider: "openai",
          providerProfile: "openai",
          logicalModel: "gpt-5.4",
          effectiveModelId: "gpt-5.4",
        })}
      />,
    );

    expect(screen.getByText("Delegate")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toHaveAttribute(
      "title",
      expect.stringContaining("Upstream: openai"),
    );
    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.queryByText("delegated")).not.toBeInTheDocument();
  });

  it("shows collapsed completed summary metrics", () => {
    render(<TaskSubagentCard task={makeStreamingTask()} />);

    expect(
      screen.getByText("6s · 1,532 tokens · 3 tools · $0.43"),
    ).toBeInTheDocument();
  });

  it("shows failure status while preserving subagent type chrome", () => {
    render(
      <TaskSubagentCard
        task={makeStreamingTask({
          status: "failed",
          totalDurationMs: undefined,
          totalTokens: undefined,
          totalToolUseCount: undefined,
          estimatedUsd: undefined,
        })}
      />,
    );

    expect(screen.getByText("Explore")).toBeInTheDocument();
    expect(screen.getByText("failed")).toBeInTheDocument();
  });
});
