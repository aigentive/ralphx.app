import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import type { ChatMessageResponse } from "@/api/chat";
import type { StreamingTask } from "@/types/streaming-task";
import type { ToolCall } from "./ToolCallIndicator";
import {
  buildTaskCardTranscriptEntriesFromConversation,
  buildTaskCardTranscriptEntryFromStreamingTask,
  buildTaskCardTranscriptEntryFromToolCall,
  TaskCardTranscriptView,
} from "./TaskCardTranscript";

function makeToolCall(overrides?: Partial<ToolCall>): ToolCall {
  return {
    id: "toolu-1",
    name: "bash",
    arguments: { command: "pwd" },
    result: "ok",
    ...overrides,
  };
}

function makeStreamingTask(overrides?: Partial<StreamingTask>): StreamingTask {
  return {
    toolUseId: "toolu-task-1",
    toolName: "Task",
    description: "Inspect repository layout",
    subagentType: "Explore",
    model: "sonnet",
    status: "completed",
    startedAt: Date.now() - 10_000,
    completedAt: Date.now(),
    totalDurationMs: 10_000,
    totalTokens: 1234,
    totalToolUseCount: 2,
    childToolCalls: [],
    ...overrides,
  };
}

describe("TaskCardTranscript", () => {
  it("builds a persisted task transcript entry from child tool calls and text output", () => {
    const entry = buildTaskCardTranscriptEntryFromToolCall({
      entryId: "task-toolu-1",
      bodyText: "Smoke checks completed cleanly.",
      childToolCalls: [
        makeToolCall({ id: "child-1", name: "bash", arguments: { command: "npm test" } }),
      ],
    });

    expect(entry.id).toBe("task-toolu-1");
    expect(entry.blocks).toHaveLength(2);
    expect(entry.blocks[0]?.type).toBe("tool_call");
    expect(entry.blocks[1]).toEqual({
      type: "text",
      text: "Smoke checks completed cleanly.",
    });
  });

  it("builds a streaming transcript entry with a live activity block", () => {
    const entry = buildTaskCardTranscriptEntryFromStreamingTask(
      makeStreamingTask({
        status: "running",
        textOutput: "Review still in progress.",
        childToolCalls: [
          makeToolCall({ id: "child-2", name: "grep", arguments: { pattern: "TODO" } }),
        ],
      }),
    );

    expect(entry.blocks.map((block) => block.type)).toEqual([
      "tool_call",
      "activity",
      "text",
    ]);
    expect(entry.blocks[1]).toEqual({
      type: "activity",
      label: "Working…",
    });
  });

  it("builds delegated conversation transcript entries from persisted chat messages", () => {
    const messages: ChatMessageResponse[] = [
      {
        id: "msg-user-1",
        sessionId: "session-1",
        projectId: "project-1",
        taskId: null,
        role: "user",
        content: "Review the patch.",
        metadata: null,
        parentMessageId: null,
        conversationId: "conv-1",
        toolCalls: null,
        contentBlocks: null,
        sender: null,
        createdAt: "2026-04-12T10:00:00Z",
      },
      {
        id: "msg-assistant-1",
        sessionId: "session-1",
        projectId: "project-1",
        taskId: null,
        role: "assistant",
        content: "Found one blocker.",
        metadata: null,
        parentMessageId: "msg-user-1",
        conversationId: "conv-1",
        toolCalls: null,
        contentBlocks: [
          {
            type: "text",
            text: "I inspected the handler first.",
          },
          {
            type: "tool_use",
            id: "toolu-child-1",
            name: "bash",
            arguments: { command: "rg parent_tool_use_id" },
            result: "match",
          },
        ],
        sender: "delegated reviewer",
        createdAt: "2026-04-12T10:00:05Z",
      },
    ];

    const entries = buildTaskCardTranscriptEntriesFromConversation(messages);

    expect(entries).toHaveLength(2);
    expect(entries[0]?.speakerLabel).toBe("User");
    expect(entries[1]?.speakerLabel).toBe("delegated reviewer");
    expect(entries[1]?.blocks.map((block) => block.type)).toEqual(["text", "tool_call"]);
  });

  it("renders transcript entries through one shared body view", () => {
    const entries = [
      buildTaskCardTranscriptEntryFromStreamingTask(
        makeStreamingTask({
          status: "running",
          childToolCalls: [
            makeToolCall({ id: "child-3", name: "bash", arguments: { command: "cargo test" } }),
          ],
          textOutput: "Streaming verification output",
        }),
      ),
    ];

    render(
      <TaskCardTranscriptView
        entries={entries}
        dataTestId="shared-task-card-transcript"
      />,
    );

    expect(screen.getByTestId("shared-task-card-transcript")).toBeInTheDocument();
    expect(screen.getAllByText("cargo test").length).toBeGreaterThan(0);
    expect(screen.getByText("Working…")).toBeInTheDocument();
    expect(screen.getByText("Streaming verification output")).toBeInTheDocument();
  });
});
