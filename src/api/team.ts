/**
 * Tauri invoke wrappers for team management commands
 *
 * Provides typed API functions for team lifecycle, messaging, and status queries.
 * Uses Zod schemas for response validation following the project API pattern.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Schemas
// ============================================================================

export const TeammateStatusSchema = z.object({
  name: z.string(),
  color: z.string(),
  model: z.string(),
  role_description: z.string(),
  status: z.string(),
  current_activity: z.string().nullable(),
  tokens_used: z.number(),
  estimated_cost_usd: z.number(),
});

export const TeamMessageSchema = z.object({
  id: z.string(),
  from: z.string(),
  to: z.string(),
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
  contextType: ContextType,
  contextId: string,
): Promise<TeamStatusResponse | null> {
  const result = await invoke("get_team_status", { contextType, contextId });
  return result ? TeamStatusSchema.parse(result) : null;
}

export async function sendTeamMessage(
  contextType: ContextType,
  contextId: string,
  target: string,
  content: string,
): Promise<void> {
  await invoke("send_team_message", { contextType, contextId, target, content });
}

export async function stopTeammate(
  contextType: ContextType,
  contextId: string,
  teammateName: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_teammate", { contextType, contextId, teammateName })
  );
}

export async function stopTeam(
  contextType: ContextType,
  contextId: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_team", { contextType, contextId })
  );
}

export async function getTeamMessages(
  contextType: ContextType,
  contextId: string,
  since?: string,
): Promise<TeamMessageResponse[]> {
  return z.array(TeamMessageSchema).parse(
    await invoke("get_team_messages", {
      contextType,
      contextId,
      ...(since !== undefined && { since }),
    })
  );
}
