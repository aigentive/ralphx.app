/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */
import { PLAN_TOOLS } from "./plan-tools.js";
import { WORKER_CONTEXT_TOOLS } from "./worker-context-tools.js";
import { STEP_TOOLS } from "./step-tools.js";
import { ISSUE_TOOLS } from "./issue-tools.js";
/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export const ALL_TOOLS = [
    // ========================================================================
    // IDEATION TOOLS (orchestrator-ideation agent)
    // ========================================================================
    {
        name: "create_task_proposal",
        description: "Create a new task proposal in the ideation session. Use this when the user describes a new feature, fix, or improvement they want to implement.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Clear, concise task title (e.g., 'Add dark mode toggle')",
                },
                description: {
                    type: "string",
                    description: "Detailed description of what needs to be done",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Task category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Suggested priority level. Default: medium",
                },
                steps: {
                    type: "array",
                    items: { type: "string" },
                    description: "Step-by-step implementation plan. Each step should be a clear, actionable task (1-3 sentences). Typically 3-7 steps.",
                },
                acceptance_criteria: {
                    type: "array",
                    items: { type: "string" },
                    description: "Testable criteria to verify task completion (e.g., 'API returns 200 with valid schema', 'All tests pass'). Typically 3-5 criteria.",
                },
            },
            required: ["session_id", "title", "category"],
        },
    },
    {
        name: "update_task_proposal",
        description: "Update an existing task proposal. Use when the user wants to modify a proposal's details, priority, or implementation plan.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to update",
                },
                title: {
                    type: "string",
                    description: "Updated task title",
                },
                description: {
                    type: "string",
                    description: "Updated description",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Updated category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                user_priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Updated priority level (overrides AI-suggested priority)",
                },
                steps: {
                    type: "array",
                    items: { type: "string" },
                    description: "Updated implementation steps. Each step should be a clear, actionable task (1-3 sentences). Typically 3-7 steps.",
                },
                acceptance_criteria: {
                    type: "array",
                    items: { type: "string" },
                    description: "Updated acceptance criteria. Testable criteria to verify task completion (e.g., 'API returns 200 with valid schema'). Typically 3-5 criteria.",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "delete_task_proposal",
        description: "Delete a task proposal. Use when the user wants to remove a proposal that's no longer needed.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to delete",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "add_proposal_dependency",
        description: "Add a dependency relationship between two proposals. Use when one task must be completed before another can start.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal that depends on another",
                },
                depends_on_id: {
                    type: "string",
                    description: "The proposal that must be completed first",
                },
            },
            required: ["proposal_id", "depends_on_id"],
        },
    },
    {
        name: "apply_proposal_dependencies",
        description: "Apply AI-suggested dependencies directly to proposals. Clears existing dependencies and applies new ones. Used by dependency-suggester agent.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
                dependencies: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            proposal_id: {
                                type: "string",
                                description: "The proposal that depends on another",
                            },
                            depends_on_id: {
                                type: "string",
                                description: "The proposal that must be completed first",
                            },
                            reason: {
                                type: "string",
                                description: "Brief explanation of why this dependency exists",
                            },
                        },
                        required: ["proposal_id", "depends_on_id"],
                    },
                    description: "Array of dependency suggestions to apply",
                },
            },
            required: ["session_id", "dependencies"],
        },
    },
    {
        name: "update_session_title",
        description: "Update the title of an ideation session. Used by session-namer agent to set auto-generated titles.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID to update",
                },
                title: {
                    type: "string",
                    description: "The new title for the session (exactly 2 words)",
                },
            },
            required: ["session_id", "title"],
        },
    },
    {
        name: "list_session_proposals",
        description: "List all task proposals in an ideation session. Returns summary info (id, title, category, priority, dependencies). Use get_proposal for full details including steps and acceptance criteria.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "get_proposal",
        description: "Get full details of a task proposal including steps and acceptance criteria. Use after list_session_proposals to get complete information for a specific proposal.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to fetch",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "analyze_session_dependencies",
        description: "Get full dependency graph analysis including critical path, cycle detection, and blocking relationships. " +
            "Use to provide intelligent recommendations about proposal execution order. " +
            "If analysis_in_progress is true in the response, wait 2-3 seconds and retry for complete results.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID to analyze",
                },
            },
            required: ["session_id"],
        },
    },
    // ========================================================================
    // QUESTION TOOLS (orchestrator-ideation agent — inline AskUserQuestion)
    // ========================================================================
    {
        name: "ask_user_question",
        description: "Ask the user a clarifying question with optional predefined answer options. " +
            "The question appears as an inline card in the chat. " +
            "This tool blocks until the user responds (up to 5 minutes). " +
            "Use for confirmations, multi-choice selections, or open-ended questions during ideation.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                question: {
                    type: "string",
                    description: "The question text to display to the user",
                },
                header: {
                    type: "string",
                    description: "Optional header/title above the question (e.g., 'Confirm Plan')",
                },
                options: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            label: {
                                type: "string",
                                description: "Short label for the option (e.g., 'Yes', 'Option A')",
                            },
                            value: {
                                type: "string",
                                description: "Programmatic value returned when this option is selected. Defaults to label if omitted.",
                            },
                            description: {
                                type: "string",
                                description: "Optional longer description of what this option means",
                            },
                        },
                        required: ["label"],
                    },
                    description: "Predefined answer options. If omitted, user can type a free-form response.",
                },
                multi_select: {
                    type: "boolean",
                    description: "If true and options are provided, user can select multiple options. Default: false.",
                },
            },
            required: ["session_id", "question"],
        },
    },
    // ========================================================================
    // TASK TOOLS (chat-task agent)
    // ========================================================================
    {
        name: "update_task",
        description: "Update an existing task's details. Use when the user wants to modify task title, description, or priority. For status changes, use move_task or workflow commands.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to update",
                },
                title: {
                    type: "string",
                    description: "Updated task title",
                },
                description: {
                    type: "string",
                    description: "Updated description",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Updated priority",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "add_task_note",
        description: "Add a note or comment to a task. Use when the user wants to document progress, issues, or decisions.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID",
                },
                note: {
                    type: "string",
                    description: "The note content",
                },
            },
            required: ["task_id", "note"],
        },
    },
    {
        name: "get_task_details",
        description: "Get full details for a task including current status, notes, and history. Use when you need complete task information.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID",
                },
            },
            required: ["task_id"],
        },
    },
    // ========================================================================
    // PROJECT TOOLS (chat-project agent)
    // ========================================================================
    {
        name: "suggest_task",
        description: "Suggest a new task based on project analysis. Use when you've identified something that should be done based on codebase exploration.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Suggested task title",
                },
                description: {
                    type: "string",
                    description: "Why this task should be done",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Task category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Suggested priority level",
                },
            },
            required: ["project_id", "title", "description", "category"],
        },
    },
    {
        name: "list_tasks",
        description: "List tasks in the project with optional filtering. Use to answer questions about what tasks exist, their status, or priorities.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                status: {
                    type: "string",
                    enum: [
                        "backlog",
                        "ready",
                        "blocked",
                        "executing",
                        "qa_refining",
                        "qa_testing",
                        "qa_passed",
                        "qa_failed",
                        "pending_review",
                        "reviewing",
                        "review_passed",
                        "escalated",
                        "revision_needed",
                        "re_executing",
                        "approved",
                        "failed",
                        "cancelled",
                    ],
                    description: "Filter by status (optional)",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Filter by category (optional): setup, feature, fix, refactor, docs, test, performance, security, devops, research, design, chore",
                },
            },
            required: ["project_id"],
        },
    },
    // ========================================================================
    // MERGE TOOLS (merger agent)
    // ========================================================================
    {
        name: "complete_merge",
        description: "Signal successful merge completion. Call get_merge_target first to determine the correct target branch." +
            "\n\nIMPORTANT: Call this AFTER you have:" +
            "\n1. Resolved all conflicts (if any)" +
            "\n2. Merged the task branch INTO the target branch (git checkout <target_branch> && git merge <source_branch>)" +
            "\n3. Obtained the merge commit SHA from the target branch (git rev-parse HEAD on target branch)" +
            "\n\nThe commit_sha MUST be a commit ON the target branch, not the task branch." +
            "\nThis transitions the task from Merging to Merged state and triggers cleanup of the task branch/worktree.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID that was being merged",
                },
                commit_sha: {
                    type: "string",
                    description: "The SHA of the merge commit ON the target branch (run: git rev-parse HEAD after merging into target)",
                },
            },
            required: ["task_id", "commit_sha"],
        },
    },
    {
        name: "report_conflict",
        description: "Signal that merge conflicts could not be resolved automatically. Call this when conflicts are too complex (ambiguous intent, architectural incompatibility, or missing context). This transitions the task from Merging to MergeConflict state, keeping the branch/worktree for manual resolution.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID with unresolved conflicts",
                },
                conflict_files: {
                    type: "array",
                    items: { type: "string" },
                    description: "List of file paths that still have conflicts",
                },
                reason: {
                    type: "string",
                    description: "Explanation of why the conflicts couldn't be resolved",
                },
            },
            required: ["task_id", "conflict_files", "reason"],
        },
    },
    {
        name: "report_incomplete",
        description: "Report that merge cannot be completed due to non-conflict errors (e.g., git operation failures, missing configuration). " +
            "Use this instead of report_conflict when there are no actual merge conflicts but the merge still failed. " +
            "This transitions the task from Merging to MergeIncomplete state.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID where merge failed",
                },
                reason: {
                    type: "string",
                    description: "Detailed explanation of why the merge failed",
                },
                diagnostic_info: {
                    type: "string",
                    description: "Git status, logs, or other diagnostic output to help debug the issue",
                },
            },
            required: ["task_id", "reason"],
        },
    },
    {
        name: "get_merge_target",
        description: "Get the resolved merge target branches for a task. " +
            "Returns source_branch (task's branch) and target_branch (where to merge INTO). " +
            "IMPORTANT: Always call this BEFORE merging to know the correct target. " +
            "The target may be a plan feature branch instead of main.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: { type: "string", description: "The task ID" },
            },
            required: ["task_id"],
        },
    },
    // ========================================================================
    // REVIEW TOOLS (reviewer agent)
    // ========================================================================
    {
        name: "complete_review",
        description: "Submit a code review decision. Use after reviewing changes to approve, request changes, or escalate to supervisor.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task being reviewed",
                },
                decision: {
                    type: "string",
                    enum: ["approved", "needs_changes", "escalate"],
                    description: "Review decision: approved (ship it), needs_changes (fixable issues), escalate (major concerns)",
                },
                feedback: {
                    type: "string",
                    description: "Detailed feedback: what's good, what needs improvement, specific issues found",
                },
                issues: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "major", "minor", "suggestion"],
                            },
                            file: { type: "string" },
                            line: { type: "number" },
                            description: { type: "string" },
                        },
                        required: ["severity", "description"],
                    },
                    description: "Specific issues found during review",
                },
            },
            required: ["task_id", "decision", "feedback"],
        },
    },
    {
        name: "get_review_notes",
        description: "Get all review feedback for a task. Call this before re-executing a task to understand what needs to be fixed.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to get review notes for",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "approve_task",
        description: "Approve a task after AI review. ONLY available when task is in 'review_passed' or 'escalated' status (awaiting human decision). " +
            "Use this when the user confirms they want to approve the task after discussing the review with you. " +
            "This will NOT work during active review - use complete_review for that.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to approve",
                },
                comment: {
                    type: "string",
                    description: "Optional approval comment or notes",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "request_task_changes",
        description: "Request changes on a task after AI review. ONLY available when task is in 'review_passed' or 'escalated' status (awaiting human decision). " +
            "Use this when the user wants to request changes after discussing the review with you. " +
            "This will NOT work during active review - use complete_review for that.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to request changes on",
                },
                feedback: {
                    type: "string",
                    description: "Detailed feedback explaining what changes are needed",
                },
            },
            required: ["task_id", "feedback"],
        },
    },
    // ========================================================================
    // PLAN ARTIFACT TOOLS (orchestrator-ideation agent)
    // ========================================================================
    ...PLAN_TOOLS,
    // ========================================================================
    // WORKER CONTEXT TOOLS (worker agent)
    // ========================================================================
    ...WORKER_CONTEXT_TOOLS,
    // ========================================================================
    // STEP TOOLS (worker agent)
    // ========================================================================
    ...STEP_TOOLS,
    // ========================================================================
    // ISSUE TOOLS (worker + reviewer agents)
    // ========================================================================
    ...ISSUE_TOOLS,
    // ========================================================================
    // PROJECT ANALYSIS TOOLS (worker/reviewer/merger + project-analyzer agents)
    // ========================================================================
    {
        name: "get_project_analysis",
        description: "Get project analysis data including build commands, validation commands, and worktree setup instructions. " +
            "Returns path-scoped entries with resolved template variables ({project_root}, {worktree_path}, {task_branch}). " +
            "If analysis hasn't been run yet, returns { status: 'analyzing', retry_after_secs: 30 }.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (from RALPHX_PROJECT_ID env var)",
                },
                task_id: {
                    type: "string",
                    description: "Optional task ID for resolving {worktree_path} and {task_branch} template variables",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "save_project_analysis",
        description: "Save auto-detected project analysis data. Updates detected_analysis and analyzed_at fields. " +
            "Never touches custom_analysis (user overrides). Only callable by the project-analyzer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                entries: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            path: {
                                type: "string",
                                description: "Subpath relative to project root (e.g., '.', 'src-tauri/')",
                            },
                            label: {
                                type: "string",
                                description: "Human-readable label (e.g., 'Node.js root', 'Rust backend')",
                            },
                            install: {
                                type: "string",
                                description: "Install command (e.g., 'npm install'). Null if not needed.",
                            },
                            validate: {
                                type: "array",
                                items: { type: "string" },
                                description: "Validation commands (e.g., ['npm run typecheck', 'npm run lint'])",
                            },
                            worktree_setup: {
                                type: "array",
                                items: { type: "string" },
                                description: "Commands to run in worktree setup (e.g., ['ln -s {project_root}/node_modules {worktree_path}/node_modules'])",
                            },
                        },
                        required: ["path", "label"],
                    },
                    description: "Array of path-scoped analysis entries",
                },
            },
            required: ["project_id", "entries"],
        },
    },
];
/**
 * Tool scoping per agent type
 * Hard enforcement: each agent only sees tools appropriate for its role
 */
export const TOOL_ALLOWLIST = {
    "orchestrator-ideation": [
        "create_task_proposal",
        "update_task_proposal",
        "delete_task_proposal",
        // Note: add_proposal_dependency removed - dependencies are now auto-suggested by dependency-suggester agent
        "list_session_proposals",
        "get_proposal",
        "analyze_session_dependencies",
        "create_plan_artifact",
        "update_plan_artifact",
        "get_plan_artifact",
        "link_proposals_to_plan",
        "get_session_plan",
        "ask_user_question",
    ],
    "chat-task": ["update_task", "add_task_note", "get_task_details"],
    "chat-project": ["suggest_task", "list_tasks"],
    "ralphx-reviewer": [
        // specific review tools
        "complete_review",
        // issue tools (re-review workflow)
        "get_task_issues",
        "get_step_progress",
        "get_issue_progress",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
    ],
    // Post-review chat agent - helps user discuss review findings and take action
    "ralphx-review-chat": [
        // specific review tools
        "approve_task",
        "request_task_changes",
        // common context tools
        "get_review_notes",
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
    ],
    "ralphx-worker": [
        // step management tools
        "start_step",
        "complete_step",
        "skip_step",
        "fail_step",
        "add_step",
        "get_step_progress",
        // issue tools (re-execution workflow)
        "get_task_issues",
        "mark_issue_in_progress",
        "mark_issue_addressed",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
    ],
    // Session naming agent - generates titles for IDA sessions
    "session-namer": ["update_session_title"],
    // Dependency suggester agent - analyzes proposals and auto-applies dependencies
    "dependency-suggester": ["apply_proposal_dependencies"],
    // Merger agent - resolves merge conflicts when programmatic merge fails
    "ralphx-merger": [
        // merge tools
        "complete_merge",
        "report_conflict",
        "report_incomplete",
        "get_merge_target",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
    ],
    // Project analyzer agent - detects build/validation commands
    "project-analyzer": [
        "save_project_analysis",
        "get_project_analysis",
    ],
    // These agents have NO MCP tools - they use filesystem tools only
    supervisor: [],
    "qa-prep": [],
    "qa-tester": [],
    // Debug mode: shows ALL tools (use RALPHX_AGENT_TYPE=debug)
    debug: ALL_TOOLS.map((t) => t.name),
};
/**
 * Module-level agent type storage
 * Set by index.ts on startup after parsing CLI args
 * This is needed because CLI args take precedence over env vars
 * (Claude CLI doesn't pass env vars to MCP servers it spawns)
 */
let currentAgentType = "";
/**
 * Set the current agent type (called from index.ts after parsing CLI args)
 * @param agentType - The agent type to set
 */
export function setAgentType(agentType) {
    currentAgentType = agentType;
}
/**
 * Get the current agent type
 * @returns The current agent type
 */
export function getAgentType() {
    return currentAgentType || process.env.RALPHX_AGENT_TYPE || "";
}
/**
 * Get allowed tool names for the current agent type
 * @returns Array of tool names this agent is allowed to use
 */
export function getAllowedToolNames() {
    const agentType = getAgentType();
    return TOOL_ALLOWLIST[agentType] || [];
}
/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export function getFilteredTools() {
    const allowedNames = getAllowedToolNames();
    return ALL_TOOLS.filter((tool) => allowedNames.includes(tool.name));
}
/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export function isToolAllowed(toolName) {
    const allowedNames = getAllowedToolNames();
    return allowedNames.includes(toolName);
}
/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export function getAllTools() {
    return ALL_TOOLS;
}
/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export function getToolsByAgent() {
    return TOOL_ALLOWLIST;
}
/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export function logAllTools() {
    console.error("\n=== RalphX MCP Server - All Available Tools ===\n");
    for (const [agentType, tools] of Object.entries(TOOL_ALLOWLIST)) {
        if (tools.length > 0) {
            console.error(`[${agentType}]`);
            tools.forEach((t) => console.error(`  - ${t}`));
            console.error("");
        }
    }
    console.error("=== End of Tools List ===\n");
}
//# sourceMappingURL=tools.js.map