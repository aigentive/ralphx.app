// Centralized agent name constants
//
// Single source of truth for all fully-qualified agent names used throughout
// the RalphX Rust backend. These match the `ralphx:<name>` format where `<name>`
// is the `name:` field from each agent's frontmatter in ralphx-plugin/agents/*.md.
//
// If an agent is renamed in the plugin, update ONLY this file.

/// Plugin prefix for all RalphX agents
pub const PLUGIN_PREFIX: &str = "ralphx:";

// ── ChatService agents (resolve_agent → build_command → --agent flag) ─────

/// Ideation orchestrator (default for ChatContextType::Ideation)
pub const AGENT_ORCHESTRATOR_IDEATION: &str = "ralphx:orchestrator-ideation";

/// Ideation orchestrator in read-only mode (session status = "accepted")
pub const AGENT_ORCHESTRATOR_IDEATION_READONLY: &str = "ralphx:orchestrator-ideation-readonly";

/// Task-scoped chat (ChatContextType::Task)
pub const AGENT_CHAT_TASK: &str = "ralphx:chat-task";

/// Project-scoped chat (ChatContextType::Project)
pub const AGENT_CHAT_PROJECT: &str = "ralphx:chat-project";

/// Worker execution agent (ChatContextType::TaskExecution)
pub const AGENT_WORKER: &str = "ralphx:ralphx-worker";

/// Reviewer agent (ChatContextType::Review, fresh review cycle)
pub const AGENT_REVIEWER: &str = "ralphx:ralphx-reviewer";

/// Merger agent (ChatContextType::Merge)
pub const AGENT_MERGER: &str = "ralphx:ralphx-merger";

/// Review-chat agent (ChatContextType::Review when status = "review_passed")
pub const AGENT_REVIEW_CHAT: &str = "ralphx:ralphx-review-chat";

/// Review-history agent (ChatContextType::Review when status = "approved")
pub const AGENT_REVIEW_HISTORY: &str = "ralphx:ralphx-review-history";

// ── Fire-and-forget agents (spawn_agent path) ────────────────────────────

/// Session naming agent (haiku, background)
pub const AGENT_SESSION_NAMER: &str = "ralphx:session-namer";

/// Dependency suggestion agent (haiku, background)
pub const AGENT_DEPENDENCY_SUGGESTER: &str = "ralphx:dependency-suggester";

/// Project analysis agent (background)
pub const AGENT_PROJECT_ANALYZER: &str = "ralphx:project-analyzer";

// ── QA agents (spawner path from state machine) ──────────────────────────

/// QA preparation agent (background, spawned on Ready state entry)
pub const AGENT_QA_PREP: &str = "ralphx:ralphx-qa-prep";

/// QA refiner agent (spawned on QaRefining state entry)
/// Note: No dedicated plugin agent file — uses default Claude behavior
pub const AGENT_QA_REFINER: &str = "ralphx:qa-refiner";

/// QA tester agent (spawned on QaTesting state entry)
/// Note: No dedicated plugin agent file — uses default Claude behavior
pub const AGENT_QA_TESTER: &str = "ralphx:qa-tester";

/// Map a state-machine spawner agent type string to the correct FQ agent name.
///
/// The state machine uses short identifiers ("qa-prep", "qa-refiner", "qa-tester")
/// when calling `agent_spawner.spawn()`. Tests also pass "worker" and "reviewer".
/// This maps them to their fully-qualified names that match the plugin's agent frontmatter.
pub fn spawner_agent_name(agent_type: &str) -> &'static str {
    match agent_type {
        // Production: QA agents via state machine side effects
        "qa-prep" => AGENT_QA_PREP,
        "qa-refiner" => AGENT_QA_REFINER,
        "qa-tester" => AGENT_QA_TESTER,
        // Test/fallback: worker, reviewer, merger via spawner tests and mocks
        "worker" | "ralphx-worker" => AGENT_WORKER,
        "reviewer" | "ralphx-reviewer" => AGENT_REVIEWER,
        "merger" | "ralphx-merger" => AGENT_MERGER,
        // Fallback: qualify using prefix
        other => {
            tracing::warn!(agent_type = other, "Unknown spawner agent type, falling back to qualify_agent_name");
            Box::leak(super::qualify_agent_name(other).into_boxed_str())
        }
    }
}
