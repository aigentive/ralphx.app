/**
 * Event name constants for Tauri events
 *
 * Use these constants instead of hardcoding event strings to:
 * - Prevent typos
 * - Enable IDE autocomplete
 * - Maintain consistency with backend event names
 */

// Unified events (new API - includes context_type in payload)
export const AGENT_CHUNK = "agent:chunk";
export const AGENT_TOOL_CALL = "agent:tool_call";
export const AGENT_RUN_STARTED = "agent:run_started";
export const AGENT_RUN_COMPLETED = "agent:run_completed";
export const AGENT_MESSAGE_CREATED = "agent:message_created";
export const AGENT_ERROR = "agent:error";
export const AGENT_QUEUE_SENT = "agent:queue_sent";
export const AGENT_MESSAGE = "agent:message";
export const AGENT_TASK_STARTED = "agent:task_started";
export const AGENT_TASK_COMPLETED = "agent:task_completed";
export const AGENT_HOOK = "agent:hook";
export const AGENT_SESSION_RECOVERED = "agent:session_recovered";

// Team lifecycle events
export const TEAM_CREATED = "team:created";
export const TEAM_TEAMMATE_SPAWNED = "team:teammate_spawned";
export const TEAM_TEAMMATE_IDLE = "team:teammate_idle";
export const TEAM_TEAMMATE_SHUTDOWN = "team:teammate_shutdown";
export const TEAM_MESSAGE = "team:message";
export const TEAM_DISBANDED = "team:disbanded";
export const TEAM_COST_UPDATE = "team:cost_update";
