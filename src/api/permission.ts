/**
 * Permission API Module
 *
 * Provides a centralized API wrapper for resolving permission requests.
 * This module follows the domain API pattern used by other centralized modules.
 */

import { invoke } from "@tauri-apps/api/core";

// ============================================================================
// Types
// ============================================================================

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
} as const;
