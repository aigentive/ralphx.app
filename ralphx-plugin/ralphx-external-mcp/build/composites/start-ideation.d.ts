/**
 * Composite: start_ideation
 *
 * Creates an ideation session and spawns an orchestrator agent.
 * Phase 4 implementation.
 */
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
/**
 * Start an ideation session on the backend and return session_id + status.
 * POST /api/external/start_ideation
 */
export declare function startIdeation(input: StartIdeationInput, context: ApiKeyContext): Promise<StartIdeationResult>;
//# sourceMappingURL=start-ideation.d.ts.map