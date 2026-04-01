/**
 * Task tool handlers for external MCP.
 *
 * v1_get_task_steps: list all steps for a task.
 * v1_batch_task_status: batch lookup up to 50 task IDs.
 */
import type { ApiKeyContext } from "../types.js";
/**
 * v1_get_task_steps — list all steps for a task.
 * GET /api/task_steps/:task_id
 */
export declare function handleGetTaskSteps(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_batch_task_status — batch lookup up to 50 task IDs.
 * POST /api/external/tasks/batch_status
 */
export declare function handleBatchTaskStatus(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
//# sourceMappingURL=tasks.d.ts.map