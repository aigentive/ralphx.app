/**
 * Pipeline supervision tool handlers — Flow 3 (Phase 5)
 *
 * 11 tools for monitoring and controlling task pipeline execution.
 * Read-only tools proxy to new backend endpoints.
 * State transition tools proxy through task_transition or review_action endpoints.
 */
import { getBackendClient, BackendError } from "../backend-client.js";
import { resumeScheduling } from "../composites/resume-scheduling.js";
function handleError(err) {
    if (err instanceof BackendError) {
        return JSON.stringify({ error: "backend_error", status: err.statusCode, message: err.message }, null, 2);
    }
    return JSON.stringify({ error: "unexpected_error", message: String(err) }, null, 2);
}
/**
 * v1_get_task_detail — get full task details + steps.
 * GET /api/external/task/:id
 */
export async function handleGetTaskDetail(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/task/${encodeURIComponent(taskId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_get_task_diff — get git diff stats for a task branch.
 * GET /api/external/task/:id/diff
 */
export async function handleGetTaskDiff(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/task/${encodeURIComponent(taskId)}/diff`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_get_review_summary — get review notes and findings for a task.
 * GET /api/external/task/:id/review_summary
 */
export async function handleGetReviewSummary(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/task/${encodeURIComponent(taskId)}/review_summary`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_approve_review — approve a review, moving it to merge.
 * POST /api/external/review_action
 */
export async function handleApproveReview(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/review_action", context, { task_id: taskId, action: "approve_review" });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_request_changes — request changes on a task review with feedback.
 * POST /api/external/review_action
 */
export async function handleRequestChanges(args, context) {
    const taskId = args.task_id;
    const feedback = args.feedback;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    if (!feedback) {
        return JSON.stringify({ error: "missing_argument", message: "feedback is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/review_action", context, { task_id: taskId, action: "request_changes", feedback });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_get_merge_pipeline — get all merge activity for a project.
 * GET /api/external/merge_pipeline/:project_id
 */
export async function handleGetMergePipeline(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/merge_pipeline/${encodeURIComponent(projectId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_resolve_escalation — handle an escalated review.
 * POST /api/external/review_action
 */
export async function handleResolveEscalation(args, context) {
    const taskId = args.task_id;
    const resolution = args.resolution;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    if (!resolution) {
        return JSON.stringify({ error: "missing_argument", message: "resolution is required" }, null, 2);
    }
    const validResolutions = ["approve", "request_changes", "cancel"];
    if (!validResolutions.includes(resolution)) {
        return JSON.stringify({
            error: "invalid_argument",
            message: `resolution must be one of: ${validResolutions.join(", ")}`,
        }, null, 2);
    }
    const feedback = args.feedback;
    try {
        const response = await getBackendClient().post("/api/external/review_action", context, { task_id: taskId, action: "resolve_escalation", resolution, feedback });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_pause_task — pause a running task.
 * POST /api/external/task_transition
 */
export async function handlePauseTask(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/task_transition", context, { task_id: taskId, action: "pause" });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_cancel_task — cancel a task.
 * POST /api/external/task_transition
 */
export async function handleCancelTask(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/task_transition", context, { task_id: taskId, action: "cancel" });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_retry_task — retry a failed or stopped task.
 * POST /api/external/task_transition
 */
export async function handleRetryTask(args, context) {
    const taskId = args.task_id;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/task_transition", context, { task_id: taskId, action: "retry" });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_resume_scheduling — resume a failed accept_plan_and_schedule from its last step.
 * Delegates to resumeScheduling composite.
 */
export async function handleResumeScheduling(args, context) {
    const sessionId = args.session_id;
    if (!sessionId) {
        return JSON.stringify({ error: "missing_argument", message: "session_id is required" }, null, 2);
    }
    try {
        const result = await resumeScheduling({ sessionId }, context);
        return JSON.stringify(result, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_create_task_note — annotate a task with a progress note.
 * POST /api/external/task-note
 */
export async function handleCreateTaskNote(args, context) {
    const taskId = args.task_id;
    const note = args.note;
    if (!taskId) {
        return JSON.stringify({ error: "missing_argument", message: "task_id is required" }, null, 2);
    }
    if (!note) {
        return JSON.stringify({ error: "missing_argument", message: "note is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().post("/api/external/task-note", context, { task_id: taskId, note });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
//# sourceMappingURL=pipeline.js.map