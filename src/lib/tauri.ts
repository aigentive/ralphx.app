// Tauri invoke wrappers with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

/**
 * Generic invoke wrapper with runtime Zod validation
 * @param cmd The Tauri command name
 * @param args The arguments to pass to the command
 * @param schema The Zod schema to validate the response
 * @returns The validated response
 * @throws If the response doesn't match the schema
 */
export async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

/**
 * Health check response schema
 */
export const HealthResponseSchema = z.object({
  status: z.string(),
});

export type HealthResponse = z.infer<typeof HealthResponseSchema>;

/**
 * API object containing all typed Tauri command wrappers
 */
export const api = {
  health: {
    /**
     * Check if the backend is running
     * @returns { status: "ok" } if healthy
     */
    check: () => typedInvoke("health_check", {}, HealthResponseSchema),
  },
} as const;
