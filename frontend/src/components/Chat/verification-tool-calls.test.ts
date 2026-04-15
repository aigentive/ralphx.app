import { describe, expect, it } from "vitest";

import {
  normalizeStreamingVerificationContentBlocks,
  normalizeToolCallTranscriptPayload,
} from "./verification-tool-calls";
import type { ToolCall } from "./tool-widgets/shared.constants";
import type { StreamingContentBlock } from "@/types/streaming-task";

function makeToolCall(name: string, overrides: Partial<ToolCall> = {}): ToolCall {
  return {
    id: `${name}-id`,
    name,
    arguments: {},
    ...overrides,
  };
}

function mcpWrap(payload: unknown): unknown {
  return [{ type: "text", text: JSON.stringify(payload) }];
}

describe("normalizeToolCallTranscriptPayload", () => {
  it("drops a stale verification round card once a later round report resolves the same round", () => {
    const { toolCalls } = normalizeToolCallTranscriptPayload({
      toolCalls: [
        makeToolCall("mcp__ralphx__run_verification_round", {
          arguments: { round: 2 },
        }),
        makeToolCall("mcp__ralphx__report_verification_round", {
          arguments: { round: 2 },
          result: mcpWrap({
            status: "needs_revision",
            in_progress: true,
            current_round: 2,
            current_gaps: [{ severity: "high", category: "scope", description: "gap" }],
          }),
        }),
      ],
    });

    expect(toolCalls).toHaveLength(1);
    expect(toolCalls[0]?.name).toBe("mcp__ralphx__report_verification_round");
  });

  it("drops requested-only enrichment once later verification steps exist", () => {
    const { toolCalls } = normalizeToolCallTranscriptPayload({
      toolCalls: [
        makeToolCall("mcp__ralphx__run_verification_enrichment", {
          arguments: { selected_specialists: ["intent", "code-quality"] },
          result: mcpWrap({
            selected_specialists: [],
            findings_by_critic: [],
          }),
        }),
        makeToolCall("mcp__ralphx__run_verification_round", {
          arguments: { round: 1 },
          result: mcpWrap({
            round: 1,
            classification: "complete",
          }),
        }),
      ],
    });

    expect(toolCalls).toHaveLength(1);
    expect(toolCalls[0]?.name).toBe("mcp__ralphx__run_verification_round");
  });

  it("keeps meaningful enrichment results even when later verification steps exist", () => {
    const { toolCalls } = normalizeToolCallTranscriptPayload({
      toolCalls: [
        makeToolCall("mcp__ralphx__run_verification_enrichment", {
          arguments: { selected_specialists: ["intent"] },
          result: mcpWrap({
            selected_specialists: [{ label: "intent", critic: "intent" }],
            delegate_snapshots: [{ job_id: "job-1", status: "completed", label: "intent" }],
            findings_by_critic: [{ critic: "intent", found: true, total_matches: 1 }],
          }),
        }),
        makeToolCall("mcp__ralphx__report_verification_round", {
          arguments: { round: 1 },
          result: mcpWrap({
            status: "reviewing",
            in_progress: true,
            current_round: 1,
          }),
        }),
      ],
    });

    expect(toolCalls).toHaveLength(2);
    expect(toolCalls[0]?.name).toBe("mcp__ralphx__run_verification_enrichment");
    expect(toolCalls[1]?.name).toBe("mcp__ralphx__report_verification_round");
  });
});

describe("normalizeStreamingVerificationContentBlocks", () => {
  it("drops stale running verification widgets once later authoritative verification steps arrive", () => {
    const blocks: StreamingContentBlock[] = [
      {
        type: "tool_use",
        toolCall: makeToolCall("mcp__ralphx__run_verification_enrichment", {
          arguments: { selected_specialists: ["intent", "code-quality"] },
          result: mcpWrap({
            selected_specialists: [],
            findings_by_critic: [],
          }),
        }),
      },
      {
        type: "text",
        text: "The enrichment pass didn't surface actionable findings, so I'm starting round 1.",
      },
      {
        type: "tool_use",
        toolCall: makeToolCall("mcp__ralphx__run_verification_round", {
          arguments: { round: 1 },
        }),
      },
      {
        type: "tool_use",
        toolCall: makeToolCall("mcp__ralphx__report_verification_round", {
          arguments: { round: 1 },
          result: mcpWrap({
            status: "needs_revision",
            in_progress: false,
            current_round: 1,
            current_gaps: [{ severity: "high", category: "state-machine", description: "gap" }],
          }),
        }),
      },
    ];

    const normalized = normalizeStreamingVerificationContentBlocks(blocks);

    expect(normalized).toHaveLength(2);
    expect(normalized[0]).toMatchObject({ type: "text" });
    expect(normalized[1]).toMatchObject({
      type: "tool_use",
      toolCall: { name: "mcp__ralphx__report_verification_round" },
    });
  });
});
