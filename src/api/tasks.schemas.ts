// Zod schemas for tasks API responses (snake_case from Rust backend)

import { z } from "zod";
import { TaskSchema } from "@/types/task";

/**
 * Inject task response schema from Rust (snake_case)
 * Backend outputs snake_case (Rust default). Transform layer converts to camelCase for UI.
 */
export const InjectTaskResponseSchemaRaw = z.object({
  task: TaskSchema,
  target: z.enum(["backlog", "planned"]),
  priority: z.number().int(),
  make_next_applied: z.boolean(),
});
