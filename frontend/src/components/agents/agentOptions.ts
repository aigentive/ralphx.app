import type { AgentProvider, AgentRuntimeSelection } from "@/stores/agentSessionStore";
import { AGENT_MODEL_CATALOG, DEFAULT_CODEX_MODEL_ID } from "@/lib/agent-models";

export interface AgentModelOption {
  id: string;
  label: string;
}

export const AGENT_PROVIDER_OPTIONS: Array<{ id: AgentProvider; label: string }> = [
  { id: "claude", label: "Claude" },
  { id: "codex", label: "Codex" },
];

export const AGENT_MODEL_OPTIONS: Record<AgentProvider, AgentModelOption[]> = {
  claude: AGENT_MODEL_CATALOG.claude.map(({ id, label }) => ({ id, label })),
  codex: AGENT_MODEL_CATALOG.codex.map(({ id, label }) => ({ id, label })),
};

export const DEFAULT_AGENT_RUNTIME: AgentRuntimeSelection = {
  provider: "codex",
  modelId: DEFAULT_CODEX_MODEL_ID,
};

function isAgentProvider(value: unknown): value is AgentProvider {
  return AGENT_PROVIDER_OPTIONS.some((provider) => provider.id === value);
}

export function defaultModelForProvider(provider: AgentProvider): string {
  return AGENT_MODEL_OPTIONS[provider]?.[0]?.id ?? DEFAULT_AGENT_RUNTIME.modelId;
}

export function normalizeRuntimeSelection(
  runtime: unknown
): AgentRuntimeSelection {
  if (!runtime || typeof runtime !== "object") {
    return DEFAULT_AGENT_RUNTIME;
  }

  const candidate = runtime as Partial<Record<keyof AgentRuntimeSelection, unknown>>;
  if (!isAgentProvider(candidate.provider)) {
    return DEFAULT_AGENT_RUNTIME;
  }

  const provider = candidate.provider;
  const modelId = typeof candidate.modelId === "string" ? candidate.modelId : "";
  const availableModels = AGENT_MODEL_OPTIONS[provider];

  if (availableModels.some((model) => model.id === modelId)) {
    return { provider, modelId };
  }

  return {
    provider,
    modelId: defaultModelForProvider(provider),
  };
}
