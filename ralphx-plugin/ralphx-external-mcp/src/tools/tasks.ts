/**
 * Task tool handlers for external MCP.
 *
 * v1_get_task_steps: list all steps for a task.
 * v1_batch_task_status: batch lookup up to 50 task IDs.
 */

import { getBackendClient, BackendError } from "../backend-client.js";
import type { ApiKeyContext } from "../types.js";

function handleError(err: unknown): string {
  if (err instanceof BackendError) {
    return JSON.stringify(
      { error: "backend_error", status: err.statusCode, message: err.message },
      null,
      2
    );
  }
  return JSON.stringify(
    { error: "unexpected_error", message: String(err) },
    null,
    2
  );
}

/**
 * v1_get_task_steps — list all steps for a task.
 * GET /api/task_steps/:task_id
 */
export async function handleGetTaskSteps(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const taskId = args.task_id as string;
  if (!taskId) {
    return JSON.stringify(
      { error: "missing_argument", message: "task_id is required" },
      null,
      2
    );
  }
  try {
    const response = await getBackendClient().get(
      `/api/task_steps/${encodeURIComponent(taskId)}`,
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}

/**
 * v1_batch_task_status — batch lookup up to 50 task IDs.
 * POST /api/external/tasks/batch_status
 */
export async function handleBatchTaskStatus(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const taskIds = args.task_ids as string[];
  if (!taskIds || !Array.isArray(taskIds) || taskIds.length === 0) {
    return JSON.stringify(
      { error: "missing_argument", message: "task_ids array is required" },
      null,
      2
    );
  }
  try {
    const response = await getBackendClient().post(
      "/api/external/tasks/batch_status",
      context,
      { task_ids: taskIds }
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}
