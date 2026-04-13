#!/usr/bin/env node
/**
 * RalphX MCP Server
 *
 * A proxy MCP server that forwards tool calls to the RalphX Tauri backend via HTTP.
 * All business logic lives in Rust - this server is a thin transport layer.
 *
 * Tool scoping:
 * - Reads agent type from CLI args (--agent-type=<type>) or environment (RALPHX_AGENT_TYPE)
 * - CLI args take precedence (because Claude CLI doesn't pass env vars to MCP servers)
 * - Filters available tools based on agent type (hard enforcement)
 * - Each agent only sees tools appropriate for its role
 */
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { CallToolRequestSchema, ListToolsRequestSchema, } from "@modelcontextprotocol/sdk/types.js";
import { callTauri, callTauriGet, TauriClientError } from "./tauri-client.js";
import { getTraceLogPath, safeError, safeTrace } from "./redact.js";
import { getFilteredTools, isToolAllowed, getAllowedToolNames, parseAllowedToolsFromArgs, formatToolErrorMessage, logAllTools, getToolsByAgent, setAgentType, } from "./tools.js";
import { FILESYSTEM_TOOL_NAMES, formatFilesystemToolError, handleFilesystemToolCall, } from "./filesystem-tools.js";
import { permissionRequestTool, handlePermissionRequest, } from "./permission-handler.js";
import { handleAskUserQuestion } from "./question-handler.js";
import { handleRequestTeamPlan } from "./team-plan-handler.js";
import { hydrateRalphxRuntimeEnvFromCli, parseCliOptionFromArgs, } from "./runtime-context.js";
import { assessVerificationRound, } from "./verification-round-assessment.js";
import { completePlanVerificationWithSettlement } from "./verification-completion.js";
import { runVerificationEnrichmentPass, runVerificationRoundPass, } from "./verification-orchestration.js";
/**
 * Semantic keyword patterns for cross-project detection in plan text.
 * Exported for unit testing.
 */
export const CROSS_PROJECT_KEYWORDS = [
    "cross[- ]?project",
    "multi[- ]?project",
    "target project",
    "another project",
    "different project",
    "project[_ ]?b\\b",
    "separate\\s+repo(?:sitory)?",
    "new\\s+repo(?:sitory)?",
    "different\\s+codebase",
    "other\\s+codebase",
    "monorepo\\s+boundary",
    "external\\s+package",
    "external\\s+module",
];
/**
 * Strip fenced and inline markdown code blocks from text before path scanning.
 * Prevents false-positive path detection on code snippets like `...>>` or `...`.
 * Exported for unit testing.
 */
export function stripMarkdownCodeBlocks(text) {
    // Remove fenced code blocks (``` ... ```) — non-greedy, handles multi-line
    let stripped = text.replace(/```[\s\S]*?```/g, "");
    // Remove inline code (`...`)
    stripped = stripped.replace(/`[^`\n]+`/g, "");
    return stripped;
}
/**
 * Filter out detected paths that belong to the same project root.
 * Returns only paths that genuinely reference a different project.
 *
 * @param detectedPaths - Raw list of absolute or relative paths found in plan text
 * @param projectWorkingDir - The project's working directory (e.g. /Users/alice/Code/ralphx)
 * @returns Paths that do NOT start with projectWorkingDir (i.e. are truly cross-project)
 */
export function filterCrossProjectPaths(detectedPaths, projectWorkingDir) {
    if (!projectWorkingDir) {
        return detectedPaths;
    }
    // Normalize: ensure root ends with exactly one slash for prefix matching
    const root = projectWorkingDir.endsWith("/")
        ? projectWorkingDir
        : projectWorkingDir + "/";
    return detectedPaths.filter((p) => {
        // Exact match: path equals project root (without trailing slash)
        if (p === projectWorkingDir)
            return false;
        // Prefix match: path is inside project root
        if (p.startsWith(root))
            return false;
        return true;
    });
}
function summarizeResult(result) {
    if (result === null) {
        return { kind: "null" };
    }
    if (result === undefined) {
        return { kind: "undefined" };
    }
    if (typeof result === "string") {
        return { kind: "string", length: result.length };
    }
    if (typeof result === "number" || typeof result === "boolean") {
        return { kind: typeof result, value: result };
    }
    if (Array.isArray(result)) {
        return { kind: "array", length: result.length };
    }
    if (typeof result === "object") {
        return {
            kind: "object",
            keys: Object.keys(result).slice(0, 20),
        };
    }
    return { kind: typeof result };
}
export function selectLatestArtifactsByPrefix(artifacts, prefixes, createdAfter) {
    const createdAfterMs = typeof createdAfter === "string" && createdAfter.length > 0
        ? Date.parse(createdAfter)
        : Number.NaN;
    const hasThreshold = Number.isFinite(createdAfterMs);
    return prefixes.map((prefix) => {
        const matches = artifacts
            .filter((artifact) => artifact.name.startsWith(prefix))
            .filter((artifact) => {
            if (!hasThreshold)
                return true;
            const createdAtMs = Date.parse(artifact.created_at);
            return Number.isFinite(createdAtMs) && createdAtMs >= createdAfterMs;
        })
            .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at));
        const latest = matches[0];
        return latest
            ? {
                prefix,
                found: true,
                total_matches: matches.length,
                artifact: latest,
            }
            : {
                prefix,
                found: false,
                total_matches: 0,
            };
    });
}
async function sleep(ms) {
    await new Promise((resolve) => setTimeout(resolve, ms));
}
async function loadVerificationArtifactsByPrefix(args) {
    const teamArtifacts = await callTauriGet(`team/artifacts/${args.sessionId}`);
    const matches = selectLatestArtifactsByPrefix(teamArtifacts.artifacts ?? [], args.prefixes, args.createdAfter);
    return await Promise.all(matches.map(async (match) => {
        if (!match.artifact || !args.includeFullContent) {
            return match;
        }
        const fullArtifact = await callTauriGet(`artifact/${match.artifact.id}`);
        return {
            ...match,
            artifact: {
                ...match.artifact,
                content: fullArtifact.content ?? "",
            },
        };
    }));
}
function selectLatestVerificationFindingsByCritic(findings, critics, createdAfter, round) {
    const createdAfterMs = typeof createdAfter === "string" && createdAfter.length > 0
        ? Date.parse(createdAfter)
        : Number.NaN;
    const hasThreshold = Number.isFinite(createdAfterMs);
    return critics.map((critic) => {
        const normalizedCritic = critic.trim().toLowerCase();
        const matches = findings
            .filter((finding) => finding.critic.trim().toLowerCase() === normalizedCritic)
            .filter((finding) => (typeof round === "number" ? finding.round === round : true))
            .filter((finding) => {
            if (!hasThreshold) {
                return true;
            }
            const createdAtMs = Date.parse(finding.created_at);
            return Number.isFinite(createdAtMs) && createdAtMs >= createdAfterMs;
        })
            .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at));
        const latest = matches[0];
        return latest
            ? {
                critic: normalizedCritic,
                found: true,
                total_matches: matches.length,
                finding: latest,
            }
            : {
                critic: normalizedCritic,
                found: false,
                total_matches: 0,
            };
    });
}
async function loadVerificationFindingsByCritic(args) {
    const searchParams = new URLSearchParams();
    if (typeof args.round === "number") {
        searchParams.set("round", String(args.round));
    }
    if (typeof args.createdAfter === "string" && args.createdAfter.length > 0) {
        searchParams.set("created_after", args.createdAfter);
    }
    const query = searchParams.toString();
    const response = await callTauriGet(`team/verification-findings/${args.sessionId}${query.length > 0 ? `?${query}` : ""}`);
    return selectLatestVerificationFindingsByCritic(response.findings ?? [], args.critics, args.createdAfter, args.round);
}
async function loadVerificationDelegateSnapshots(args) {
    return await Promise.all(args.delegates.map(async (delegate) => {
        try {
            return await callTauri("coordination/delegate/wait", {
                job_id: delegate.job_id,
                include_delegated_status: true,
                include_messages: args.includeMessages,
                message_limit: args.messageLimit,
            });
        }
        catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            return {
                job_id: delegate.job_id,
                status: "failed",
                error: errorMessage,
            };
        }
    }));
}
const REQUIRED_VERIFICATION_CRITICS = [
    {
        agent_name: "ralphx:ralphx-plan-critic-completeness",
        critic: "completeness",
        artifact_prefix: "Completeness: ",
        label: "completeness",
        initial_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one completeness verification finding with publish_verification_finding. Use critic='completeness'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
        rescue_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nCompleteness rescue pass. Publish the completeness verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
    },
    {
        agent_name: "ralphx:ralphx-plan-critic-implementation-feasibility",
        critic: "feasibility",
        artifact_prefix: "Feasibility: ",
        label: "feasibility",
        initial_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one feasibility verification finding with publish_verification_finding. Use critic='feasibility'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
        rescue_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nFeasibility rescue pass. Publish the feasibility verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
    },
];
async function loadVerificationPlanSnapshot(sessionId) {
    const planData = await callTauriGet(`get_session_plan/${sessionId}`);
    return {
        artifact_id: typeof planData.artifact_id === "string"
            ? planData.artifact_id
            : typeof planData.id === "string"
                ? planData.id
                : undefined,
        content: typeof planData.content === "string" ? planData.content : "",
        project_working_directory: typeof planData.project_working_directory === "string"
            ? planData.project_working_directory
            : null,
    };
}
async function awaitOptionalVerificationDelegates(args) {
    const deadline = Date.now() + Math.min(Math.max(args.maxWaitMs, 0), 30000);
    let pollsPerformed = 0;
    while (true) {
        pollsPerformed += 1;
        const artifactsByPrefix = await loadVerificationArtifactsByPrefix({
            sessionId: args.sessionId,
            prefixes: args.prefixes,
            createdAfter: args.createdAfter,
            includeFullContent: args.includeFullContent,
        });
        const delegateSnapshots = await loadVerificationDelegateSnapshots({
            delegates: args.delegates,
            includeMessages: args.includeMessages,
            messageLimit: args.messageLimit,
        });
        const allSettled = args.delegates.every((delegate) => {
            const artifact = artifactsByPrefix.find((entry) => entry.prefix === delegate.artifact_prefix);
            if (artifact?.found === true) {
                return true;
            }
            const snapshot = delegateSnapshots.find((entry) => entry.job_id === delegate.job_id);
            const statuses = [
                snapshot?.status,
                snapshot?.delegated_status?.latest_run?.status ?? undefined,
                snapshot?.delegated_status?.agent_state?.estimated_status ?? undefined,
            ];
            return statuses.some((status) => status === "completed" || status === "failed" || status === "cancelled");
        });
        if (allSettled || Date.now() >= deadline) {
            return {
                created_after: args.createdAfter,
                polls_performed: pollsPerformed,
                timed_out: !allSettled,
                delegates: args.delegates.map(({ job_id, artifact_prefix, label, required }) => ({
                    job_id,
                    artifact_prefix,
                    label,
                    required,
                })),
                artifacts_by_prefix: artifactsByPrefix,
                delegate_snapshots: delegateSnapshots,
            };
        }
        await sleep(args.pollIntervalMs);
    }
}
async function startManagedVerificationDelegate(args) {
    return await callTauri("coordination/delegate/start", {
        agent_name: args.agentName,
        parent_session_id: args.parentSessionId,
        prompt: args.prompt,
        delegated_session_id: args.delegatedSessionId,
        caller_agent_name: AGENT_TYPE,
        caller_context_type: RALPHX_CONTEXT_TYPE,
        caller_context_id: RALPHX_CONTEXT_ID,
    });
}
function summarizeVerificationInfraFailure(args) {
    const rescueDelegates = args.rescueDelegates ?? [];
    return {
        session_id: args.sessionId,
        round: args.round,
        created_after: args.createdAfter,
        rescue_dispatched: args.rescueDispatched === true,
        required_delegates: args.delegates.map(({ job_id, artifact_prefix, label, required }) => ({
            job_id,
            artifact_prefix,
            label,
            required,
        })),
        rescue_delegates: rescueDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
            job_id,
            artifact_prefix,
            label,
            required,
        })),
        settlement: {
            session_id: args.sessionId,
            created_after: args.createdAfter,
            rescue_budget_exhausted: args.rescueDispatched === true,
            settled: true,
            timed_out: false,
            polls_performed: 0,
            classification: "infra_failure",
            recommended_next_action: "complete_verification_with_infra_failure",
            summary: args.summary,
            missing_required_prefixes: REQUIRED_VERIFICATION_CRITICS
                .filter((critic) => !args.delegates.some((delegate) => delegate.artifact_prefix === critic.artifact_prefix))
                .map((critic) => critic.artifact_prefix),
            delegate_assessments: [],
            artifacts_by_prefix: [],
            error: args.error ?? null,
        },
    };
}
async function runRequiredVerificationCriticRound(args) {
    const dispatchStartedAt = Date.now();
    const createdAfter = new Date(dispatchStartedAt - 5000).toISOString();
    const initialLaunches = await Promise.all(REQUIRED_VERIFICATION_CRITICS.map(async (critic) => {
        try {
            const launched = await startManagedVerificationDelegate({
                agentName: critic.agent_name,
                parentSessionId: args.sessionId,
                prompt: critic.initial_prompt(args.sessionId, args.round),
            });
            return {
                ok: true,
                critic,
                delegate: {
                    job_id: launched.job_id,
                    delegated_session_id: launched.delegated_session_id,
                    agent_name: critic.agent_name,
                    artifact_prefix: critic.artifact_prefix,
                    label: critic.label,
                    required: true,
                },
            };
        }
        catch (error) {
            return {
                ok: false,
                critic,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }));
    const initialDelegates = initialLaunches
        .filter((launch) => launch.ok)
        .map((launch) => launch.delegate);
    const initialLaunchFailures = initialLaunches.filter((launch) => !launch.ok);
    if (initialLaunchFailures.length > 0) {
        return summarizeVerificationInfraFailure({
            sessionId: args.sessionId,
            round: args.round,
            createdAfter,
            delegates: initialDelegates,
            summary: "Required critic dispatch failed before the verification round could settle.",
            error: initialLaunchFailures
                .map((failure) => `${failure.critic.label}: ${failure.error}`)
                .join("; "),
        });
    }
    const initialDelegateInputs = initialDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
    }));
    const firstSettlement = await awaitVerificationRoundSettlement({
        session_id: args.sessionId,
        delegates: initialDelegateInputs,
        created_after: createdAfter,
        rescue_budget_exhausted: false,
        include_full_content: args.includeFullContent,
        include_messages: args.includeMessages,
        message_limit: args.messageLimit,
        max_wait_ms: args.maxWaitMs,
        poll_interval_ms: args.pollIntervalMs,
    });
    if (firstSettlement.classification !== "pending") {
        return {
            session_id: args.sessionId,
            round: args.round,
            created_after: createdAfter,
            rescue_dispatched: false,
            required_delegates: initialDelegateInputs,
            rescue_delegates: [],
            settlement: firstSettlement,
        };
    }
    const missingPrefixes = new Set(firstSettlement.missing_required_prefixes);
    const rescueTargets = initialDelegates.filter((delegate) => missingPrefixes.has(delegate.artifact_prefix));
    const rescueLaunches = await Promise.all(rescueTargets.map(async (target) => {
        const critic = REQUIRED_VERIFICATION_CRITICS.find((entry) => entry.artifact_prefix === target.artifact_prefix);
        if (!critic) {
            return {
                ok: false,
                delegate: target,
                error: `Unknown required critic prefix ${target.artifact_prefix}`,
            };
        }
        try {
            const launched = await startManagedVerificationDelegate({
                agentName: critic.agent_name,
                parentSessionId: args.sessionId,
                delegatedSessionId: target.delegated_session_id,
                prompt: critic.rescue_prompt(args.sessionId, args.round),
            });
            return {
                ok: true,
                target,
                delegate: {
                    job_id: launched.job_id,
                    delegated_session_id: launched.delegated_session_id ?? target.delegated_session_id,
                    agent_name: critic.agent_name,
                    artifact_prefix: critic.artifact_prefix,
                    label: critic.label,
                    required: true,
                },
            };
        }
        catch (error) {
            return {
                ok: false,
                delegate: target,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }));
    const rescueFailures = rescueLaunches.filter((launch) => !launch.ok);
    const successfulRescues = rescueLaunches
        .filter((launch) => launch.ok)
        .map((launch) => launch.delegate);
    if (rescueFailures.length > 0) {
        return summarizeVerificationInfraFailure({
            sessionId: args.sessionId,
            round: args.round,
            createdAfter,
            delegates: initialDelegates,
            rescueDispatched: true,
            rescueDelegates: successfulRescues,
            summary: "A required critic rescue dispatch failed, so the verification round cannot be trusted as plan feedback.",
            error: rescueFailures
                .map((failure) => `${failure.delegate.label ?? failure.delegate.artifact_prefix}: ${failure.error}`)
                .join("; "),
        });
    }
    const finalDelegates = initialDelegates.map((delegate) => {
        const replacement = successfulRescues.find((rescue) => rescue.artifact_prefix === delegate.artifact_prefix);
        return replacement ?? delegate;
    });
    const finalDelegateInputs = finalDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
    }));
    const finalSettlement = await awaitVerificationRoundSettlement({
        session_id: args.sessionId,
        delegates: finalDelegateInputs,
        created_after: createdAfter,
        rescue_budget_exhausted: true,
        include_full_content: args.includeFullContent,
        include_messages: args.includeMessages,
        message_limit: args.messageLimit,
        max_wait_ms: args.maxWaitMs,
        poll_interval_ms: args.pollIntervalMs,
    });
    return {
        session_id: args.sessionId,
        round: args.round,
        created_after: createdAfter,
        rescue_dispatched: true,
        required_delegates: finalDelegateInputs,
        rescue_delegates: successfulRescues.map(({ job_id, artifact_prefix, label, required }) => ({
            job_id,
            artifact_prefix,
            label,
            required,
        })),
        settlement: finalSettlement,
    };
}
async function awaitVerificationRoundSettlement(args) {
    const includeMessages = args.include_messages !== false;
    const messageLimit = Math.min(Math.max(args.message_limit ?? 5, 1), 50);
    const rescueBudgetExhausted = args.rescue_budget_exhausted === true;
    const maxWaitMs = Math.min(Math.max(args.max_wait_ms ?? 8000, 0), 30000);
    const pollIntervalMs = Math.max(args.poll_interval_ms ?? 750, 100);
    const uniquePrefixes = Array.from(new Set(args.delegates.map((delegate) => delegate.artifact_prefix)));
    let pollsPerformed = 0;
    let timedOut = false;
    const deadline = Date.now() + maxWaitMs;
    while (true) {
        pollsPerformed += 1;
        const findingMatches = await loadVerificationFindingsByCritic({
            sessionId: args.session_id,
            critics: Array.from(new Set(args.delegates
                .map((delegate) => delegate.label?.trim().toLowerCase())
                .filter((label) => Boolean(label)))),
            createdAfter: args.created_after,
        });
        const findingByCritic = new Map(findingMatches.map((match) => [match.critic, match]));
        const artifacts_by_prefix = uniquePrefixes.map((prefix) => {
            const delegate = args.delegates.find((entry) => entry.artifact_prefix === prefix);
            const critic = delegate?.label?.trim().toLowerCase();
            const findingMatch = critic ? findingByCritic.get(critic) : undefined;
            return findingMatch?.found && findingMatch.finding
                ? {
                    prefix,
                    found: true,
                    total_matches: findingMatch.total_matches,
                    artifact: {
                        id: findingMatch.finding.artifact_id,
                        name: findingMatch.finding.title,
                        created_at: findingMatch.finding.created_at,
                    },
                }
                : {
                    prefix,
                    found: false,
                    total_matches: 0,
                };
        });
        const delegateSnapshots = await loadVerificationDelegateSnapshots({
            delegates: args.delegates,
            includeMessages,
            messageLimit,
        });
        const assessment = assessVerificationRound({
            delegates: args.delegates,
            artifactsByPrefix: artifacts_by_prefix,
            delegateSnapshots,
            rescueBudgetExhausted,
        });
        const settled = assessment.classification !== "pending";
        if (settled || Date.now() >= deadline) {
            timedOut = !settled && assessment.classification === "pending";
            const finalAssessment = timedOut && rescueBudgetExhausted
                ? {
                    ...assessment,
                    classification: "infra_failure",
                    recommended_next_action: "complete_verification_with_infra_failure",
                    summary: assessment.missing_required_prefixes.length > 0
                        ? `Required verification artifacts are still missing after waiting for delegates to either publish artifacts or reach terminal state: ${assessment.missing_required_prefixes.join(", ")}.`
                        : "Required verification delegates did not settle before the terminal wait budget expired.",
                }
                : assessment;
            return {
                session_id: args.session_id,
                created_after: args.created_after ?? null,
                rescue_budget_exhausted: rescueBudgetExhausted,
                settled,
                timed_out: timedOut,
                polls_performed: pollsPerformed,
                max_wait_ms: maxWaitMs,
                poll_interval_ms: pollIntervalMs,
                verification_findings: findingMatches
                    .filter((match) => match.found && match.finding)
                    .map((match) => match.finding),
                ...finalAssessment,
            };
        }
        await sleep(pollIntervalMs);
    }
}
const runtimeContext = hydrateRalphxRuntimeEnvFromCli(process.argv, process.env);
const cliAgentType = parseCliOptionFromArgs(process.argv, "agent-type");
// Agent type: prefer CLI args over environment and hydrate process.env from CLI first
// because Codex does not reliably propagate parent env vars into MCP child processes.
const AGENT_TYPE = runtimeContext.agentType || "unknown";
// Set the agent type in tools module for filtering
setAgentType(AGENT_TYPE);
// Log how agent type was determined
if (cliAgentType) {
    safeError(`[RalphX MCP] Agent type from CLI args: ${AGENT_TYPE}`);
}
else if (process.env.RALPHX_AGENT_TYPE) {
    safeError(`[RalphX MCP] Agent type from env: ${AGENT_TYPE}`);
}
else {
    safeError(`[RalphX MCP] Agent type unknown (no CLI arg or env var)`);
}
// Runtime scope for task/project/context enforcement.
const RALPHX_TASK_ID = runtimeContext.taskId;
const RALPHX_PROJECT_ID = runtimeContext.projectId;
const RALPHX_WORKING_DIRECTORY = runtimeContext.workingDirectory;
const RALPHX_CONTEXT_TYPE = runtimeContext.contextType;
const RALPHX_CONTEXT_ID = runtimeContext.contextId;
async function resolveVerifierParentSessionId(rawSessionId, toolName) {
    if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
        return rawSessionId;
    }
    if (AGENT_TYPE === "ralphx-plan-verifier" && typeof RALPHX_CONTEXT_ID === "string" && RALPHX_CONTEXT_ID.length > 0) {
        const parentContext = await callTauriGet(`parent_session_context/${RALPHX_CONTEXT_ID}`);
        if (typeof parentContext.parent_session?.id === "string" && parentContext.parent_session.id.length > 0) {
            return parentContext.parent_session.id;
        }
    }
    throw new Error(`${toolName} requires session_id unless it is called from an active ralphx-plan-verifier child session with a resolvable parent ideation session.`);
}
function resolveContextSessionId(rawSessionId, toolName) {
    if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
        return rawSessionId;
    }
    if (typeof RALPHX_CONTEXT_ID === "string" && RALPHX_CONTEXT_ID.trim().length > 0) {
        return RALPHX_CONTEXT_ID;
    }
    throw new Error(`${toolName} requires session_id unless the current RalphX session context is available for automatic injection.`);
}
/**
 * Validate that a tool call's task_id parameter matches the assigned task
 * @param toolName - Name of the tool being called
 * @param args - Arguments passed to the tool
 * @returns Error message if validation fails, null if validation passes or not applicable
 *
 * Test Cases:
 * 1. Non-scoped tool (get_artifact) => returns null (no validation)
 * 2. Scoped tool, no RALPHX_TASK_ID set => returns null (backward compat)
 * 3. Scoped tool, matching task_id => returns null (validation passed)
 * 4. Scoped tool, mismatched task_id => returns error message
 */
function validateTaskScope(toolName, args) {
    // Only validate tools that have task_id parameter directly
    // Note: start_step, complete_step, skip_step, fail_step take step_id, not task_id
    // The backend validates step ownership - we can't do it here without a DB lookup
    const taskScopedTools = [
        "complete_review",
        "approve_task",
        "request_task_changes",
        "update_task",
        "add_task_note",
        "get_task_details",
        "get_task_context",
        "get_review_notes",
        "get_task_steps",
        "add_step",
        "get_step_progress",
        // Merge tools (merger agent)
        "report_conflict",
        "report_incomplete",
        "complete_merge",
        "get_merge_target",
        // Issue tools (worker + reviewer agents)
        "get_task_issues",
        "get_issue_progress",
        // Execution complete (worker agent)
        "execution_complete",
    ];
    if (!taskScopedTools.includes(toolName)) {
        return null; // No validation needed
    }
    if (!RALPHX_TASK_ID) {
        return null; // No task scope set, allow (backward compatibility)
    }
    const providedTaskId = args.task_id;
    if (providedTaskId !== RALPHX_TASK_ID) {
        return `ERROR: Task scope violation.\n\nYou are assigned to task "${RALPHX_TASK_ID}" but attempted to modify task "${providedTaskId}".\n\nYour assigned task details:\n- Task ID: ${RALPHX_TASK_ID}\n- You should only call ${toolName} with this task_id.\n\nPlease correct your tool call and try again.`;
    }
    return null; // Validation passed
}
/**
 * Validate that a tool call's project_id parameter matches the assigned project
 * @param toolName - Name of the tool being called
 * @param args - Arguments passed to the tool
 * @returns Error message if validation fails, null if validation passes or not applicable
 */
function validateProjectScope(toolName, args) {
    const projectScopedTools = [
        "get_project_analysis",
        "save_project_analysis",
        // Memory write tools (memory agents only)
        // Note: mark_memory_obsolete excluded - uses memory_id lookup for implicit project validation
        "upsert_memories",
        "refresh_memory_rule_index",
        "ingest_rule_file",
        "rebuild_archive_snapshots",
    ];
    if (!projectScopedTools.includes(toolName)) {
        return null;
    }
    if (!RALPHX_PROJECT_ID) {
        return null; // No project scope set, allow (backward compatibility)
    }
    const providedProjectId = args.project_id;
    if (providedProjectId !== RALPHX_PROJECT_ID) {
        return `ERROR: Project scope violation.\n\nYou are assigned to project "${RALPHX_PROJECT_ID}" but attempted to access project "${providedProjectId}".\n\nPlease correct your tool call and try again.`;
    }
    return null;
}
/**
 * Create and configure the MCP server
 */
const server = new Server({
    name: "ralphx",
    version: "1.0.0",
}, {
    capabilities: {
        tools: {},
    },
});
/**
 * List available tools (filtered by agent type)
 * Note: permission_request tool is always included (not scoped by agent type)
 */
server.setRequestHandler(ListToolsRequestSchema, async () => {
    // Parse once — reuse for logging and to avoid a redundant argv scan inside getAllowedToolNames()
    const cliToolsArg = parseAllowedToolsFromArgs();
    const tools = getFilteredTools();
    // Always include permission_request tool (not scoped by agent type)
    const allTools = [...tools, permissionRequestTool];
    // Log tool scoping for debugging
    if (cliToolsArg !== undefined) {
        safeError(`[RalphX MCP] Tools from --allowed-tools: ${cliToolsArg.length > 0 ? cliToolsArg.join(", ") : "none (explicit __NONE__)"}`);
    }
    const toolNames = tools.map((t) => t.name);
    safeError(`[RalphX MCP] Agent type: ${AGENT_TYPE}, Tools: ${toolNames.length > 0 ? toolNames.join(", ") : "none"} + permission_request`);
    safeTrace("tools.list", {
        agent_type: AGENT_TYPE,
        tools: toolNames,
        includes_permission_request: true,
    });
    return { tools: allTools };
});
/**
 * Execute tool calls (with authorization check)
 */
server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    safeTrace("tool.request", { name, args });
    // Special handling for permission_request tool (always allowed, not scoped by agent type)
    if (name === "permission_request") {
        try {
            const result = await handlePermissionRequest(args);
            safeTrace("tool.success", {
                name,
                result: summarizeResult(result),
            });
            return result;
        }
        catch (error) {
            safeTrace("tool.error", {
                name,
                error: error instanceof Error ? error.message : String(error),
            });
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: JSON.stringify({ behavior: "deny", message }) }],
            };
        }
    }
    if (FILESYSTEM_TOOL_NAMES.includes(name)) {
        if (!isToolAllowed(name)) {
            return {
                content: [
                    {
                        type: "text",
                        text: `ERROR: Tool "${name}" is not available for agent type "${AGENT_TYPE}".`,
                    },
                ],
                isError: true,
            };
        }
        try {
            const result = await handleFilesystemToolCall(name, args);
            safeTrace("tool.success", {
                name,
                result: summarizeResult(result),
            });
            return result;
        }
        catch (error) {
            safeTrace("tool.error", {
                name,
                error: error instanceof Error ? error.message : String(error),
            });
            return formatFilesystemToolError(error);
        }
    }
    // Special handling for ask_user_question (register + long-poll, like permission_request)
    if (name === "ask_user_question") {
        // Still check authorization (must be in agent's allowlist)
        if (!isToolAllowed(name)) {
            return {
                content: [
                    {
                        type: "text",
                        text: `ERROR: Tool "${name}" is not available for agent type "${AGENT_TYPE}".`,
                    },
                ],
                isError: true,
            };
        }
        try {
            const result = await handleAskUserQuestion(args);
            safeTrace("tool.success", {
                name,
                result: summarizeResult(result),
            });
            return result;
        }
        catch (error) {
            safeTrace("tool.error", {
                name,
                error: error instanceof Error ? error.message : String(error),
            });
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: `ERROR: Unexpected error: ${message}` }],
                isError: true,
            };
        }
    }
    // Special handling for request_team_plan (two-phase: register POST + long-poll GET)
    if (name === "request_team_plan") {
        // Still check authorization (must be in agent's allowlist)
        if (!isToolAllowed(name)) {
            return {
                content: [
                    {
                        type: "text",
                        text: `ERROR: Tool "${name}" is not available for agent type "${AGENT_TYPE}".`,
                    },
                ],
                isError: true,
            };
        }
        const leadSessionId = globalThis.process.env.RALPHX_LEAD_SESSION_ID;
        try {
            const result = await handleRequestTeamPlan(args, RALPHX_CONTEXT_TYPE ?? "ideation", RALPHX_CONTEXT_ID ?? "", leadSessionId);
            safeTrace("tool.success", {
                name,
                result: summarizeResult(result),
            });
            return result;
        }
        catch (error) {
            safeTrace("tool.error", {
                name,
                error: error instanceof Error ? error.message : String(error),
            });
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: `ERROR: Unexpected error: ${message}` }],
                isError: true,
            };
        }
    }
    // Authorization check (defense in depth)
    if (!isToolAllowed(name)) {
        const allowedNames = getAllowedToolNames();
        const errorMessage = allowedNames.length > 0
            ? `Tool "${name}" is not available for agent type "${AGENT_TYPE}". Allowed tools: ${allowedNames.join(", ")}`
            : `Agent type "${AGENT_TYPE}" has no MCP tools available. This agent should use filesystem tools (Read, Grep, Glob, Bash, Edit, Write) instead.`;
        safeError(`[RalphX MCP] Unauthorized tool call: ${name}`);
        safeTrace("tool.denied", { name, reason: "unauthorized" });
        return {
            content: [
                {
                    type: "text",
                    text: `ERROR: ${errorMessage}`,
                },
            ],
            isError: true,
        };
    }
    // Task scope validation
    const scopeError = validateTaskScope(name, args || {});
    if (scopeError) {
        safeError(`[RalphX MCP] Task scope violation: ${name}`);
        safeTrace("tool.denied", { name, reason: "task_scope_violation" });
        return {
            content: [
                {
                    type: "text",
                    text: scopeError,
                },
            ],
            isError: true,
        };
    }
    // Project scope validation
    const projectScopeError = validateProjectScope(name, args || {});
    if (projectScopeError) {
        safeError(`[RalphX MCP] Project scope violation: ${name}`);
        safeTrace("tool.denied", { name, reason: "project_scope_violation" });
        return {
            content: [
                {
                    type: "text",
                    text: projectScopeError,
                },
            ],
            isError: true,
        };
    }
    try {
        // Forward to Tauri backend
        safeError(`[RalphX MCP] Calling Tauri: ${name} with args:`, JSON.stringify(args));
        safeTrace("tool.dispatch", { name });
        let result;
        // Special handling for GET endpoints with path parameters
        if (name === "get_task_context") {
            const { task_id } = args;
            result = await callTauriGet(`task_context/${task_id}`);
        }
        else if (name === "get_artifact") {
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}`);
        }
        else if (name === "get_artifact_version") {
            const { artifact_id, version } = args;
            result = await callTauriGet(`artifact/${artifact_id}/version/${version}`);
        }
        else if (name === "get_related_artifacts") {
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}/related`);
        }
        else if (name === "get_plan_artifact") {
            // DEPRECATED: alias for backward compat — routes to get_artifact handler
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}`);
        }
        else if (name === "get_session_plan") {
            // Also handle get_session_plan as GET
            const { session_id } = args;
            result = await callTauriGet(`get_session_plan/${session_id}`);
        }
        else if (name === "get_plan_verification") {
            // GET /api/ideation/sessions/:id/verification
            const session_id = await resolveVerifierParentSessionId(args.session_id, "get_plan_verification");
            result = await callTauriGet(`ideation/sessions/${session_id}/verification`);
        }
        else if (name === "report_verification_round") {
            // POST /api/ideation/sessions/:id/verification (verifier-friendly alias)
            const { session_id: raw_session_id, ...body } = args;
            const session_id = await resolveVerifierParentSessionId(raw_session_id, "report_verification_round");
            result = await callTauri(`ideation/sessions/${session_id}/verification`, {
                ...body,
                status: "reviewing",
                in_progress: true,
            });
        }
        else if (name === "assess_verification_round") {
            const { session_id, delegates, created_after, rescue_budget_exhausted = false, include_messages = true, message_limit = 5, } = args;
            const findingMatches = await loadVerificationFindingsByCritic({
                sessionId: session_id,
                critics: Array.from(new Set(delegates
                    .map((delegate) => delegate.label?.trim().toLowerCase())
                    .filter((label) => Boolean(label)))),
                createdAfter: created_after,
            });
            const findingByCritic = new Map(findingMatches.map((match) => [match.critic, match]));
            const artifacts_by_prefix = Array.from(new Set(delegates.map((delegate) => delegate.artifact_prefix))).map((prefix) => {
                const delegate = delegates.find((entry) => entry.artifact_prefix === prefix);
                const critic = delegate?.label?.trim().toLowerCase();
                const findingMatch = critic ? findingByCritic.get(critic) : undefined;
                return findingMatch?.found && findingMatch.finding
                    ? {
                        prefix,
                        found: true,
                        total_matches: findingMatch.total_matches,
                        artifact: {
                            id: findingMatch.finding.artifact_id,
                            name: findingMatch.finding.title,
                            created_at: findingMatch.finding.created_at,
                        },
                    }
                    : {
                        prefix,
                        found: false,
                        total_matches: 0,
                    };
            });
            const delegateSnapshots = await Promise.all(delegates.map(async (delegate) => {
                try {
                    return await callTauri("coordination/delegate/wait", {
                        job_id: delegate.job_id,
                        include_delegated_status: true,
                        include_messages,
                        message_limit,
                    });
                }
                catch (error) {
                    const errorMessage = error instanceof Error ? error.message : String(error);
                    return {
                        job_id: delegate.job_id,
                        status: "failed",
                        error: errorMessage,
                    };
                }
            }));
            result = {
                session_id,
                created_after: created_after ?? null,
                rescue_budget_exhausted,
                verification_findings: findingMatches
                    .filter((match) => match.found && match.finding)
                    .map((match) => match.finding),
                ...assessVerificationRound({
                    delegates,
                    artifactsByPrefix: artifacts_by_prefix,
                    delegateSnapshots,
                    rescueBudgetExhausted: rescue_budget_exhausted,
                }),
            };
        }
        else if (name === "run_verification_enrichment") {
            const { session_id: raw_session_id, disabled_specialists = [], include_full_content = true, include_messages = true, message_limit = 5, max_wait_ms = 4000, poll_interval_ms = 500, } = args;
            const session_id = await resolveVerifierParentSessionId(raw_session_id, "run_verification_enrichment");
            result = await runVerificationEnrichmentPass({
                loadPlanSnapshot: loadVerificationPlanSnapshot,
                startDelegate: async ({ agentName, parentSessionId, prompt, delegatedSessionId }) => {
                    const launched = await startManagedVerificationDelegate({
                        agentName,
                        parentSessionId,
                        prompt,
                        delegatedSessionId,
                    });
                    return {
                        job_id: launched.job_id,
                        delegated_session_id: launched.delegated_session_id,
                        agent_name: agentName,
                        artifact_prefix: "",
                        required: false,
                    };
                },
                awaitOptionalDelegates: awaitOptionalVerificationDelegates,
                runRequiredCriticRound: runRequiredVerificationCriticRound,
            }, {
                sessionId: session_id,
                disabledSpecialists: new Set((disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
                includeFullContent: include_full_content !== false,
                includeMessages: include_messages !== false,
                messageLimit: Math.min(Math.max(message_limit ?? 5, 1), 50),
                maxWaitMs: Math.min(Math.max(max_wait_ms ?? 4000, 0), 30000),
                pollIntervalMs: Math.max(poll_interval_ms ?? 500, 100),
            });
        }
        else if (name === "run_verification_round") {
            const { session_id: raw_session_id, round, disabled_specialists = [], include_full_content = true, include_messages = true, message_limit = 5, max_wait_ms = 8000, optional_wait_ms = 4000, poll_interval_ms = 750, } = args;
            const session_id = await resolveVerifierParentSessionId(raw_session_id, "run_verification_round");
            result = await runVerificationRoundPass({
                loadPlanSnapshot: loadVerificationPlanSnapshot,
                startDelegate: async ({ agentName, parentSessionId, prompt, delegatedSessionId }) => {
                    const launched = await startManagedVerificationDelegate({
                        agentName,
                        parentSessionId,
                        prompt,
                        delegatedSessionId,
                    });
                    return {
                        job_id: launched.job_id,
                        delegated_session_id: launched.delegated_session_id,
                        agent_name: agentName,
                        artifact_prefix: "",
                        required: false,
                    };
                },
                awaitOptionalDelegates: awaitOptionalVerificationDelegates,
                runRequiredCriticRound: runRequiredVerificationCriticRound,
            }, {
                sessionId: session_id,
                round,
                disabledSpecialists: new Set((disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
                includeFullContent: include_full_content !== false,
                includeMessages: include_messages !== false,
                messageLimit: Math.min(Math.max(message_limit ?? 5, 1), 50),
                maxWaitMs: Math.min(Math.max(max_wait_ms ?? 8000, 0), 30000),
                optionalWaitMs: Math.min(Math.max(optional_wait_ms ?? 4000, 0), 30000),
                pollIntervalMs: Math.max(poll_interval_ms ?? 750, 100),
            });
        }
        else if (name === "run_required_verification_critic_round") {
            const { session_id: raw_session_id, round, include_full_content = true, include_messages = true, message_limit = 5, max_wait_ms = 8000, poll_interval_ms = 750, } = args;
            const session_id = await resolveVerifierParentSessionId(raw_session_id, "run_required_verification_critic_round");
            result = await runRequiredVerificationCriticRound({
                sessionId: session_id,
                round,
                includeFullContent: include_full_content !== false,
                includeMessages: include_messages !== false,
                messageLimit: Math.min(Math.max(message_limit ?? 5, 1), 50),
                maxWaitMs: Math.min(Math.max(max_wait_ms ?? 8000, 0), 30000),
                pollIntervalMs: Math.max(poll_interval_ms ?? 750, 100),
            });
        }
        else if (name === "await_verification_round_settlement") {
            result = await awaitVerificationRoundSettlement({
                ...args,
                session_id: await resolveVerifierParentSessionId(args.session_id, "await_verification_round_settlement"),
            });
        }
        else if (name === "complete_plan_verification") {
            // POST /api/ideation/sessions/:id/verification (verifier-friendly terminal alias)
            const { session_id: raw_session_id, required_delegates, created_after, rescue_budget_exhausted = false, include_full_content = true, include_messages = true, message_limit = 5, max_wait_ms = 8000, poll_interval_ms = 750, ...body } = args;
            const session_id = await resolveVerifierParentSessionId(raw_session_id, "complete_plan_verification");
            const isVerifierRoundTerminalUpdate = AGENT_TYPE === "ralphx-plan-verifier" &&
                typeof body.round === "number" &&
                body.status !== "skipped" &&
                body.convergence_reason !== "user_stopped" &&
                body.convergence_reason !== "user_skipped" &&
                body.convergence_reason !== "user_reverted";
            if (body.status === "reviewing") {
                throw new Error("complete_plan_verification is terminal-only. Use verified or needs_revision here, not reviewing.");
            }
            if (isVerifierRoundTerminalUpdate) {
                if (!Array.isArray(required_delegates) || required_delegates.length === 0) {
                    throw new Error("Verifier round terminal completion requires required_delegates so the settlement barrier cannot be bypassed.");
                }
                if (!created_after) {
                    throw new Error("Verifier round terminal completion requires created_after so settlement is scoped to the active round window.");
                }
            }
            let settlement;
            if (Array.isArray(required_delegates) && required_delegates.length > 0) {
                result = await completePlanVerificationWithSettlement({
                    sessionId: session_id,
                    body,
                    requiredDelegates: required_delegates,
                    createdAfter: created_after,
                    rescueBudgetExhausted: rescue_budget_exhausted,
                    includeFullContent: include_full_content,
                    includeMessages: include_messages,
                    messageLimit: message_limit,
                    maxWaitMs: max_wait_ms,
                    pollIntervalMs: poll_interval_ms,
                    isVerifierRoundTerminalUpdate,
                    awaitVerificationRoundSettlement,
                    callInfraFailure: async ({ generation, convergence_reason, round }) => (await callTauri(`ideation/sessions/${session_id}/verification/infra-failure`, {
                        generation,
                        convergence_reason: convergence_reason ?? "agent_error",
                        round,
                    })),
                    callCompletion: async (completionBody) => await callTauri(`ideation/sessions/${session_id}/verification`, completionBody),
                });
            }
            else {
                result = await callTauri(`ideation/sessions/${session_id}/verification`, {
                    ...body,
                    in_progress: false,
                });
            }
        }
        else if (name === "update_plan_verification") {
            // POST /api/ideation/sessions/:id/verification
            const { session_id, ...body } = args;
            result = await callTauri(`ideation/sessions/${session_id}/verification`, body);
        }
        else if (name === "revert_and_skip") {
            // POST /api/ideation/sessions/:id/revert-and-skip
            const { session_id, plan_version_to_restore } = args;
            result = await callTauri(`ideation/sessions/${session_id}/revert-and-skip`, { plan_version_to_restore });
        }
        else if (name === "stop_verification") {
            // POST /api/ideation/sessions/:id/stop-verification
            const { session_id } = args;
            result = await callTauri(`ideation/sessions/${session_id}/stop-verification`, {});
        }
        else if (name === "get_task_steps") {
            // GET /api/task_steps/:task_id
            const { task_id } = args;
            result = await callTauriGet(`task_steps/${task_id}`);
        }
        else if (name === "get_step_progress") {
            // GET /api/step_progress/:task_id
            const { task_id } = args;
            result = await callTauriGet(`step_progress/${task_id}`);
        }
        else if (name === "get_step_context") {
            // GET /api/step_context/:step_id
            const { step_id } = args;
            result = await callTauriGet(`step_context/${step_id}`);
        }
        else if (name === "get_sub_steps") {
            // GET /api/sub_steps/:parent_step_id
            const { parent_step_id } = args;
            result = await callTauriGet(`sub_steps/${parent_step_id}`);
        }
        else if (name === "get_review_notes") {
            // GET /api/review_notes/:task_id
            const { task_id } = args;
            result = await callTauriGet(`review_notes/${task_id}`);
        }
        else if (name === "list_session_proposals") {
            // GET /api/list_session_proposals/:session_id
            const { session_id } = args;
            result = await callTauriGet(`list_session_proposals/${session_id}`);
        }
        else if (name === "get_proposal") {
            // GET /api/proposal/:proposal_id
            const { proposal_id } = args;
            result = await callTauriGet(`proposal/${proposal_id}`);
        }
        else if (name === "analyze_session_dependencies") {
            // GET /api/analyze_dependencies/:session_id
            const { session_id } = args;
            result = await callTauriGet(`analyze_dependencies/${session_id}`);
        }
        else if (name === "complete_merge") {
            // POST /api/git/tasks/:task_id/complete-merge
            const { task_id, commit_sha } = args;
            result = await callTauri(`git/tasks/${task_id}/complete-merge`, { commit_sha });
        }
        else if (name === "report_conflict") {
            // POST /api/git/tasks/:task_id/report-conflict
            const { task_id, conflict_files, reason } = args;
            result = await callTauri(`git/tasks/${task_id}/report-conflict`, { conflict_files, reason });
        }
        else if (name === "report_incomplete") {
            // POST /api/git/tasks/:task_id/report-incomplete
            const { task_id, reason, diagnostic_info } = args;
            result = await callTauri(`git/tasks/${task_id}/report-incomplete`, { reason, diagnostic_info });
        }
        else if (name === "get_merge_target") {
            const { task_id } = args;
            result = await callTauriGet(`git/tasks/${task_id}/merge-target`);
        }
        else if (name === "get_task_issues") {
            // GET /api/task_issues/:task_id?status=<filter>
            const { task_id, status_filter } = args;
            const query = status_filter ? `?status=${status_filter}` : "";
            result = await callTauriGet(`task_issues/${task_id}${query}`);
        }
        else if (name === "get_issue_progress") {
            // GET /api/issue_progress/:task_id
            const { task_id } = args;
            result = await callTauriGet(`issue_progress/${task_id}`);
        }
        else if (name === "mark_issue_in_progress") {
            // POST /api/mark_issue_in_progress
            const { issue_id } = args;
            result = await callTauri("mark_issue_in_progress", { issue_id });
        }
        else if (name === "mark_issue_addressed") {
            // POST /api/mark_issue_addressed
            const { issue_id, resolution_notes, attempt_number } = args;
            result = await callTauri("mark_issue_addressed", { issue_id, resolution_notes, attempt_number });
        }
        else if (name === "create_child_session") {
            // POST /api/create_child_session
            const { parent_session_id, title, description, inherit_context, initial_prompt, team_mode, team_config, purpose } = args;
            // Propagate external trigger context from the spawning process env var.
            // RALPHX_IS_EXTERNAL_TRIGGER=1 is set by the backend when the agent was spawned
            // in response to an external MCP message (is_external_mcp=true).
            const is_external_trigger = process.env.RALPHX_IS_EXTERNAL_TRIGGER === "1";
            result = await callTauri("create_child_session", { parent_session_id, title, description, inherit_context, initial_prompt, team_mode, team_config, purpose, is_external_trigger });
        }
        else if (name === "create_followup_session") {
            // POST /api/create_child_session with first-class execution/review provenance
            const { source_ideation_session_id, title, description, inherit_context, initial_prompt, source_task_id, source_context_type, source_context_id, spawn_reason, blocker_fingerprint, } = args;
            let resolvedParentSessionId = source_ideation_session_id;
            let resolvedBlockerFingerprint = blocker_fingerprint;
            if (!resolvedParentSessionId && source_task_id) {
                const taskContext = await callTauriGet(`task_context/${source_task_id}`);
                resolvedParentSessionId = taskContext.task?.ideation_session_id ?? undefined;
                if (!resolvedBlockerFingerprint && spawn_reason === "out_of_scope_failure") {
                    resolvedBlockerFingerprint = taskContext.out_of_scope_blocker_fingerprint ?? undefined;
                }
            }
            if (!resolvedParentSessionId) {
                throw new Error("create_followup_session requires either source_ideation_session_id or a source_task_id that belongs to an ideation-backed task");
            }
            result = await callTauri("create_child_session", {
                parent_session_id: resolvedParentSessionId,
                title,
                description,
                inherit_context,
                initial_prompt,
                source_task_id,
                source_context_type,
                source_context_id,
                spawn_reason,
                blocker_fingerprint: resolvedBlockerFingerprint,
            });
        }
        else if (name === "get_parent_session_context") {
            // GET /api/parent_session_context/:session_id
            const { session_id } = args;
            result = await callTauriGet(`parent_session_context/${session_id}`);
        }
        else if (name === "delegate_start") {
            result = await callTauri("coordination/delegate/start", {
                ...args,
                caller_agent_name: AGENT_TYPE,
                caller_context_type: RALPHX_CONTEXT_TYPE,
                caller_context_id: RALPHX_CONTEXT_ID,
            });
        }
        else if (name === "delegate_wait") {
            result = await callTauri("coordination/delegate/wait", args);
        }
        else if (name === "delegate_cancel") {
            result = await callTauri("coordination/delegate/cancel", args);
        }
        else if (name === "get_project_analysis") {
            // GET /api/projects/:project_id/analysis?task_id=
            const { project_id, task_id } = args;
            const query = task_id ? `?task_id=${task_id}` : "";
            result = await callTauriGet(`projects/${project_id}/analysis${query}`);
        }
        else if (name === "save_project_analysis") {
            // POST /api/projects/:project_id/analysis
            const { project_id, entries } = args;
            result = await callTauri(`projects/${project_id}/analysis`, { entries });
        }
        else if (name === "request_teammate_spawn") {
            // POST /api/team/spawn
            const { role, prompt, model, tools, mcp_tools, preset } = args;
            result = await callTauri("team/spawn", { role, prompt, model, tools, mcp_tools, preset });
        }
        else if (name === "create_team_artifact") {
            // POST /api/team/artifact
            const { session_id, title, content, artifact_type, related_artifact_id } = args;
            result = await callTauri("team/artifact", {
                session_id,
                title,
                content,
                artifact_type,
                related_artifact_id,
            });
        }
        else if (name === "publish_verification_finding") {
            const { session_id, critic, round, status, coverage, summary, gaps, title_suffix, } = args;
            result = await callTauri("team/verification_finding", {
                session_id: resolveContextSessionId(session_id, "publish_verification_finding"),
                critic,
                round,
                status,
                coverage,
                summary,
                gaps,
                title_suffix,
            });
        }
        else if (name === "get_team_artifacts") {
            // GET /api/team/artifacts/:session_id
            const { session_id } = args;
            result = await callTauriGet(`team/artifacts/${session_id}`);
        }
        else if (name === "get_verification_round_artifacts") {
            const { session_id, prefixes, created_after, include_full_content = true, } = args;
            const teamArtifacts = await callTauriGet(`team/artifacts/${session_id}`);
            const matches = selectLatestArtifactsByPrefix(teamArtifacts.artifacts ?? [], prefixes, created_after);
            const artifacts_by_prefix = await Promise.all(matches.map(async (match) => {
                if (!match.artifact) {
                    return match;
                }
                if (!include_full_content) {
                    return match;
                }
                const fullArtifact = await callTauriGet(`artifact/${match.artifact.id}`);
                return {
                    ...match,
                    artifact: {
                        ...match.artifact,
                        content: fullArtifact.content ?? "",
                    },
                };
            }));
            result = {
                session_id,
                created_after: created_after ?? null,
                prefixes,
                artifacts_by_prefix,
            };
        }
        else if (name === "get_team_session_state") {
            // GET /api/team/session_state/:session_id
            const { session_id } = args;
            result = await callTauriGet(`team/session_state/${session_id}`);
        }
        else if (name === "save_team_session_state") {
            // POST /api/team/session_state
            const { session_id, team_composition, phase, artifact_ids } = args;
            result = await callTauri("team/session_state", {
                session_id,
                team_composition,
                phase,
                artifact_ids,
            });
        }
        else if (name === "execution_complete") {
            // POST /api/execution/tasks/:task_id/complete
            const { task_id, summary, test_result } = args;
            const body = { summary: summary || "" };
            if (test_result) {
                body.testResult = {
                    testsRan: test_result.tests_ran,
                    testsPassed: test_result.tests_passed,
                    testSummary: test_result.test_summary,
                };
            }
            result = await callTauri(`execution/tasks/${task_id}/complete`, body);
        }
        else if (name === "list_projects") {
            // GET /api/internal/projects
            result = await callTauriGet("internal/projects");
        }
        else if (name === "create_cross_project_session") {
            // POST /api/internal/cross_project/create_session
            const { target_project_path, source_session_id, title } = args;
            result = await callTauri("internal/cross_project/create_session", {
                targetProjectPath: target_project_path,
                sourceSessionId: source_session_id,
                title,
            });
        }
        else if (name === "migrate_proposals") {
            // POST /api/internal/cross_project/migrate_proposals
            const { source_session_id, target_session_id, proposal_ids, target_project_filter } = args;
            result = await callTauri("internal/cross_project/migrate_proposals", {
                sourceSessionId: source_session_id,
                targetSessionId: target_session_id,
                proposalIds: proposal_ids,
                targetProjectFilter: target_project_filter,
            });
        }
        else if (name === "cross_project_guide") {
            // MCP-server-only: analyze plan for cross-project paths and return guidance
            const { session_id, plan_content } = args;
            let planText = plan_content ?? "";
            let projectWorkingDir = null;
            // If session_id provided, fetch plan content via get_session_plan
            if (session_id && !planText) {
                try {
                    const planData = await callTauriGet(`get_session_plan/${session_id}`);
                    planText = planData?.content ?? "";
                    projectWorkingDir = planData?.project_working_directory ?? null;
                }
                catch (err) {
                    planText = "";
                    safeError(`[RalphX MCP] cross_project_guide: failed to fetch plan for session ${session_id}:`, err);
                }
            }
            // Strip code blocks before path scanning to avoid false positives on code snippets
            const scanText = stripMarkdownCodeBlocks(planText);
            // Heuristic: detect cross-project paths (absolute paths, ../relative paths, paths with project-like names)
            const crossProjectPatterns = [
                /(?:^|\s|["'`])(\/(home|Users|workspace|projects|srv|opt)\/[^\s"'`]+)/gm,
                /(?:^|\s|["'`])(\.\.\/[^\s"'`]+)/gm,
                /(?:target[_-]?project[_-]?path|project[_-]?path|working[_-]?directory)[:\s]+["']?([^\s"'`,\n]+)/gim,
            ];
            const rawDetectedPaths = [];
            for (const pattern of crossProjectPatterns) {
                const matches = [...scanText.matchAll(pattern)];
                for (const m of matches) {
                    const p = (m[1] || m[0]).trim().replace(/^["'`]|["'`]$/g, "");
                    if (p && !rawDetectedPaths.includes(p)) {
                        rawDetectedPaths.push(p);
                    }
                }
            }
            const detectedPaths = filterCrossProjectPaths(rawDetectedPaths, projectWorkingDir);
            const crossProjectKeywordRegex = new RegExp(CROSS_PROJECT_KEYWORDS.join("|"), "i");
            const hasCrossProjectContent = detectedPaths.length > 0 ||
                crossProjectKeywordRegex.test(planText);
            const analysisResult = {
                has_cross_project_paths: hasCrossProjectContent,
                detected_paths: detectedPaths,
                guidance: hasCrossProjectContent
                    ? {
                        summary: "This plan contains cross-project references. Follow these steps to orchestrate multi-project execution:",
                        steps: [
                            "1. Call list_projects to discover existing RalphX projects and their filesystem paths.",
                            "2. For each target project, call create_cross_project_session({ target_project_path, source_session_id }) to create a new session with the inherited plan.",
                            "3. In each target session, use create_task_proposal to create proposals specific to that project's scope.",
                            "4. Call accept_plan_and_schedule (or equivalent) in each target session to push tasks to kanban.",
                        ],
                        notes: [
                            "The target project is auto-created if no RalphX project exists at the given path.",
                            "The inherited plan is read-only in the target session. Call create_plan_artifact to create a writable copy if modifications are needed.",
                            "The inherited plan status is set to 'imported_verified' — no re-verification is triggered.",
                        ],
                        detected_paths: detectedPaths,
                    }
                    : {
                        summary: "No cross-project paths detected in this plan.",
                        steps: [],
                        notes: [
                            "If you believe there are cross-project references, try providing the plan_content directly or check the session_id.",
                        ],
                        detected_paths: [],
                    },
            };
            if (session_id) {
                try {
                    await callTauri(`internal/sessions/${session_id}/cross_project_check`, {});
                    result = { ...analysisResult, gate_status: "set" };
                }
                catch (err) {
                    const errMsg = err instanceof Error ? err.message : String(err);
                    safeError(`[RalphX MCP] cross_project_guide: failed to set gate for session ${session_id}:`, err);
                    result = {
                        ...analysisResult,
                        gate_status: "backend_unavailable",
                        gate_error: `Backend call failed: ${errMsg}`,
                    };
                }
            }
            else {
                result = {
                    ...analysisResult,
                    gate_status: "no_session_id",
                    gate_message: "Provide session_id to set the cross-project gate and unlock proposal creation",
                };
            }
        }
        else if (name === "get_child_session_status") {
            // GET /api/ideation/sessions/:id/child-status
            const { session_id, include_recent_messages, message_limit } = args;
            const params = new URLSearchParams();
            if (include_recent_messages)
                params.set("include_messages", "true");
            if (message_limit)
                params.set("message_limit", String(message_limit));
            const query = params.toString() ? `?${params}` : "";
            result = await callTauriGet(`ideation/sessions/${session_id}/child-status${query}`);
        }
        else if (name === "send_ideation_session_message") {
            // POST /api/ideation/sessions/:id/message
            const { session_id, message } = args;
            result = await callTauri(`ideation/sessions/${session_id}/message`, { message });
        }
        else if (name === "get_acceptance_status") {
            // GET /api/ideation/sessions/:id/acceptance-status
            const { session_id } = args;
            result = await callTauriGet(`ideation/sessions/${session_id}/acceptance-status`);
        }
        else if (name === "get_pending_confirmations") {
            // GET /api/ideation/pending-confirmations?project_id=xxx
            const projectId = RALPHX_PROJECT_ID;
            if (!projectId) {
                throw new Error("RALPHX_PROJECT_ID is not set — cannot query pending confirmations");
            }
            result = await callTauriGet(`ideation/pending-confirmations?project_id=${encodeURIComponent(projectId)}`);
        }
        else if (name === "get_verification_confirmation_status") {
            // GET /api/verification/confirmation-status/{session_id}
            const { session_id } = args;
            result = await callTauriGet(`verification/confirmation-status/${encodeURIComponent(session_id)}`);
        }
        else if (name === "delete_task_proposal") {
            // Alias for archive_task_proposal — no /api/delete_task_proposal route exists in backend
            const { proposal_id } = args;
            result = await callTauri("archive_task_proposal", { proposal_id });
        }
        else {
            // Default: POST request
            result = await callTauri(name, args || {});
        }
        safeError(`[RalphX MCP] Success: ${name}`);
        safeTrace("tool.success", {
            name,
            result: summarizeResult(result),
        });
        // Return result as JSON text
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(result, null, 2),
                },
            ],
        };
    }
    catch (error) {
        safeError(`[RalphX MCP] Error calling ${name}:`, error);
        safeTrace("tool.error", {
            name,
            error: error instanceof Error ? error.message : String(error),
            details: error instanceof TauriClientError ? error.details : undefined,
        });
        if (error instanceof TauriClientError) {
            return {
                content: [
                    {
                        type: "text",
                        text: formatToolErrorMessage(name, error.message, error.details),
                    },
                ],
                isError: true,
            };
        }
        return {
            content: [
                {
                    type: "text",
                    text: `ERROR: Unexpected error: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
/**
 * Start the server
 */
async function main() {
    console.error("[RalphX MCP] Starting server...");
    safeError(`[RalphX MCP] Agent type: ${AGENT_TYPE}`);
    if (RALPHX_TASK_ID) {
        safeError(`[RalphX MCP] Task scope: ${RALPHX_TASK_ID}`);
    }
    if (RALPHX_PROJECT_ID) {
        safeError(`[RalphX MCP] Project scope: ${RALPHX_PROJECT_ID}`);
    }
    if (RALPHX_WORKING_DIRECTORY) {
        safeError(`[RalphX MCP] Working directory root: ${RALPHX_WORKING_DIRECTORY}`);
    }
    safeError(`[RalphX MCP] Tauri API URL: ${process.env.TAURI_API_URL || "http://127.0.0.1:3847"}`);
    safeError(`[RalphX MCP] Trace log: ${getTraceLogPath()}`);
    safeTrace("server.start", {
        argv: process.argv.slice(2),
        tauri_api_url: process.env.TAURI_API_URL || "http://127.0.0.1:3847",
    });
    // Log all tools if in debug mode or if RALPHX_DEBUG_TOOLS is set
    if (AGENT_TYPE === "debug" || process.env.RALPHX_DEBUG_TOOLS === "1") {
        logAllTools();
    }
    // Always log available tools for this agent
    const toolsByAgent = getToolsByAgent();
    const agentTools = toolsByAgent[AGENT_TYPE] || [];
    safeError(`[RalphX MCP] Tools for ${AGENT_TYPE}: ${agentTools.length > 0 ? agentTools.join(", ") : "(none - using filesystem tools)"}`);
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("[RalphX MCP] Server running on stdio");
    safeTrace("server.ready");
}
// Global handler for unhandled promise rejections.
// Prevents secrets in HTTP error bodies or rejected promises from leaking via Node's default stderr handler.
process.on("unhandledRejection", (reason) => {
    safeError("[RalphX MCP] Unhandled rejection:", reason);
});
main().catch((error) => {
    safeError("[RalphX MCP] Fatal error:", error);
    process.exit(1);
});
//# sourceMappingURL=index.js.map