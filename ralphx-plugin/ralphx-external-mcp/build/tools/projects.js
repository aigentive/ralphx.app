/**
 * Project setup tools for ralphx-external-mcp
 *
 * Handles project registration via v1_register_project.
 * Requires CREATE_PROJECT permission (bit 8).
 */
import { Permission, hasPermission } from "../types.js";
import { getBackendClient } from "../backend-client.js";
import { invalidateCacheByKeyId } from "../auth.js";
/**
 * Register a folder as a RalphX project.
 * Creates directory if needed, initializes git if needed.
 * Requires CREATE_PROJECT permission.
 */
export async function handleRegisterProject(args, context) {
    if (!hasPermission(context.permissions, Permission.CREATE_PROJECT)) {
        return JSON.stringify({
            error: "permission_denied",
            message: "CREATE_PROJECT permission required",
        });
    }
    const { working_directory, name } = args;
    if (!working_directory) {
        return JSON.stringify({
            error: "missing_argument",
            message: "working_directory is required",
        });
    }
    const backendClient = getBackendClient();
    const result = await backendClient.post("/api/external/projects", context, { working_directory, name });
    if (result.body && result.body.id && context.keyId) {
        // Invalidate scope cache so next request picks up the new project
        invalidateCacheByKeyId(context.keyId);
    }
    return JSON.stringify(result.body);
}
//# sourceMappingURL=projects.js.map