/**
 * Permission API Module
 *
 * Provides a centralized API wrapper for resolving permission requests.
 * This module follows the domain API pattern used by other centralized modules.
 */

import { invoke } from "@tauri-apps/api/core";
import type { PermissionRequest } from "@/types/permission";

// ============================================================================
// Types
// ============================================================================

/** Raw shape returned by the backend get_pending_permissions command (snake_case) */
interface PendingPermissionInfoRaw {
  request_id: string;
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string | null;
  agent_type?: string | null;
  task_id?: string | null;
  context_type?: string | null;
  context_id?: string | null;
}

export interface ResolvePermissionInput {
  requestId: string;
  decision: "allow" | "deny";
  message?: string;
}

// ============================================================================
// Permission API Object
// ============================================================================

/**
 * Permission API object containing typed Tauri command wrappers
 */
export const permissionApi = {
  /**
   * Resolve a permission request from an agent
   * @param input The resolution details including request ID and decision
   */
  resolveRequest: async (input: ResolvePermissionInput): Promise<void> => {
    await invoke("resolve_permission_request", {
      args: {
        request_id: input.requestId,
        decision: input.decision,
        message: input.message,
      },
    });
    // Command returns () on success, no parsing needed
  },

  /**
   * Fetch all currently pending permission requests from the backend in-memory state.
   * Used to hydrate the UI for requests whose Tauri events were missed
   * (e.g., because the permission dialog wasn't mounted when the event fired).
   */
  getPendingPermissions: async (): Promise<PermissionRequest[]> => {
    const raw = await invoke<PendingPermissionInfoRaw[]>("get_pending_permissions");
    return raw.map((item) => ({
      request_id: item.request_id,
      tool_name: item.tool_name,
      tool_input: item.tool_input,
      ...(item.context != null && { context: item.context }),
      ...(item.agent_type != null && { agent_type: item.agent_type }),
      ...(item.task_id != null && { task_id: item.task_id }),
      ...(item.context_type != null && { context_type: item.context_type }),
      ...(item.context_id != null && { context_id: item.context_id }),
    }));
  },
} as const;
