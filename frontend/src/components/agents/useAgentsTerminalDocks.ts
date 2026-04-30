import { useState } from "react";

export function useAgentsTerminalDocks() {
  const [terminalChatDockElement, setTerminalChatDockElement] =
    useState<HTMLDivElement | null>(null);
  const [terminalPanelDockElement, setTerminalPanelDockElement] =
    useState<HTMLDivElement | null>(null);

  return {
    setTerminalChatDockElement,
    setTerminalPanelDockElement,
    terminalChatDockElement,
    terminalPanelDockElement,
  };
}
