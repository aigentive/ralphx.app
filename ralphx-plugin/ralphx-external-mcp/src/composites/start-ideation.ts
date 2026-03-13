/**
 * Composite: start_ideation
 *
 * Creates an ideation session and spawns an orchestrator agent.
 * Phase 4 implementation.
 */

import { getBackendClient, BackendError } from "../backend-client.js";
import type { ApiKeyContext } from "../types.js";

export interface StartIdeationInput {
  projectId: string;
  prompt: string;
}

export interface StartIdeationResult {
  sessionId: string;
  status: "started" | "blocked";
  agentSpawned: boolean;
  agentSpawnBlockedReason?: string;
}

interface StartIdeationBackendResponse {
  session_id: string;
  status: string;
  agent_spawned?: boolean;
  agent_spawn_blocked_reason?: string;
}

/**
 * Start an ideation session on the backend and return session_id + status.
 * POST /api/external/start_ideation
 */
export async function startIdeation(
  input: StartIdeationInput,
  context: ApiKeyContext
): Promise<StartIdeationResult> {
  const response = await getBackendClient().post<StartIdeationBackendResponse>(
    "/api/external/start_ideation",
    context,
    {
      project_id: input.projectId,
      prompt: input.prompt,
    }
  );

  if (response.status < 200 || response.status >= 300) {
    throw new BackendError(
      response.status,
      `Failed to start ideation session: HTTP ${response.status}`
    );
  }

  const body = response.body;
  if (!body.session_id) {
    throw new Error("Backend returned no session_id for start_ideation");
  }

  const agentSpawned = body.agent_spawned ?? false;
  const blocked = !agentSpawned && !!body.agent_spawn_blocked_reason;

  return {
    sessionId: body.session_id,
    status: blocked ? "blocked" : "started",
    agentSpawned,
    ...(body.agent_spawn_blocked_reason !== undefined
      ? { agentSpawnBlockedReason: body.agent_spawn_blocked_reason }
      : {}),
  };
}
