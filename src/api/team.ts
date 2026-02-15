/**
 * Tauri invoke wrappers for team management commands
 *
 * Provides typed API functions for team lifecycle, messaging, and status queries.
 * Uses Zod schemas for response validation following the project API pattern.
 *
 * All commands accept `team_name` to identify the team (matches backend expectation).
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Schemas
// ============================================================================

export const TeammateStatusSchema = z.object({
  name: z.string(),
  color: z.string(),
  model: z.string(),
  role: z.string(),
  status: z.string(),
  current_activity: z.string().nullable(),
  tokens_used: z.number(),
  estimated_cost_usd: z.number(),
});

export const TeamMessageSchema = z.object({
  id: z.string(),
  sender: z.string(),
  recipient: z.string(),
  content: z.string(),
  timestamp: z.string(),
});

export const TeamStatusSchema = z.object({
  team_name: z.string(),
  context_type: z.string(),
  context_id: z.string(),
  lead_name: z.string(),
  teammates: z.array(TeammateStatusSchema),
  messages: z.array(TeamMessageSchema),
  total_tokens: z.number(),
  estimated_cost_usd: z.number(),
  created_at: z.string(),
});

// ============================================================================
// Types
// ============================================================================

export type TeamStatusResponse = z.infer<typeof TeamStatusSchema>;
export type TeammateStatusResponse = z.infer<typeof TeammateStatusSchema>;
export type TeamMessageResponse = z.infer<typeof TeamMessageSchema>;

// ============================================================================
// API Functions
// ============================================================================

export async function getTeamStatus(
  teamName: string,
): Promise<TeamStatusResponse | null> {
  const result = await invoke("get_team_status", { team_name: teamName });
  return result ? TeamStatusSchema.parse(result) : null;
}

export async function sendTeamMessage(
  teamName: string,
  target: string,
  content: string,
): Promise<void> {
  await invoke("send_team_message", { team_name: teamName, content, target });
}

export async function stopTeammate(
  teamName: string,
  teammateName: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_teammate", { team_name: teamName, teammate_name: teammateName }),
  );
}

export async function stopTeam(
  teamName: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_team", { team_name: teamName }),
  );
}

export async function getTeamMessages(
  teamName: string,
  since?: string,
): Promise<TeamMessageResponse[]> {
  return z.array(TeamMessageSchema).parse(
    await invoke("get_team_messages", {
      team_name: teamName,
      ...(since !== undefined && { since }),
    }),
  );
}
