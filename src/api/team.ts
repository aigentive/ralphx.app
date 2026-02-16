/**
 * Tauri invoke wrappers for team management commands
 *
 * Provides typed API functions for team lifecycle, messaging, and status queries.
 * Uses Zod schemas for response validation following the project API pattern.
 *
 * Param naming: Tauri 2.0 auto-renames snake_case Rust params to camelCase.
 * Struct params (e.g. SendTeamMessageInput) are wrapped in the param name.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Schemas (match Rust TeamStateTracker response structs)
// ============================================================================

const TeammateCostSchema = z.object({
  input_tokens: z.number(),
  output_tokens: z.number(),
  cache_creation_tokens: z.number(),
  cache_read_tokens: z.number(),
  estimated_usd: z.number(),
});

export const TeammateStatusSchema = z.object({
  name: z.string(),
  color: z.string(),
  model: z.string(),
  role: z.string(),
  status: z.string(),
  cost: TeammateCostSchema,
  spawned_at: z.string(),
  last_activity_at: z.string(),
});

export const TeamMessageSchema = z.object({
  id: z.string(),
  sender: z.string(),
  recipient: z.string().nullable(),
  content: z.string(),
  message_type: z.string(),
  timestamp: z.string(),
});

export const TeamStatusSchema = z.object({
  name: z.string(),
  context_type: z.string(),
  context_id: z.string(),
  lead_name: z.string().nullable(),
  teammates: z.array(TeammateStatusSchema),
  phase: z.string(),
  created_at: z.string(),
  message_count: z.number(),
});

// ============================================================================
// Types
// ============================================================================

export type TeamStatusResponse = z.infer<typeof TeamStatusSchema>;
export type TeammateStatusResponse = z.infer<typeof TeammateStatusSchema>;
export type TeamMessageResponse = z.infer<typeof TeamMessageSchema>;

// ============================================================================
// History Schemas (for hydrating past team sessions)
// ============================================================================

export const TeammateSnapshotSchema = z.object({
  name: z.string(),
  color: z.string(),
  model: z.string(),
  role: z.string(),
  status: z.string(),
  cost: TeammateCostSchema,
  spawned_at: z.string(),
  last_activity_at: z.string(),
});

export const TeamSessionHistorySchema = z.object({
  team_name: z.string(),
  lead_name: z.string(),
  context_type: z.string(),
  context_id: z.string(),
  phase: z.string(),
  created_at: z.string(),
  ended_at: z.string().nullable(),
  teammates: z.array(TeammateSnapshotSchema),
  total_tokens: z.number(),
  total_estimated_cost_usd: z.number(),
});

export const TeamHistoryResponseSchema = z.object({
  session: TeamSessionHistorySchema.nullable(),
  messages: z.array(TeamMessageSchema),
});

export type TeammateSnapshot = z.infer<typeof TeammateSnapshotSchema>;
export type TeamSessionHistory = z.infer<typeof TeamSessionHistorySchema>;
export type TeamHistoryResponse = z.infer<typeof TeamHistoryResponseSchema>;

// ============================================================================
// API Functions
// ============================================================================

// ============================================================================
// Plan Approval (calls HTTP server directly — the approve handler has the
// spawning infrastructure that Tauri commands don't have access to)
// ============================================================================

const MCP_SERVER_URL = "http://127.0.0.1:3847";

export const ApproveTeamPlanResponseSchema = z.object({
  success: z.boolean(),
  team_name: z.string(),
  teammates_spawned: z.array(z.object({
    name: z.string(),
    role: z.string(),
    model: z.string(),
    color: z.string(),
  })),
  message: z.string(),
});

export type ApproveTeamPlanResponse = z.infer<typeof ApproveTeamPlanResponseSchema>;

export async function approveTeamPlan(
  planId: string,
  contextType: string,
  contextId: string,
): Promise<ApproveTeamPlanResponse> {
  const resp = await fetch(`${MCP_SERVER_URL}/api/team/plan/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      plan_id: planId,
      context_type: contextType,
      context_id: contextId,
    }),
  });

  if (!resp.ok) {
    const errorText = await resp.text();
    throw new Error(`Failed to approve team plan: ${errorText}`);
  }

  return ApproveTeamPlanResponseSchema.parse(await resp.json());
}

export async function rejectTeamPlan(planId: string): Promise<void> {
  const resp = await fetch(`${MCP_SERVER_URL}/api/team/plan/reject`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ plan_id: planId }),
  });

  if (!resp.ok) {
    const errorText = await resp.text();
    throw new Error(`Failed to reject team plan: ${errorText}`);
  }
}

// ============================================================================
// API Functions (Tauri IPC)
// ============================================================================

export async function getTeamStatus(
  teamName: string,
): Promise<TeamStatusResponse | null> {
  const result = await invoke("get_team_status", { teamName });
  return result ? TeamStatusSchema.parse(result) : null;
}

export async function sendTeamMessage(
  teamName: string,
  target: string,
  content: string,
): Promise<TeamMessageResponse> {
  const result = await invoke("send_team_message", {
    input: { teamName, target, content },
  });
  return TeamMessageSchema.parse(result);
}

export async function sendTeammateMessage(
  teamName: string,
  teammateName: string,
  content: string,
): Promise<void> {
  await invoke("send_teammate_message", {
    input: { teamName, teammateName, content },
  });
}

export async function stopTeammate(
  teamName: string,
  teammateName: string,
): Promise<void> {
  await invoke("stop_teammate", { teamName, teammateName });
}

export async function stopTeam(
  teamName: string,
): Promise<void> {
  await invoke("stop_team", { teamName });
}

export async function getTeamMessages(
  teamName: string,
  limit?: number,
): Promise<TeamMessageResponse[]> {
  return z.array(TeamMessageSchema).parse(
    await invoke("get_team_messages", {
      teamName,
      ...(limit !== undefined && { limit }),
    }),
  );
}

export async function getTeamHistory(
  contextType: string,
  contextId: string,
): Promise<TeamHistoryResponse> {
  const result = await invoke("get_team_history", {
    input: { context_type: contextType, context_id: contextId },
  });
  return TeamHistoryResponseSchema.parse(result);
}

export async function getTeammateCost(
  teamName: string,
  teammateName: string,
): Promise<{ teammate_name: string; input_tokens: number; output_tokens: number; cache_creation_tokens: number; cache_read_tokens: number; estimated_usd: number }> {
  return z.object({
    teammate_name: z.string(),
    input_tokens: z.number(),
    output_tokens: z.number(),
    cache_creation_tokens: z.number(),
    cache_read_tokens: z.number(),
    estimated_usd: z.number(),
  }).parse(
    await invoke("get_teammate_cost", { teamName, teammateName }),
  );
}
