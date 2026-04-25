import type { AgentProvider } from "@/stores/agentSessionStore";

export interface AgentModelCatalogEntry {
  id: string;
  label: string;
  menuLabel: string;
  description?: string;
}

const CLAUDE_MODEL_CATALOG = [
  { id: "sonnet", label: "sonnet", menuLabel: "sonnet" },
  { id: "opus", label: "opus", menuLabel: "opus" },
  { id: "haiku", label: "haiku", menuLabel: "haiku" },
] as const satisfies readonly AgentModelCatalogEntry[];

export const CODEX_MODEL_CATALOG = [
  {
    id: "gpt-5.5",
    label: "gpt-5.5 - Frontier model for complex coding, research, and real-world work.",
    menuLabel: "gpt-5.5 (Current)",
    description: "Frontier model for complex coding, research, and real-world work.",
  },
  {
    id: "gpt-5.4",
    label: "gpt-5.4 - Strong model for everyday coding.",
    menuLabel: "gpt-5.4",
    description: "Strong model for everyday coding.",
  },
  {
    id: "gpt-5.4-mini",
    label: "gpt-5.4-mini - Small, fast, and cost-efficient model for simpler coding tasks.",
    menuLabel: "gpt-5.4-mini",
    description: "Small, fast, and cost-efficient model for simpler coding tasks.",
  },
  {
    id: "gpt-5.3-codex",
    label: "gpt-5.3-codex - Coding-optimized model.",
    menuLabel: "gpt-5.3-codex",
    description: "Coding-optimized model.",
  },
  {
    id: "gpt-5.3-codex-spark",
    label: "gpt-5.3-codex-spark - Ultra-fast coding model.",
    menuLabel: "gpt-5.3-codex-spark",
    description: "Ultra-fast coding model.",
  },
] as const satisfies readonly AgentModelCatalogEntry[];

export const AGENT_MODEL_CATALOG: Record<AgentProvider, readonly AgentModelCatalogEntry[]> = {
  claude: CLAUDE_MODEL_CATALOG,
  codex: CODEX_MODEL_CATALOG,
};

export const DEFAULT_CODEX_MODEL_ID = CODEX_MODEL_CATALOG[0].id;
