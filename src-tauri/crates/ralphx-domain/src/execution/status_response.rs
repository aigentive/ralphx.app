use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutionStatusResponse {
    pub is_paused: bool,
    pub halt_mode: String,
    pub running_count: u32,
    pub max_concurrent: u32,
    pub global_max_concurrent: u32,
    pub queued_count: u32,
    pub queued_message_count: u32,
    pub can_start_task: bool,
    pub provider_blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_blocked_until: Option<u64>,
    pub ideation_active: u32,
    pub ideation_idle: u32,
    pub ideation_waiting: u32,
    pub ideation_max_project: u32,
    pub ideation_max_global: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutionCommandResponse {
    pub success: bool,
    pub status: ExecutionStatusResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStatusInput {
    pub is_paused: bool,
    pub halt_mode: String,
    pub running_count: u32,
    pub max_concurrent: u32,
    pub global_max_concurrent: u32,
    pub queued_count: u32,
    pub queued_message_count: u32,
    pub provider_blocked: bool,
    pub provider_blocked_until_epoch: u64,
    pub total_project_active: u32,
    pub global_running_count: u32,
    pub ideation_active: u32,
    pub ideation_idle: u32,
    pub ideation_waiting: u32,
    pub ideation_max_project: u32,
    pub ideation_max_global: u32,
}

pub fn build_execution_status_response(input: ExecutionStatusInput) -> ExecutionStatusResponse {
    ExecutionStatusResponse {
        is_paused: input.is_paused,
        halt_mode: input.halt_mode,
        running_count: input.running_count,
        max_concurrent: input.max_concurrent,
        global_max_concurrent: input.global_max_concurrent,
        queued_count: input.queued_count,
        queued_message_count: input.queued_message_count,
        can_start_task: !input.is_paused
            && !input.provider_blocked
            && input.total_project_active < input.max_concurrent
            && input.global_running_count < input.global_max_concurrent,
        provider_blocked: input.provider_blocked,
        provider_blocked_until: (input.provider_blocked_until_epoch > 0)
            .then_some(input.provider_blocked_until_epoch),
        ideation_active: input.ideation_active,
        ideation_idle: input.ideation_idle,
        ideation_waiting: input.ideation_waiting,
        ideation_max_project: input.ideation_max_project,
        ideation_max_global: input.ideation_max_global,
    }
}

#[cfg(test)]
#[path = "status_response_tests.rs"]
mod tests;
