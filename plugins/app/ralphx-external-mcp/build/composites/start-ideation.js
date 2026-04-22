/**
 * Composite: start_ideation
 *
 * Creates an ideation session and spawns an orchestrator agent.
 * Phase 4 implementation.
 */
import { getBackendClient, BackendError } from "../backend-client.js";
/**
 * Start an ideation session on the backend and return session_id + status.
 * POST /api/external/start_ideation
 */
export async function startIdeation(input, context) {
    const body = {
        project_id: input.projectId,
        prompt: input.prompt,
    };
    if (input.idempotencyKey !== undefined) {
        body.idempotency_key = input.idempotencyKey;
    }
    const response = await getBackendClient().post("/api/external/start_ideation", context, body);
    if (response.status < 200 || response.status >= 300) {
        throw new BackendError(response.status, `Failed to start ideation session: HTTP ${response.status}`);
    }
    const b = response.body;
    if (!b.session_id) {
        throw new Error("Backend returned no session_id for start_ideation");
    }
    const agentSpawned = b.agent_spawned ?? false;
    const blocked = !agentSpawned && !!b.agent_spawn_blocked_reason;
    const result = {
        sessionId: b.session_id,
        status: blocked ? "blocked" : "started",
        agentSpawned,
        ...(b.agent_spawn_blocked_reason !== undefined
            ? { agentSpawnBlockedReason: b.agent_spawn_blocked_reason }
            : {}),
        ...(b.pending_initial_prompt !== undefined
            ? { pendingInitialPrompt: b.pending_initial_prompt }
            : {}),
        ...(b.existing_active_sessions !== undefined
            ? {
                existingActiveSessions: b.existing_active_sessions.map((s) => ({
                    sessionId: s.session_id,
                    ...(s.title !== undefined ? { title: s.title } : {}),
                    status: s.status,
                    createdAt: s.created_at,
                    ...(s.external_activity_phase !== undefined
                        ? { externalActivityPhase: s.external_activity_phase }
                        : {}),
                })),
            }
            : {}),
        ...(b.exists !== undefined ? { exists: b.exists } : {}),
        ...(b.duplicate_detected !== undefined ? { duplicateDetected: b.duplicate_detected } : {}),
        ...(b.similarity_score !== undefined ? { similarityScore: b.similarity_score } : {}),
        ...(b.next_action !== undefined ? { nextAction: b.next_action } : {}),
        ...(b.hint !== undefined ? { hint: b.hint } : {}),
    };
    return result;
}
//# sourceMappingURL=start-ideation.js.map