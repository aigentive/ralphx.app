// Centralized agent name constants
//
// Single source of truth for all fully-qualified agent names used throughout
// the RalphX Rust backend. These match the `ralphx:<name>` format where `<name>`
// is the `name:` field from each agent's frontmatter in ralphx-plugin/agents/*.md.
//
// If an agent is renamed in the plugin, update ONLY this file.

/// Plugin prefix for all RalphX agents
pub const PLUGIN_PREFIX: &str = "ralphx:";

// ── Short names (without "ralphx:" prefix) ───────────────────────────────
// Used by AGENT_CONFIGS in agent_config/ and MCP TOOL_ALLOWLIST in tools.ts.
// These match the `name:` field in each agent's frontmatter (ralphx-plugin/agents/*.md).

pub const SHORT_ORCHESTRATOR_IDEATION: &str = "orchestrator-ideation";
pub const SHORT_ORCHESTRATOR_IDEATION_READONLY: &str = "orchestrator-ideation-readonly";
pub const SHORT_SESSION_NAMER: &str = "session-namer";
pub const SHORT_CHAT_TASK: &str = "chat-task";
pub const SHORT_CHAT_PROJECT: &str = "chat-project";
pub const SHORT_REVIEW_CHAT: &str = "ralphx-review-chat";
pub const SHORT_REVIEW_HISTORY: &str = "ralphx-review-history";
pub const SHORT_WORKER: &str = "ralphx-worker";
pub const SHORT_CODER: &str = "ralphx-coder";
pub const SHORT_REVIEWER: &str = "ralphx-reviewer";
pub const SHORT_QA_PREP: &str = "ralphx-qa-prep";
pub const SHORT_QA_EXECUTOR: &str = "ralphx-qa-executor";
pub const SHORT_ORCHESTRATOR: &str = "ralphx-orchestrator";
pub const SHORT_SUPERVISOR: &str = "ralphx-supervisor";
pub const SHORT_DEEP_RESEARCHER: &str = "ralphx-deep-researcher";
pub const SHORT_PROJECT_ANALYZER: &str = "project-analyzer";
pub const SHORT_MERGER: &str = "ralphx-merger";
pub const SHORT_MEMORY_MAINTAINER: &str = "memory-maintainer";
pub const SHORT_MEMORY_CAPTURE: &str = "memory-capture";

// ── Plan verification critic agents ─────────────────────────────────────
pub const SHORT_PLAN_CRITIC_LAYER1: &str = "plan-critic-layer1";
pub const SHORT_PLAN_CRITIC_LAYER2: &str = "plan-critic-layer2";
pub const SHORT_PLAN_VERIFIER: &str = "plan-verifier";

// ── Team lead variants (extends base agents) ────────────────────────────
pub const SHORT_IDEATION_TEAM_LEAD: &str = "ideation-team-lead";
pub const SHORT_WORKER_TEAM: &str = "ralphx-worker-team";
pub const SHORT_IDEATION_TEAM_MEMBER: &str = "ideation-team-member";

// ── Ideation specialist agents (spawned by ideation-team-lead) ───────────
pub const SHORT_IDEATION_SPECIALIST_BACKEND: &str = "ideation-specialist-backend";
pub const SHORT_IDEATION_SPECIALIST_FRONTEND: &str = "ideation-specialist-frontend";
pub const SHORT_IDEATION_SPECIALIST_INFRA: &str = "ideation-specialist-infra";
pub const SHORT_IDEATION_SPECIALIST_UX: &str = "ideation-specialist-ux";
pub const SHORT_IDEATION_SPECIALIST_CODE_QUALITY: &str = "ideation-specialist-code-quality";
pub const SHORT_IDEATION_ADVOCATE: &str = "ideation-advocate";
pub const SHORT_IDEATION_CRITIC: &str = "ideation-critic";

// ── ChatService team agents (team_mode=true → resolve_agent_with_team_mode) ──

/// Ideation team lead (ChatContextType::Ideation + team_mode)
pub const AGENT_IDEATION_TEAM_LEAD: &str = "ralphx:ideation-team-lead";

/// Worker team lead (ChatContextType::TaskExecution + team_mode)
pub const AGENT_WORKER_TEAM: &str = "ralphx:ralphx-worker-team";

// ── ChatService agents (resolve_agent → build_command → --agent flag) ─────

/// Ideation orchestrator (default for ChatContextType::Ideation)
pub const AGENT_ORCHESTRATOR_IDEATION: &str = "ralphx:orchestrator-ideation";

/// Ideation orchestrator in read-only mode (session status = "accepted")
pub const AGENT_ORCHESTRATOR_IDEATION_READONLY: &str = "ralphx:orchestrator-ideation-readonly";

/// Plan verifier agent (ChatContextType::Ideation when session_purpose = Verification)
pub const AGENT_PLAN_VERIFIER: &str = "ralphx:plan-verifier";

/// Task-scoped chat (ChatContextType::Task)
pub const AGENT_CHAT_TASK: &str = "ralphx:chat-task";

/// Project-scoped chat (ChatContextType::Project)
pub const AGENT_CHAT_PROJECT: &str = "ralphx:chat-project";

/// Worker execution agent (ChatContextType::TaskExecution)
pub const AGENT_WORKER: &str = "ralphx:ralphx-worker";

/// Delegated coding execution agent (invoked by worker orchestration)
pub const AGENT_CODER: &str = "ralphx:ralphx-coder";

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
        "coder" | "ralphx-coder" => AGENT_CODER,
        "reviewer" | "ralphx-reviewer" => AGENT_REVIEWER,
        "merger" | "ralphx-merger" => AGENT_MERGER,
        // Fallback: qualify using prefix
        other => {
            tracing::warn!(
                agent_type = other,
                "Unknown spawner agent type, falling back to qualify_agent_name"
            );
            Box::leak(super::qualify_agent_name(other).into_boxed_str())
        }
    }
}
