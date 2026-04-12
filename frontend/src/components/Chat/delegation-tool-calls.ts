import type { ToolCall } from "./tool-widgets/shared.constants";
import { parseMcpToolResultRaw } from "./tool-widgets/shared.constants";

export const DELEGATION_START_TOOL_NAME = "delegate_start";
export const DELEGATION_WAIT_TOOL_NAME = "delegate_wait";
export const DELEGATION_CANCEL_TOOL_NAME = "delegate_cancel";

type UnknownRecord = Record<string, unknown>;

export interface DelegationMetadata {
  jobId?: string;
  status?: string;
  agentName?: string;
  prompt?: string;
  title?: string;
  providerHarness?: string;
  providerSessionId?: string;
  upstreamProvider?: string;
  providerProfile?: string;
  delegatedSessionId?: string;
  delegatedConversationId?: string;
  delegatedAgentRunId?: string;
  logicalModel?: string;
  effectiveModelId?: string;
  logicalEffort?: string;
  effectiveEffort?: string;
  approvalPolicy?: string;
  sandboxMode?: string;
  inputTokens?: number;
  outputTokens?: number;
  cacheCreationTokens?: number;
  cacheReadTokens?: number;
  totalTokens?: number;
  estimatedUsd?: number;
  durationMs?: number;
  textOutput?: string;
}

function asRecord(value: unknown): UnknownRecord | null {
  return value != null && typeof value === "object" && !Array.isArray(value)
    ? (value as UnknownRecord)
    : null;
}

function getFirstRecord(record: UnknownRecord | null, ...keys: string[]): UnknownRecord | null {
  if (!record) return null;
  for (const key of keys) {
    const nested = asRecord(record[key]);
    if (nested) return nested;
  }
  return null;
}

function getFirstString(record: UnknownRecord | null, ...keys: string[]): string | undefined {
  if (!record) return undefined;
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.length > 0) {
      return value;
    }
  }
  return undefined;
}

function getFirstNumber(record: UnknownRecord | null, ...keys: string[]): number | undefined {
  if (!record) return undefined;
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "number" && Number.isFinite(value)) {
      return value;
    }
  }
  return undefined;
}

function getLastMessageText(messages: unknown): string | undefined {
  if (!Array.isArray(messages)) return undefined;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = asRecord(messages[index]);
    const content = getFirstString(message, "content");
    if (content) return content;
  }
  return undefined;
}

function deriveDurationMs(startedAt?: string, completedAt?: string): number | undefined {
  if (!startedAt || !completedAt) return undefined;
  const started = Date.parse(startedAt);
  const completed = Date.parse(completedAt);
  if (!Number.isFinite(started) || !Number.isFinite(completed) || completed < started) {
    return undefined;
  }
  return completed - started;
}

function normalizeStatus(status: string | undefined): string | undefined {
  switch (status) {
    case "running":
    case "completed":
    case "failed":
    case "cancelled":
      return status;
    default:
      return status;
  }
}

export function isDelegationStartToolCall(name: string): boolean {
  return name.toLowerCase() === DELEGATION_START_TOOL_NAME;
}

export function isDelegationControlToolCall(name: string): boolean {
  const lowered = name.toLowerCase();
  return lowered === DELEGATION_WAIT_TOOL_NAME || lowered === DELEGATION_CANCEL_TOOL_NAME;
}

export function isDelegationToolCall(name: string): boolean {
  return isDelegationStartToolCall(name) || isDelegationControlToolCall(name);
}

export function extractDelegationMetadata(
  args: unknown,
  result: unknown,
): DelegationMetadata {
  const argRecord = asRecord(args);
  const resultRecord = asRecord(parseMcpToolResultRaw(result));
  const delegatedStatus =
    getFirstRecord(resultRecord, "delegated_status", "delegatedStatus");
  const latestRun = getFirstRecord(delegatedStatus, "latest_run", "latestRun");
  const session = getFirstRecord(delegatedStatus, "session");

  const inputTokens = getFirstNumber(latestRun, "input_tokens", "inputTokens");
  const outputTokens = getFirstNumber(latestRun, "output_tokens", "outputTokens");
  const cacheCreationTokens = getFirstNumber(
    latestRun,
    "cache_creation_tokens",
    "cacheCreationTokens",
  );
  const cacheReadTokens = getFirstNumber(
    latestRun,
    "cache_read_tokens",
    "cacheReadTokens",
  );

  const totalTokens =
    inputTokens != null ||
    outputTokens != null ||
    cacheCreationTokens != null ||
    cacheReadTokens != null
      ? (inputTokens ?? 0)
        + (outputTokens ?? 0)
        + (cacheCreationTokens ?? 0)
        + (cacheReadTokens ?? 0)
      : undefined;

  const textOutput =
    getFirstString(resultRecord, "content")
    ?? getLastMessageText(delegatedStatus?.recent_messages ?? delegatedStatus?.recentMessages);

  const jobId =
    getFirstString(resultRecord, "job_id", "jobId")
    ?? getFirstString(argRecord, "job_id", "jobId");
  const status = normalizeStatus(
    getFirstString(resultRecord, "status")
    ?? getFirstString(latestRun, "status")
    ?? getFirstString(session, "status"),
  );
  const agentName =
    getFirstString(resultRecord, "agent_name", "agentName")
    ?? getFirstString(argRecord, "agent_name", "agentName");
  const prompt = getFirstString(argRecord, "prompt");
  const title = getFirstString(argRecord, "title");
  const providerHarness =
    getFirstString(latestRun, "harness")
    ?? getFirstString(resultRecord, "harness")
    ?? getFirstString(session, "harness")
    ?? getFirstString(argRecord, "harness", "harness_override", "harnessOverride");
  const providerSessionId =
    getFirstString(latestRun, "provider_session_id", "providerSessionId")
    ?? getFirstString(session, "provider_session_id", "providerSessionId");
  const upstreamProvider =
    getFirstString(latestRun, "upstream_provider", "upstreamProvider");
  const providerProfile =
    getFirstString(latestRun, "provider_profile", "providerProfile");
  const delegatedSessionId =
    getFirstString(resultRecord, "delegated_session_id", "delegatedSessionId")
    ?? getFirstString(argRecord, "delegated_session_id", "delegatedSessionId");
  const delegatedConversationId =
    getFirstString(resultRecord, "delegated_conversation_id", "delegatedConversationId")
    ?? getFirstString(delegatedStatus, "conversation_id", "conversationId");
  const delegatedAgentRunId =
    getFirstString(resultRecord, "delegated_agent_run_id", "delegatedAgentRunId")
    ?? getFirstString(latestRun, "agent_run_id", "agentRunId");
  const logicalModel =
    getFirstString(latestRun, "logical_model", "logicalModel")
    ?? getFirstString(argRecord, "model", "logical_model", "logicalModel");
  const effectiveModelId =
    getFirstString(latestRun, "effective_model_id", "effectiveModelId");
  const logicalEffort =
    getFirstString(latestRun, "logical_effort", "logicalEffort")
    ?? getFirstString(argRecord, "logical_effort", "logicalEffort");
  const effectiveEffort =
    getFirstString(latestRun, "effective_effort", "effectiveEffort");
  const approvalPolicy =
    getFirstString(latestRun, "approval_policy", "approvalPolicy")
    ?? getFirstString(argRecord, "approval_policy", "approvalPolicy");
  const sandboxMode =
    getFirstString(latestRun, "sandbox_mode", "sandboxMode")
    ?? getFirstString(argRecord, "sandbox_mode", "sandboxMode");
  const estimatedUsd = getFirstNumber(latestRun, "estimated_usd", "estimatedUsd");
  const durationMs = deriveDurationMs(
    getFirstString(latestRun, "started_at", "startedAt"),
    getFirstString(latestRun, "completed_at", "completedAt"),
  );

  return {
    ...(jobId ? { jobId } : {}),
    ...(status ? { status } : {}),
    ...(agentName ? { agentName } : {}),
    ...(prompt ? { prompt } : {}),
    ...(title ? { title } : {}),
    ...(providerHarness ? { providerHarness } : {}),
    ...(providerSessionId ? { providerSessionId } : {}),
    ...(upstreamProvider ? { upstreamProvider } : {}),
    ...(providerProfile ? { providerProfile } : {}),
    ...(delegatedSessionId ? { delegatedSessionId } : {}),
    ...(delegatedConversationId ? { delegatedConversationId } : {}),
    ...(delegatedAgentRunId ? { delegatedAgentRunId } : {}),
    ...(logicalModel ? { logicalModel } : {}),
    ...(effectiveModelId ? { effectiveModelId } : {}),
    ...(logicalEffort ? { logicalEffort } : {}),
    ...(effectiveEffort ? { effectiveEffort } : {}),
    ...(approvalPolicy ? { approvalPolicy } : {}),
    ...(sandboxMode ? { sandboxMode } : {}),
    ...(inputTokens != null ? { inputTokens } : {}),
    ...(outputTokens != null ? { outputTokens } : {}),
    ...(cacheCreationTokens != null ? { cacheCreationTokens } : {}),
    ...(cacheReadTokens != null ? { cacheReadTokens } : {}),
    ...(totalTokens != null ? { totalTokens } : {}),
    ...(estimatedUsd != null ? { estimatedUsd } : {}),
    ...(durationMs != null ? { durationMs } : {}),
    ...(textOutput ? { textOutput } : {}),
  };
}

export function mergeDelegationToolCalls(toolCalls: ToolCall[]): ToolCall[] {
  const merged: ToolCall[] = [];
  const startIndexByJobId = new Map<string, number>();

  for (const toolCall of toolCalls) {
    if (!isDelegationToolCall(toolCall.name)) {
      merged.push(toolCall);
      continue;
    }

    const metadata = extractDelegationMetadata(toolCall.arguments, toolCall.result);

    if (isDelegationStartToolCall(toolCall.name)) {
      merged.push(toolCall);
      if (metadata.jobId) {
        startIndexByJobId.set(metadata.jobId, merged.length - 1);
      }
      continue;
    }

    if (metadata.jobId) {
      const startIndex = startIndexByJobId.get(metadata.jobId);
      if (startIndex != null) {
        const startToolCall = merged[startIndex];
        if (startToolCall) {
          merged[startIndex] = {
            ...startToolCall,
            result: toolCall.result ?? startToolCall.result,
            ...(toolCall.error || startToolCall.error
              ? { error: toolCall.error ?? startToolCall.error }
              : {}),
          };
          continue;
        }
      }
    }

    merged.push(toolCall);
  }

  return merged;
}

export function mergeDelegationContentBlocks<
  T extends { type: string; name?: string; arguments?: unknown; result?: unknown; error?: string },
>(blocks: T[]): T[] {
  const merged: T[] = [];
  const startIndexByJobId = new Map<string, number>();

  for (const block of blocks) {
    if (block.type !== "tool_use" || !block.name || !isDelegationToolCall(block.name)) {
      merged.push(block);
      continue;
    }

    const metadata = extractDelegationMetadata(block.arguments, block.result);

    if (isDelegationStartToolCall(block.name)) {
      merged.push(block);
      if (metadata.jobId) {
        startIndexByJobId.set(metadata.jobId, merged.length - 1);
      }
      continue;
    }

    if (metadata.jobId) {
      const startIndex = startIndexByJobId.get(metadata.jobId);
      if (startIndex != null) {
        const startBlock = merged[startIndex];
        if (startBlock) {
          merged[startIndex] = {
            ...startBlock,
            result: block.result ?? startBlock.result,
            ...(block.error || startBlock.error
              ? { error: block.error ?? startBlock.error }
              : {}),
          };
          continue;
        }
      }
    }

    merged.push(block);
  }

  return merged;
}
