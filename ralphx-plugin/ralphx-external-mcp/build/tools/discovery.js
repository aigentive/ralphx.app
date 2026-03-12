/**
 * Discovery tool handlers — Flow 1 (Phase 4)
 *
 * These tools expose project/pipeline read-only data to external API key holders.
 * All backend calls go to :3847; the BackendClient injects the project scope header.
 */
import { getBackendClient, BackendError } from "../backend-client.js";
/**
 * v1_list_projects — list projects accessible to this API key.
 * GET /api/external/projects
 * Backend filters by X-RalphX-Project-Scope header.
 */
export async function handleListProjects(_args, context) {
    try {
        const response = await getBackendClient().get("/api/external/projects", context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        if (err instanceof BackendError) {
            return JSON.stringify({ error: "backend_error", status: err.statusCode, message: err.message }, null, 2);
        }
        return JSON.stringify({ error: "unexpected_error", message: String(err) }, null, 2);
    }
}
/**
 * v1_get_project_status — get project details, task counts, and running agent status.
 * GET /api/external/project/:project_id/status
 */
export async function handleGetProjectStatus(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/project/${encodeURIComponent(projectId)}/status`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        if (err instanceof BackendError) {
            return JSON.stringify({ error: "backend_error", status: err.statusCode, message: err.message }, null, 2);
        }
        return JSON.stringify({ error: "unexpected_error", message: String(err) }, null, 2);
    }
}
/**
 * v1_get_pipeline_overview — get tasks grouped by pipeline stage with counts.
 * GET /api/external/pipeline/:project_id
 */
export async function handleGetPipelineOverview(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().get(`/api/external/pipeline/${encodeURIComponent(projectId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        if (err instanceof BackendError) {
            return JSON.stringify({ error: "backend_error", status: err.statusCode, message: err.message }, null, 2);
        }
        return JSON.stringify({ error: "unexpected_error", message: String(err) }, null, 2);
    }
}
//# sourceMappingURL=discovery.js.map