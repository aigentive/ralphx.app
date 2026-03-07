/**
 * Composite: resume_scheduling
 *
 * Resumes a failed v1_accept_plan_and_schedule from its last successful step.
 * Looks up the current state and retries the failed step.
 * Phase 5 implementation.
 */
import type { ApiKeyContext } from "../types.js";
export interface ResumeSchedulingInput {
    sessionId: string;
}
export interface ResumeSchedulingResult {
    success: boolean;
    taskIds: string[];
    message: string;
}
/**
 * Resume a failed accept_plan_and_schedule by:
 * 1. Loading session proposals to determine what's already done
 * 2. Re-calling apply_proposals if not yet completed
 *
 * This is idempotent — apply_proposals is safe to call multiple times.
 */
export declare function resumeScheduling(input: ResumeSchedulingInput, context: ApiKeyContext): Promise<ResumeSchedulingResult>;
//# sourceMappingURL=resume-scheduling.d.ts.map