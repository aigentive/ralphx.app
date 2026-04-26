import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AgentConversationWorkspace } from "@/api/chat";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AgentTerminalDrawer } from "./AgentTerminalDrawer";

const {
  listenMock,
  openAgentTerminalMock,
  closeAgentTerminalMock,
  clearAgentTerminalMock,
  resizeAgentTerminalMock,
  restartAgentTerminalMock,
  writeAgentTerminalMock,
  terminalOpenMock,
} = vi.hoisted(() => ({
  listenMock: vi.fn(),
  openAgentTerminalMock: vi.fn(),
  closeAgentTerminalMock: vi.fn(),
  clearAgentTerminalMock: vi.fn(),
  resizeAgentTerminalMock: vi.fn(),
  restartAgentTerminalMock: vi.fn(),
  writeAgentTerminalMock: vi.fn(),
  terminalOpenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

vi.mock("@/api/terminal", () => ({
  AGENT_TERMINAL_EVENT: "agent-terminal://event",
  AgentTerminalEventSchema: {
    safeParse: vi.fn(() => ({ success: false })),
  },
  DEFAULT_AGENT_TERMINAL_ID: "default",
  closeAgentTerminal: (...args: unknown[]) => closeAgentTerminalMock(...args),
  clearAgentTerminal: (...args: unknown[]) => clearAgentTerminalMock(...args),
  openAgentTerminal: (...args: unknown[]) => openAgentTerminalMock(...args),
  resizeAgentTerminal: (...args: unknown[]) => resizeAgentTerminalMock(...args),
  restartAgentTerminal: (...args: unknown[]) => restartAgentTerminalMock(...args),
  writeAgentTerminal: (...args: unknown[]) => writeAgentTerminalMock(...args),
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 100;
    rows = 24;
    loadAddon = vi.fn();
    open = terminalOpenMock;
    write = vi.fn();
    reset = vi.fn();
    clear = vi.fn();
    focus = vi.fn();
    dispose = vi.fn();
    onData = vi.fn(() => ({ dispose: vi.fn() }));
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = vi.fn();
  },
}));

const workspace = (
  overrides: Partial<AgentConversationWorkspace> = {},
): AgentConversationWorkspace => ({
  conversationId: "conversation-1",
  projectId: "project-1",
  mode: "edit",
  baseRefKind: "current_branch",
  baseRef: "feature/agent-screen",
  baseDisplayName: "Current branch (feature/agent-screen)",
  baseCommit: "base-sha",
  branchName: "ralphx/ralphx/agent-746b9fa7",
  worktreePath: "/tmp/ralphx/agent-conversation",
  linkedIdeationSessionId: null,
  linkedPlanBranchId: null,
  publicationPrNumber: null,
  publicationPrUrl: null,
  publicationPrStatus: null,
  publicationPushStatus: null,
  status: "active",
  createdAt: "2026-04-26T00:00:00.000Z",
  updatedAt: "2026-04-26T00:00:00.000Z",
  ...overrides,
});

describe("AgentTerminalDrawer", () => {
  let rafCallbacks: FrameRequestCallback[];

  beforeEach(() => {
    vi.useFakeTimers();
    rafCallbacks = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      rafCallbacks.push(callback);
      return rafCallbacks.length;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation((handle) => {
      rafCallbacks[handle - 1] = () => undefined;
    });
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe = vi.fn();
        disconnect = vi.fn();
      },
    );

    listenMock.mockReset();
    openAgentTerminalMock.mockReset();
    closeAgentTerminalMock.mockReset();
    clearAgentTerminalMock.mockReset();
    resizeAgentTerminalMock.mockReset();
    restartAgentTerminalMock.mockReset();
    writeAgentTerminalMock.mockReset();
    terminalOpenMock.mockReset();

    listenMock.mockResolvedValue(vi.fn());
    openAgentTerminalMock.mockResolvedValue({
      status: "running",
      cwd: "/tmp/ralphx/agent-conversation",
      workspaceBranch: "ralphx/ralphx/agent-746b9fa7",
      history: "",
      updatedAt: "2026-04-26T00:00:00.000Z",
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("paints the drawer shell before starting xterm hydration", async () => {
    const dockElement = document.createElement("div");
    document.body.appendChild(dockElement);

    render(
      <TooltipProvider>
        <AgentTerminalDrawer
          conversationId="conversation-1"
          workspace={workspace()}
          height={220}
          onHeightChange={vi.fn()}
          onClose={vi.fn()}
          placement="auto"
          onPlacementChange={vi.fn()}
          dockElement={dockElement}
        />
      </TooltipProvider>,
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.getByTestId("agent-terminal-drawer")).toBeInTheDocument();
    expect(screen.getByText("Starting terminal...")).toBeInTheDocument();
    expect(rafCallbacks).toHaveLength(1);
    expect(terminalOpenMock).not.toHaveBeenCalled();
    expect(openAgentTerminalMock).not.toHaveBeenCalled();

    await act(async () => {
      rafCallbacks[0]?.(0);
      await vi.dynamicImportSettled();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(terminalOpenMock).not.toHaveBeenCalled();
    expect(openAgentTerminalMock).not.toHaveBeenCalled();

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await vi.dynamicImportSettled();
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(terminalOpenMock).toHaveBeenCalled();
    expect(openAgentTerminalMock).toHaveBeenCalledWith(
      expect.objectContaining({
        conversationId: "conversation-1",
        terminalId: "default",
      }),
    );
  });
});
