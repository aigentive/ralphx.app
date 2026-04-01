/**
 * Project setup tools for ralphx-external-mcp
 *
 * Handles project registration via v1_register_project.
 * Requires CREATE_PROJECT permission (bit 8).
 */

import type { ApiKeyContext } from "../types.js";
import { Permission, hasPermission } from "../types.js";
import { getBackendClient } from "../backend-client.js";
import { invalidateCacheByKeyId } from "../auth.js";

export interface RegisterProjectArgs {
  working_directory: string;
  name?: string;
}

export interface ToolResult {
  text: string;
  isError: boolean;
}

/**
 * Register a folder as a RalphX project.
 * Creates directory if needed, initializes git if needed.
 * Requires CREATE_PROJECT permission.
 */
export async function handleRegisterProject(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<ToolResult> {
  if (!hasPermission(context.permissions, Permission.CREATE_PROJECT)) {
    return {
      text: JSON.stringify({
        error: "permission_denied",
        message: "CREATE_PROJECT permission required",
      }),
      isError: true,
    };
  }

  const { working_directory, name } = args as unknown as RegisterProjectArgs;

  if (!working_directory) {
    return {
      text: JSON.stringify({
        error: "missing_argument",
        message: "working_directory is required",
      }),
      isError: true,
    };
  }

  const backendClient = getBackendClient();
  const result = await backendClient.post<{ id: string }>(
    "/api/external/projects",
    context,
    { working_directory, name }
  );

  if (result.status >= 400) {
    return {
      text: JSON.stringify({
        error: "backend_error",
        status: result.status,
        body: result.body,
      }),
      isError: true,
    };
  }

  if (result.body && (result.body as { id?: string }).id && context.keyId) {
    // Invalidate scope cache so next request picks up the new project
    invalidateCacheByKeyId(context.keyId);
  }

  return { text: JSON.stringify(result.body), isError: false };
}
