/**
 * Composite: accept_plan_and_schedule
 *
 * Saga pattern — idempotent steps, resumable on failure.
 * Phase 4 implementation.
 *
 * Steps:
 * 1. Load ideation session + proposals
 * 2. Apply proposals (create task proposals in DB)
 * 3. Create tasks from accepted proposals
 * 4. Schedule tasks (set to pending, validate dependencies)
 *
 * On partial failure: returns progress report. Each step is idempotent.
 * Resume via v1_resume_scheduling tool.
 */
import { getBackendClient, BackendError } from "../backend-client.js";
/**
 * Accept all proposals for a session and schedule the resulting tasks.
 * Each step is recorded so the saga can be resumed on failure.
 */
export async function acceptAndSchedule(input, context) {
    const progress = {
        step: "load_session",
        completed: [],
    };
    // Step 1: load proposals
    let proposalIds;
    try {
        const listResp = await getBackendClient().get(`/api/list_session_proposals/${encodeURIComponent(input.sessionId)}`, context);
        const proposals = listResp.body.proposals ?? [];
        proposalIds = proposals.map((p) => p.id);
        progress.completed.push("load_session");
        progress.step = "apply_proposals";
    }
    catch (err) {
        progress.failed = {
            step: "load_session",
            error: err instanceof Error ? err.message : String(err),
        };
        return { success: false, taskIds: [], progress };
    }
    if (proposalIds.length === 0) {
        // Nothing to apply — treat as success
        return {
            success: true,
            taskIds: [],
            progress: {
                step: "schedule_tasks",
                completed: ["load_session", "apply_proposals", "create_tasks", "schedule_tasks"],
            },
        };
    }
    // Step 2: apply proposals (POST /api/external/apply_proposals)
    let taskIds = [];
    try {
        const applyResp = await getBackendClient().post("/api/external/apply_proposals", context, {
            session_id: input.sessionId,
            proposal_ids: proposalIds,
        });
        if (applyResp.status < 200 || applyResp.status >= 300) {
            throw new BackendError(applyResp.status, `apply_proposals returned HTTP ${applyResp.status}`);
        }
        taskIds = applyResp.body.task_ids ?? [];
        progress.completed.push("apply_proposals");
        progress.step = "create_tasks";
    }
    catch (err) {
        progress.failed = {
            step: "apply_proposals",
            error: err instanceof Error ? err.message : String(err),
        };
        return { success: false, taskIds: [], progress };
    }
    // Step 3 + 4: create_tasks and schedule_tasks are handled server-side in apply_proposals.
    // Mark them as complete since the backend handles task creation + scheduling atomically.
    progress.completed.push("create_tasks");
    progress.completed.push("schedule_tasks");
    progress.step = "schedule_tasks";
    return { success: true, taskIds, progress };
}
//# sourceMappingURL=accept-and-schedule.js.map