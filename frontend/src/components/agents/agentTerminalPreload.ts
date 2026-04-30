import { preloadAgentTerminalRuntime } from "./agentTerminalRuntime";

let drawerPromise: Promise<typeof import("./AgentTerminalDrawer")> | null = null;

export function preloadAgentTerminalDrawer() {
  drawerPromise ??= import("./AgentTerminalDrawer").catch((error) => {
    drawerPromise = null;
    throw error;
  });
  return drawerPromise;
}

export function preloadAgentTerminalExperience() {
  void preloadAgentTerminalDrawer().catch(() => undefined);
  preloadAgentTerminalRuntime();
}
