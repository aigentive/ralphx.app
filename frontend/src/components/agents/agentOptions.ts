import type {
  AgentEffort,
  AgentProvider,
  AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import {
  AGENT_EFFORT_CATALOG,
  AGENT_MODEL_CATALOG,
  defaultEffortForModel,
  defaultModelForProvider,
  normalizeAgentRuntimeSelection,
} from "@/lib/agent-models";

export interface AgentModelOption {
  id: string;
  label: string;
  description?: string;
}

export interface AgentEffortOption {
  id: AgentEffort;
  label: string;
  description?: string;
}

export const AGENT_PROVIDER_OPTIONS: Array<{ id: AgentProvider; label: string }> = [
  { id: "claude", label: "Claude" },
  { id: "codex", label: "Codex" },
];

export const AGENT_MODEL_OPTIONS: Record<AgentProvider, AgentModelOption[]> = {
  claude: AGENT_MODEL_CATALOG.claude.map(({ id, menuLabel, description }) => ({
    id,
    label: menuLabel,
    ...(description ? { description } : {}),
  })),
  codex: AGENT_MODEL_CATALOG.codex.map(({ id, menuLabel, description }) => ({
    id,
    label: menuLabel,
    ...(description ? { description } : {}),
  })),
};

export const AGENT_EFFORT_OPTIONS: AgentEffortOption[] = AGENT_EFFORT_CATALOG.map(
  ({ id, label, description }) => ({
    id,
    label,
    description,
  })
);

export const DEFAULT_AGENT_RUNTIME: AgentRuntimeSelection =
  normalizeAgentRuntimeSelection(null);

export { defaultEffortForModel, defaultModelForProvider };

export function normalizeRuntimeSelection(
  runtime: unknown
): AgentRuntimeSelection {
  return normalizeAgentRuntimeSelection(runtime);
}
