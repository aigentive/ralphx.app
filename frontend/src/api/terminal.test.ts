import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  clearAgentTerminal,
  closeAgentTerminal,
  DEFAULT_AGENT_TERMINAL_ID,
  openAgentTerminal,
  resizeAgentTerminal,
  restartAgentTerminal,
  writeAgentTerminal,
} from "./terminal";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

const snapshot = {
  conversationId: "conversation-1",
  terminalId: DEFAULT_AGENT_TERMINAL_ID,
  cwd: "/tmp/ralphx/worktrees/conversation-1",
  workspaceBranch: "ralphx/project/agent-12345678",
  status: "running",
  pid: 1234,
  history: "ready\r\n",
  exitCode: null,
  exitSignal: null,
  updatedAt: "2026-04-25T08:00:00.000Z",
};

describe("terminal api", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("opens the default conversation terminal", async () => {
    mockInvoke.mockResolvedValue(snapshot);

    const result = await openAgentTerminal({
      conversationId: "conversation-1",
      cols: 120,
      rows: 30,
    });

    expect(result).toEqual(snapshot);
    expect(mockInvoke).toHaveBeenCalledWith("open_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: DEFAULT_AGENT_TERMINAL_ID,
        cols: 120,
        rows: 30,
      },
    });
  });

  it("writes data without accepting a cwd", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await writeAgentTerminal({
      conversationId: "conversation-1",
      data: "pwd\r",
    });

    expect(mockInvoke).toHaveBeenCalledWith("write_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: DEFAULT_AGENT_TERMINAL_ID,
        data: "pwd\r",
      },
    });
  });

  it("wraps resize, clear, restart, and close commands with terminal ids", async () => {
    mockInvoke.mockResolvedValue(snapshot);

    await resizeAgentTerminal({
      conversationId: "conversation-1",
      terminalId: "shell",
      cols: 90,
      rows: 24,
    });
    await clearAgentTerminal({
      conversationId: "conversation-1",
      terminalId: "shell",
      deleteHistory: true,
    });
    await restartAgentTerminal({
      conversationId: "conversation-1",
      terminalId: "shell",
      cols: 90,
      rows: 24,
    });
    await closeAgentTerminal({
      conversationId: "conversation-1",
      terminalId: "shell",
    });

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "resize_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: "shell",
        cols: 90,
        rows: 24,
      },
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "clear_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: "shell",
        deleteHistory: true,
      },
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "restart_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: "shell",
        cols: 90,
        rows: 24,
      },
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(4, "close_agent_terminal", {
      input: {
        conversationId: "conversation-1",
        terminalId: "shell",
        deleteHistory: false,
      },
    });
  });
});
