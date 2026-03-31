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
  agent_type?: string;
  task_id?: string;
  context_type?: string;
  context_id?: string;
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
  agent_type: z.string().optional(),
  task_id: z.string().optional(),
  context_type: z.string().optional(),
  context_id: z.string().optional(),
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

/**
 * Event emitted when a permission request has expired on the backend.
 * The frontend should dismiss the corresponding dialog.
 */
export interface PermissionExpiredEvent {
  request_id: string;
}
