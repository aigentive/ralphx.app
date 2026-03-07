/**
 * Event types for ralphx-external-mcp event bridge
 */
export interface ExternalEvent {
    id: number;
    event_type: string;
    project_id: string;
    payload: unknown;
    created_at: string;
}
export interface EventCursor {
    last_id: number;
}
/** Event types emitted by RalphX backend */
export type EventType = "task_status_changed" | "task_created" | "task_updated" | "ideation_session_started" | "ideation_session_ended" | "review_completed" | "merge_completed" | "execution_started" | "execution_completed";
//# sourceMappingURL=types.d.ts.map