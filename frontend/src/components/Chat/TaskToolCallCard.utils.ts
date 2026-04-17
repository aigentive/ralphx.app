import type { ToolCall } from "./tool-widgets/shared.constants";

export interface TaskArgs {
  description: string | undefined;
  subagent_type: string | undefined;
  model: string | undefined;
  prompt: string | undefined;
  name: string | undefined;
  isolation: string | undefined;
  run_in_background: boolean | undefined;
}

export interface TaskStats {
  statsAvailable: boolean;
  agentId: string | undefined;
  totalDurationMs: number | undefined;
  totalTokens: number | undefined;
  totalToolUseCount: number | undefined;
  model: string | undefined;
  textOutput: string | undefined;
  estimatedUsd?: number;
}

const EMPTY_ARGS: TaskArgs = {
  description: undefined,
  subagent_type: undefined,
  model: undefined,
  prompt: undefined,
  name: undefined,
  isolation: undefined,
  run_in_background: undefined,
};

export const EMPTY_STATS: TaskStats = {
  statsAvailable: false,
  agentId: undefined,
  totalDurationMs: undefined,
  totalTokens: undefined,
  totalToolUseCount: undefined,
  model: undefined,
  textOutput: undefined,
};

export function extractTaskArgs(args: unknown): TaskArgs {
  if (!args || typeof args !== "object") return EMPTY_ARGS;
  const a = args as Record<string, unknown>;
  return {
    description: typeof a.description === "string" ? a.description : undefined,
    subagent_type: typeof a.subagent_type === "string" ? a.subagent_type : undefined,
    model: typeof a.model === "string" ? a.model : undefined,
    prompt: typeof a.prompt === "string" ? a.prompt : undefined,
    name: typeof a.name === "string" ? a.name : undefined,
    isolation: typeof a.isolation === "string" ? a.isolation : undefined,
    run_in_background: typeof a.run_in_background === "boolean" ? a.run_in_background : undefined,
  };
}

export function extractChildToolCalls(result: unknown): ToolCall[] {
  if (!Array.isArray(result)) return [];

  const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];
  const toolResultMap = new Map<string, unknown>();

  for (const block of result) {
    if (!block || typeof block !== "object") continue;
    const b = block as Record<string, unknown>;

    if (b.type === "tool_use" && typeof b.name === "string" && typeof b.id === "string") {
      toolUseBlocks.push({ id: b.id, name: b.name, input: b.input });
    } else if (b.type === "tool_result" && typeof b.tool_use_id === "string") {
      toolResultMap.set(b.tool_use_id, b.content);
    }
  }

  return toolUseBlocks.map((tu) => {
    const tc: ToolCall = { id: tu.id, name: tu.name, arguments: tu.input };
    const resultContent = toolResultMap.get(tu.id);
    if (resultContent != null) {
      tc.result = resultContent;
    }
    return tc;
  });
}

function extractTaskStatsFromResult(result: unknown): TaskStats {
  if (result == null) return EMPTY_STATS;

  let text: string;
  if (typeof result === "string") {
    text = result;
  } else if (Array.isArray(result)) {
    const textBlocks = result.filter(
      (b: unknown) => b && typeof b === "object" && (b as Record<string, unknown>).type === "text",
    );
    if (textBlocks.length === 0) {
      return { ...EMPTY_STATS, statsAvailable: false };
    }
    text = textBlocks.map((b: unknown) => (b as Record<string, unknown>).text as string).join("\n");
  } else if (typeof result === "object") {
    const obj = result as Record<string, unknown>;
    if (typeof obj.text === "string") {
      text = obj.text;
    } else {
      return { ...EMPTY_STATS, statsAvailable: false };
    }
  } else {
    return EMPTY_STATS;
  }

  let agentId: string | undefined;
  let totalDurationMs: number | undefined;
  let totalTokens: number | undefined;
  let totalToolUseCount: number | undefined;
  let textOutput: string | undefined;

  const agentIdMatch = text.match(/agentId:\s*([a-fA-F0-9]+)/i);
  if (agentIdMatch) {
    agentId = agentIdMatch[1];
  }

  const usageMatch = text.match(/<usage>([\s\S]*?)<\/usage>/);
  if (usageMatch) {
    const usage = usageMatch[1] ?? "";
    const tokensMatch = usage.match(/total_tokens:\s*(\d+)/);
    const toolsMatch = usage.match(/tool_uses:\s*(\d+)/);
    const durationMatch = usage.match(/duration_ms:\s*(\d+)/);

    if (tokensMatch) totalTokens = parseInt(tokensMatch[1]!, 10);
    if (toolsMatch) totalToolUseCount = parseInt(toolsMatch[1]!, 10);
    if (durationMatch) totalDurationMs = parseInt(durationMatch[1]!, 10);
  }

  const agentIdPos = text.search(/(?:^|\n)agentId:/);
  if (agentIdPos >= 0) {
    textOutput = text.slice(0, agentIdPos).trim() || undefined;
  } else if (!usageMatch) {
    textOutput = text.trim() || undefined;
  }

  return { statsAvailable: true, agentId, totalDurationMs, totalTokens, totalToolUseCount, model: undefined, textOutput };
}

export function extractTaskStats(toolCall: ToolCall): TaskStats {
  if (toolCall.stats !== undefined) {
    const fromResult = extractTaskStatsFromResult(toolCall.result);
    return {
      statsAvailable: true,
      agentId: fromResult.agentId,
      totalDurationMs: toolCall.stats.durationMs,
      totalTokens: toolCall.stats.totalTokens,
      totalToolUseCount: toolCall.stats.totalToolUses,
      model: toolCall.stats.model,
      textOutput: fromResult.textOutput,
    };
  }

  if (import.meta.env.DEV) {
    console.debug("[extractTaskStats] text-parsing fallback for tool call:", toolCall.id);
  }
  return extractTaskStatsFromResult(toolCall.result);
}
