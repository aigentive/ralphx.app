type XTermModule = typeof import("@xterm/xterm");
type FitAddonModule = typeof import("@xterm/addon-fit");

let runtimePromise: Promise<{
  Terminal: XTermModule["Terminal"];
  FitAddon: FitAddonModule["FitAddon"];
}> | null = null;

export function loadAgentTerminalRuntime() {
  runtimePromise ??= Promise.all([
    import("@xterm/xterm"),
    import("@xterm/addon-fit"),
  ]).then(([xterm, fitAddon]) => ({
    Terminal: xterm.Terminal,
    FitAddon: fitAddon.FitAddon,
  })).catch((error) => {
    runtimePromise = null;
    throw error;
  });

  return runtimePromise;
}

export function preloadAgentTerminalRuntime() {
  void loadAgentTerminalRuntime().catch(() => undefined);
}
