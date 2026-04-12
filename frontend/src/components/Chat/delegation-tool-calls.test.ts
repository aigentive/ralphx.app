import { describe, expect, it } from "vitest";
import {
  extractDelegationMetadata,
  mergeDelegationContentBlocks,
  mergeDelegationToolCalls,
  normalizeDelegationTranscriptPayload,
} from "./delegation-tool-calls";
import { makeContentToolUse, makeToolCall } from "./__tests__/chatRenderFixtures";

function makeDelegationResult(payload: Record<string, unknown>) {
  return [{ type: "text", text: JSON.stringify(payload) }];
}

describe("delegation-tool-calls", () => {
  it("folds delegate_wait into the original delegate_start tool call", () => {
    const startToolCall = makeToolCall("delegate_start", {
      id: "toolu-delegate-start",
      arguments: {
        agent_name: "ralphx-execution-reviewer",
        prompt: "Review the patch",
        harness: "codex",
        model: "gpt-5.4",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "running",
      }),
    });
    const waitToolCall = makeToolCall("delegate_wait", {
      id: "toolu-delegate-wait",
      arguments: {
        job_id: "job-123",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "completed",
        content: "Delegated review finished",
        delegated_status: {
          latest_run: {
            harness: "codex",
            provider_session_id: "thread-123",
            effective_model_id: "gpt-5.4",
            logical_effort: "high",
            input_tokens: 120,
            output_tokens: 45,
          },
        },
      }),
    });

    const mergedToolCalls = mergeDelegationToolCalls([startToolCall, waitToolCall]);
    expect(mergedToolCalls).toHaveLength(1);
    expect(mergedToolCalls[0]?.id).toBe("toolu-delegate-start");

    const mergedMetadata = extractDelegationMetadata(
      mergedToolCalls[0]?.arguments,
      mergedToolCalls[0]?.result,
    );
    expect(mergedMetadata.status).toBe("completed");
    expect(mergedMetadata.textOutput).toBe("Delegated review finished");
    expect(mergedMetadata.providerHarness).toBe("codex");
    expect(mergedMetadata.totalTokens).toBe(165);
  });

  it("normalizes persisted delegation transcript payloads with one shared contract", () => {
    const startBlock = makeContentToolUse("delegate_start", {
      id: "toolu-delegate-start",
      arguments: {
        agent_name: "ralphx-execution-reviewer",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "running",
      }),
    });
    const waitBlock = makeContentToolUse("delegate_wait", {
      id: "toolu-delegate-wait",
      arguments: {
        job_id: "job-123",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "completed",
        content: "Delegated review finished",
      }),
    });
    const startToolCall = makeToolCall("delegate_start", {
      id: "toolu-delegate-start",
      arguments: {
        agent_name: "ralphx-execution-reviewer",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "running",
      }),
    });
    const waitToolCall = makeToolCall("delegate_wait", {
      id: "toolu-delegate-wait",
      arguments: {
        job_id: "job-123",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "completed",
        content: "Delegated review finished",
      }),
    });

    const normalized = normalizeDelegationTranscriptPayload({
      contentBlocks: [startBlock, waitBlock],
      toolCalls: [startToolCall, waitToolCall],
    });

    expect(normalized.contentBlocks).toHaveLength(1);
    expect(normalized.toolCalls).toHaveLength(1);

    const mergedBlockMetadata = extractDelegationMetadata(
      normalized.contentBlocks[0]?.arguments,
      normalized.contentBlocks[0]?.result,
    );
    const mergedToolMetadata = extractDelegationMetadata(
      normalized.toolCalls[0]?.arguments,
      normalized.toolCalls[0]?.result,
    );

    expect(mergedBlockMetadata.status).toBe("completed");
    expect(mergedBlockMetadata.textOutput).toBe("Delegated review finished");
    expect(mergedToolMetadata.status).toBe("completed");
    expect(mergedToolMetadata.textOutput).toBe("Delegated review finished");
  });

  it("keeps direct block-level merging behavior aligned with the shared transcript contract", () => {
    const startBlock = makeContentToolUse("delegate_start", {
      id: "toolu-delegate-start",
      arguments: {
        agent_name: "ralphx-execution-reviewer",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "running",
      }),
    });
    const waitBlock = makeContentToolUse("delegate_wait", {
      id: "toolu-delegate-wait",
      arguments: {
        job_id: "job-123",
      },
      result: makeDelegationResult({
        job_id: "job-123",
        status: "completed",
        content: "Delegated review finished",
      }),
    });

    const mergedBlocks = mergeDelegationContentBlocks([startBlock, waitBlock]);
    expect(mergedBlocks).toHaveLength(1);

    const metadata = extractDelegationMetadata(
      mergedBlocks[0]?.arguments,
      mergedBlocks[0]?.result,
    );
    expect(metadata.status).toBe("completed");
    expect(metadata.textOutput).toBe("Delegated review finished");
  });
});
