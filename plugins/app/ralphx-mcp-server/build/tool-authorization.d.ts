/**
 * Compatibility mirror derived from canonical `agents/<agent>/agent.yaml` metadata.
 * Runtime resolution should prefer CLI/env overrides, then canonical agent metadata.
 */
export declare const TOOL_ALLOWLIST: Record<string, string[]>;
export declare const LEGACY_TOOL_ALLOWLIST: Record<string, string[]>;
export declare function setAgentType(agentType: string): void;
export declare function getAgentType(): string;
export declare function parseAllowedToolsFromArgs(knownToolNames: string[]): string[] | undefined;
export declare function getAllowedToolNames(knownToolNames: string[]): string[];
export declare function getToolsByAgent(knownToolNames: string[]): Record<string, string[]>;
//# sourceMappingURL=tool-authorization.d.ts.map