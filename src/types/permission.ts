import { z } from "zod";

/**
 * Permission request sent from the backend when an agent attempts
 * to use a tool that requires user approval.
 */
export interface PermissionRequest {
  request_id: string;
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string;
}

/**
 * User's decision on a permission request.
 */
export type PermissionDecision = "allow" | "deny";

/**
 * Zod schema for validating permission request payloads from Tauri events.
 */
export const PermissionRequestSchema = z.object({
  request_id: z.string(),
  tool_name: z.string(),
  tool_input: z.record(z.string(), z.unknown()),
  context: z.string().optional(),
});

/**
 * Zod schema for validating permission decisions.
 */
export const PermissionDecisionSchema = z.union([
  z.literal("allow"),
  z.literal("deny"),
]);

/**
 * Type guard for PermissionRequest validation.
 */
export function isPermissionRequest(value: unknown): value is PermissionRequest {
  return PermissionRequestSchema.safeParse(value).success;
}
