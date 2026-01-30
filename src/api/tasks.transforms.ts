// Transform functions for converting snake_case tasks API responses to camelCase frontend types

import { z } from "zod";
import { InjectTaskResponseSchemaRaw } from "./tasks.schemas";
import { transformTask, type Task } from "@/types/task";

/**
 * Frontend InjectTaskResponse type (camelCase)
 */
export interface InjectTaskResponse {
  task: Task;
  target: "backlog" | "planned";
  priority: number;
  makeNextApplied: boolean;
}

/**
 * Transform InjectTaskResponseSchemaRaw to InjectTaskResponse
 */
export function transformInjectTaskResponse(
  raw: z.infer<typeof InjectTaskResponseSchemaRaw>
): InjectTaskResponse {
  return {
    task: transformTask(raw.task),
    target: raw.target,
    priority: raw.priority,
    makeNextApplied: raw.make_next_applied,
  };
}
