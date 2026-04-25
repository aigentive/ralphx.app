import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

export const AGENT_TERMINAL_EVENT = "agent_terminal:event";
export const DEFAULT_AGENT_TERMINAL_ID = "default";

const AgentTerminalStatusSchema = z.enum(["running", "exited", "error"]);
export type AgentTerminalStatus = z.infer<typeof AgentTerminalStatusSchema>;

const AgentTerminalSnapshotSchema = z.object({
  conversationId: z.string(),
  terminalId: z.string(),
  cwd: z.string(),
  workspaceBranch: z.string(),
  status: AgentTerminalStatusSchema,
  pid: z.number().nullable().optional(),
  history: z.string(),
  exitCode: z.number().nullable().optional(),
  exitSignal: z.string().nullable().optional(),
  updatedAt: z.string(),
});
export type AgentTerminalSnapshot = z.infer<typeof AgentTerminalSnapshotSchema>;

export const AgentTerminalEventSchema = z.object({
  type: z.enum(["started", "output", "exited", "error", "cleared", "restarted"]),
  conversationId: z.string(),
  terminalId: z.string(),
  cwd: z.string().nullable().optional(),
  workspaceBranch: z.string().nullable().optional(),
  data: z.string().nullable().optional(),
  message: z.string().nullable().optional(),
  pid: z.number().nullable().optional(),
  exitCode: z.number().nullable().optional(),
  exitSignal: z.string().nullable().optional(),
  updatedAt: z.string(),
});
export type AgentTerminalEvent = z.infer<typeof AgentTerminalEventSchema>;

export interface AgentTerminalOpenInput {
  conversationId: string;
  terminalId?: string;
  cols: number;
  rows: number;
}

export interface AgentTerminalWriteInput {
  conversationId: string;
  terminalId?: string;
  data: string;
}

export interface AgentTerminalResizeInput {
  conversationId: string;
  terminalId?: string;
  cols: number;
  rows: number;
}

export interface AgentTerminalCloseInput {
  conversationId: string;
  terminalId?: string;
  deleteHistory?: boolean;
}

async function typedInvoke<T>(
  command: string,
  input: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(command, { input });
  return schema.parse(result);
}

function terminalId(value: string | undefined): string {
  return value?.trim() || DEFAULT_AGENT_TERMINAL_ID;
}

export async function openAgentTerminal(
  input: AgentTerminalOpenInput
): Promise<AgentTerminalSnapshot> {
  return typedInvoke(
    "open_agent_terminal",
    {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      cols: input.cols,
      rows: input.rows,
    },
    AgentTerminalSnapshotSchema
  );
}

export async function writeAgentTerminal(input: AgentTerminalWriteInput): Promise<void> {
  await invoke("write_agent_terminal", {
    input: {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      data: input.data,
    },
  });
}

export async function resizeAgentTerminal(
  input: AgentTerminalResizeInput
): Promise<AgentTerminalSnapshot> {
  return typedInvoke(
    "resize_agent_terminal",
    {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      cols: input.cols,
      rows: input.rows,
    },
    AgentTerminalSnapshotSchema
  );
}

export async function clearAgentTerminal(
  input: AgentTerminalCloseInput
): Promise<AgentTerminalSnapshot> {
  return typedInvoke(
    "clear_agent_terminal",
    {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      deleteHistory: input.deleteHistory ?? false,
    },
    AgentTerminalSnapshotSchema
  );
}

export async function restartAgentTerminal(
  input: AgentTerminalOpenInput
): Promise<AgentTerminalSnapshot> {
  return typedInvoke(
    "restart_agent_terminal",
    {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      cols: input.cols,
      rows: input.rows,
    },
    AgentTerminalSnapshotSchema
  );
}

export async function closeAgentTerminal(input: AgentTerminalCloseInput): Promise<void> {
  await invoke("close_agent_terminal", {
    input: {
      conversationId: input.conversationId,
      terminalId: terminalId(input.terminalId),
      deleteHistory: input.deleteHistory ?? false,
    },
  });
}
