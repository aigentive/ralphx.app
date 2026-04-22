import type { AgentProvider, AgentRuntimeSelection } from "@/stores/agentSessionStore";

export interface AgentModelOption {
  id: string;
  label: string;
}

export const AGENT_PROVIDER_OPTIONS: Array<{ id: AgentProvider; label: string }> = [
  { id: "claude", label: "Claude" },
  { id: "codex", label: "Codex" },
];

export const AGENT_MODEL_OPTIONS: Record<AgentProvider, AgentModelOption[]> = {
  claude: [
    { id: "sonnet", label: "sonnet" },
    { id: "opus", label: "opus" },
    { id: "haiku", label: "haiku" },
  ],
  codex: [
    { id: "gpt-5.4", label: "gpt-5.4" },
    { id: "gpt-5.4-mini", label: "gpt-5.4-mini" },
    { id: "gpt-5.3-codex", label: "gpt-5.3-codex" },
    { id: "gpt-5.3-codex-spark", label: "gpt-5.3-codex-spark" },
  ],
};

export const DEFAULT_AGENT_RUNTIME: AgentRuntimeSelection = {
  provider: "codex",
  modelId: "gpt-5.4",
};

export function defaultModelForProvider(provider: AgentProvider): string {
  return AGENT_MODEL_OPTIONS[provider][0]?.id ?? DEFAULT_AGENT_RUNTIME.modelId;
}

export function normalizeRuntimeSelection(
  runtime: AgentRuntimeSelection | null | undefined
): AgentRuntimeSelection {
  if (!runtime) {
    return DEFAULT_AGENT_RUNTIME;
  }

  const availableModels = AGENT_MODEL_OPTIONS[runtime.provider] ?? [];
  if (availableModels.some((model) => model.id === runtime.modelId)) {
    return runtime;
  }

  return {
    provider: runtime.provider,
    modelId: defaultModelForProvider(runtime.provider),
  };
}
