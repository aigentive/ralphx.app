// Tauri invoke wrappers for ExternalMcpConfig settings

import { invoke } from "@tauri-apps/api/core";

export interface ExternalMcpConfigView {
  enabled: boolean;
  port: number;
  host: string;
  authToken: string | null;  // masked "••••••••" if set, null if unset
  nodePath: string | null;
}

export interface ExternalMcpConfigUpdate {
  enabled?: boolean;
  port?: number;
  host?: string;
  authToken?: string;
  nodePath?: string;
}

export async function getExternalMcpConfig(): Promise<ExternalMcpConfigView> {
  return invoke<ExternalMcpConfigView>("get_external_mcp_config");
}

export async function updateExternalMcpConfig(input: ExternalMcpConfigUpdate): Promise<void> {
  return invoke("update_external_mcp_config", { input });
}
