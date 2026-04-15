import type { ToolCall } from "./tool-widgets/shared.constants";
import { parseMcpToolResultRaw } from "./tool-widgets/shared.constants";
import { canonicalizeToolName } from "./tool-widgets/tool-name";
import { normalizeDelegationTranscriptPayload } from "./delegation-tool-calls";
import type { StreamingContentBlock } from "@/types/streaming-task";

type VerificationMergeable = {
  name?: string;
  arguments?: unknown;
  result?: unknown;
  error?: string;
};

type VerificationContentBlock = VerificationMergeable & {
  type: string;
};

interface NormalizeToolCallTranscriptPayloadArgs<
  TContentBlock extends VerificationContentBlock,
  TToolCall extends ToolCall,
> {
  contentBlocks?: TContentBlock[] | null | undefined;
  toolCalls?: TToolCall[] | null | undefined;
}

interface NormalizedToolCallTranscriptPayload<
  TContentBlock extends VerificationContentBlock,
  TToolCall extends ToolCall,
> {
  contentBlocks: TContentBlock[];
  toolCalls: TToolCall[];
}

const RUN_VERIFICATION_ENRICHMENT = "run_verification_enrichment";
const RUN_VERIFICATION_ROUND = "run_verification_round";
const REPORT_VERIFICATION_ROUND = "report_verification_round";
const COMPLETE_PLAN_VERIFICATION = "complete_plan_verification";

function asRecord(value: unknown): Record<string, unknown> | null {
  return value != null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function getNumber(value: unknown, key: string): number | undefined {
  const record = asRecord(value);
  const candidate = record?.[key];
  return typeof candidate === "number" ? candidate : undefined;
}

function getArray(value: unknown, key: string): unknown[] | undefined {
  const record = asRecord(value);
  const candidate = record?.[key];
  return Array.isArray(candidate) ? candidate : undefined;
}

function verificationToolName(name: string | undefined): string | null {
  if (!name) return null;
  const canonical = canonicalizeToolName(name);
  switch (canonical) {
    case RUN_VERIFICATION_ENRICHMENT:
    case RUN_VERIFICATION_ROUND:
    case REPORT_VERIFICATION_ROUND:
    case COMPLETE_PLAN_VERIFICATION:
      return canonical;
    default:
      return null;
  }
}

function toVerificationMergeable(name: string | undefined, args: unknown, result: unknown): VerificationMergeable {
  const mergeable: VerificationMergeable = {};
  if (name !== undefined) {
    mergeable.name = name;
  }
  if (args !== undefined) {
    mergeable.arguments = args;
  }
  if (result !== undefined) {
    mergeable.result = result;
  }
  return mergeable;
}

function toolRound(entry: VerificationMergeable): number | undefined {
  const parsed = parseMcpToolResultRaw(entry.result);
  return getNumber(parsed, "round")
    ?? getNumber(parsed, "current_round")
    ?? getNumber(entry.arguments, "round");
}

function hasMeaningfulEnrichmentResult(entry: VerificationMergeable): boolean {
  const parsed = parseMcpToolResultRaw(entry.result);
  const selectedSpecialists = getArray(parsed, "selected_specialists");
  const delegateSnapshots = getArray(parsed, "delegate_snapshots");
  const findings = getArray(parsed, "findings_by_critic");

  return (selectedSpecialists?.length ?? 0) > 0
    || (delegateSnapshots?.length ?? 0) > 0
    || (findings?.length ?? 0) > 0;
}

function mergeVerificationEntries<T>(entries: T[], accessors: {
  name: (entry: T) => string | undefined;
  arguments: (entry: T) => unknown;
  result: (entry: T) => unknown;
}): T[] {
  const keep = new Array(entries.length).fill(true);
  let sawLaterVerificationStep = false;
  const laterReportedRounds = new Set<number>();
  const laterCompletedRounds = new Set<number>();

  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const entry = entries[index];
    if (entry == null) continue;

    const toolName = verificationToolName(accessors.name(entry));
    if (!toolName) continue;

    const round = toolRound(
      toVerificationMergeable(
        accessors.name(entry),
        accessors.arguments(entry),
        accessors.result(entry),
      )
    );

    if (toolName === COMPLETE_PLAN_VERIFICATION) {
      sawLaterVerificationStep = true;
      if (round != null) {
        laterCompletedRounds.add(round);
      }
      continue;
    }

    if (toolName === REPORT_VERIFICATION_ROUND) {
      sawLaterVerificationStep = true;
      if (round != null) {
        laterReportedRounds.add(round);
      }
      continue;
    }

    if (toolName === RUN_VERIFICATION_ROUND) {
      const hasLaterResolution = round != null
        ? laterReportedRounds.has(round) || laterCompletedRounds.has(round)
        : laterReportedRounds.size > 0 || laterCompletedRounds.size > 0;
      if (hasLaterResolution) {
        keep[index] = false;
      }
      sawLaterVerificationStep = true;
      continue;
    }

    if (
      toolName === RUN_VERIFICATION_ENRICHMENT
      && sawLaterVerificationStep
      && !hasMeaningfulEnrichmentResult(
        toVerificationMergeable(
          accessors.name(entry),
          accessors.arguments(entry),
          accessors.result(entry),
        )
      )
    ) {
      keep[index] = false;
    }
  }

  return entries.filter((_, index) => keep[index]);
}

export function normalizeToolCallTranscriptPayload<
  TContentBlock extends VerificationContentBlock,
  TToolCall extends ToolCall,
>({
  contentBlocks,
  toolCalls,
}: NormalizeToolCallTranscriptPayloadArgs<TContentBlock, TToolCall>): NormalizedToolCallTranscriptPayload<TContentBlock, TToolCall> {
  const delegated = normalizeDelegationTranscriptPayload<TContentBlock, TToolCall>({
    contentBlocks,
    toolCalls,
  });

  return {
    contentBlocks: mergeVerificationEntries(delegated.contentBlocks, {
      name: (entry) => entry.name,
      arguments: (entry) => entry.arguments,
      result: (entry) => entry.result,
    }),
    toolCalls: mergeVerificationEntries(delegated.toolCalls, {
      name: (entry) => entry.name,
      arguments: (entry) => entry.arguments,
      result: (entry) => entry.result,
    }),
  };
}

export function normalizeStreamingVerificationContentBlocks(
  contentBlocks: StreamingContentBlock[] | null | undefined,
): StreamingContentBlock[] {
  if (!contentBlocks || contentBlocks.length === 0) {
    return [];
  }

  return mergeVerificationEntries(contentBlocks, {
    name: (entry) => entry.type === "tool_use" ? entry.toolCall.name : undefined,
    arguments: (entry) => entry.type === "tool_use" ? entry.toolCall.arguments : undefined,
    result: (entry) => entry.type === "tool_use" ? entry.toolCall.result : undefined,
  });
}
