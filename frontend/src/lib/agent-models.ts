export type AgentProvider = "claude" | "codex";
export type AgentEffort = "low" | "medium" | "high" | "xhigh";

export interface AgentRuntimeSelection {
  provider: AgentProvider;
  modelId: string;
  effort: AgentEffort;
}

export interface AgentEffortCatalogEntry {
  id: AgentEffort;
  label: string;
  description: string;
}

export interface AgentModelCatalogEntry {
  id: string;
  label: string;
  menuLabel: string;
  defaultEffort: AgentEffort;
  description?: string;
}

export const AGENT_EFFORT_CATALOG = [
  {
    id: "low",
    label: "Low",
    description: "Fastest responses with lighter reasoning.",
  },
  {
    id: "medium",
    label: "Medium",
    description: "Balanced reasoning depth for everyday tasks.",
  },
  {
    id: "high",
    label: "High",
    description: "Greater reasoning depth for complex work.",
  },
  {
    id: "xhigh",
    label: "Maximum",
    description: "Extra reasoning depth for the hardest tasks.",
  },
] as const satisfies readonly AgentEffortCatalogEntry[];

const CLAUDE_MODEL_CATALOG = [
  { id: "sonnet", label: "sonnet", menuLabel: "sonnet", defaultEffort: "medium" },
  { id: "opus", label: "opus", menuLabel: "opus", defaultEffort: "high" },
  { id: "haiku", label: "haiku", menuLabel: "haiku", defaultEffort: "medium" },
] as const satisfies readonly AgentModelCatalogEntry[];

export const CODEX_MODEL_CATALOG = [
  {
    id: "gpt-5.5",
    label: "gpt-5.5 - Frontier model for complex coding, research, and real-world work.",
    menuLabel: "gpt-5.5 (Current)",
    defaultEffort: "xhigh",
    description: "Frontier model for complex coding, research, and real-world work.",
  },
  {
    id: "gpt-5.4",
    label: "gpt-5.4 - Strong model for everyday coding.",
    menuLabel: "gpt-5.4",
    defaultEffort: "xhigh",
    description: "Strong model for everyday coding.",
  },
  {
    id: "gpt-5.4-mini",
    label: "gpt-5.4-mini - Small, fast, and cost-efficient model for simpler coding tasks.",
    menuLabel: "gpt-5.4-mini",
    defaultEffort: "medium",
    description: "Small, fast, and cost-efficient model for simpler coding tasks.",
  },
  {
    id: "gpt-5.3-codex",
    label: "gpt-5.3-codex - Coding-optimized model.",
    menuLabel: "gpt-5.3-codex",
    defaultEffort: "high",
    description: "Coding-optimized model.",
  },
  {
    id: "gpt-5.3-codex-spark",
    label: "gpt-5.3-codex-spark - Ultra-fast coding model.",
    menuLabel: "gpt-5.3-codex-spark",
    defaultEffort: "medium",
    description: "Ultra-fast coding model.",
  },
] as const satisfies readonly AgentModelCatalogEntry[];

export const AGENT_MODEL_CATALOG: Record<AgentProvider, readonly AgentModelCatalogEntry[]> = {
  claude: CLAUDE_MODEL_CATALOG,
  codex: CODEX_MODEL_CATALOG,
};

export const DEFAULT_CODEX_MODEL_ID = CODEX_MODEL_CATALOG[0].id;

function isAgentProvider(value: unknown): value is AgentProvider {
  return value === "claude" || value === "codex";
}

function isAgentEffort(value: unknown): value is AgentEffort {
  return AGENT_EFFORT_CATALOG.some((effort) => effort.id === value);
}

function defaultModelEntryForProvider(provider: AgentProvider): AgentModelCatalogEntry {
  return AGENT_MODEL_CATALOG[provider][0] ?? CODEX_MODEL_CATALOG[0];
}

function modelEntryForProvider(
  provider: AgentProvider,
  modelId: unknown
): AgentModelCatalogEntry {
  if (typeof modelId !== "string") {
    return defaultModelEntryForProvider(provider);
  }
  return (
    AGENT_MODEL_CATALOG[provider].find((model) => model.id === modelId) ??
    defaultModelEntryForProvider(provider)
  );
}

export function defaultModelForProvider(provider: AgentProvider): string {
  return defaultModelEntryForProvider(provider).id;
}

export function defaultEffortForModel(
  provider: AgentProvider,
  modelId: string
): AgentEffort {
  return modelEntryForProvider(provider, modelId).defaultEffort;
}

export function normalizeAgentRuntimeSelection(
  runtime: unknown
): AgentRuntimeSelection {
  const defaultEntry = defaultModelEntryForProvider("codex");
  const defaultRuntime: AgentRuntimeSelection = {
    provider: "codex",
    modelId: defaultEntry.id,
    effort: defaultEntry.defaultEffort,
  };

  if (!runtime || typeof runtime !== "object") {
    return defaultRuntime;
  }

  const candidate = runtime as Partial<Record<keyof AgentRuntimeSelection, unknown>>;
  if (!isAgentProvider(candidate.provider)) {
    return defaultRuntime;
  }

  const provider = candidate.provider;
  const requestedModelId = typeof candidate.modelId === "string" ? candidate.modelId : "";
  const hasRequestedModel = AGENT_MODEL_CATALOG[provider].some(
    (model) => model.id === requestedModelId
  );
  const model = modelEntryForProvider(provider, requestedModelId);
  const effort =
    hasRequestedModel && isAgentEffort(candidate.effort)
      ? candidate.effort
      : model.defaultEffort;

  return {
    provider,
    modelId: model.id,
    effort,
  };
}
