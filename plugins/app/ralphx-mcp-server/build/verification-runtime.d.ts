import { assessVerificationRound, type VerificationFindingSummary, type VerificationRoundDelegateInput, type VerificationRoundDelegateSnapshot } from "./verification-round-assessment.js";
import { type RequiredCriticRoundResult, type VerificationPlanSnapshot } from "./verification-orchestration.js";
export type TeamArtifactSummary = {
    id: string;
    name: string;
    artifact_type: string;
    version: number;
    content_preview: string;
    created_at: string;
    author_teammate?: string | null;
};
type VerificationFindingMatch = {
    critic: string;
    found: boolean;
    total_matches: number;
    finding?: VerificationFindingSummary;
};
export type VerificationSettlementArgs = {
    session_id: string;
    delegates: VerificationRoundDelegateInput[];
    created_after?: string;
    rescue_budget_exhausted?: boolean;
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    poll_interval_ms?: number;
};
export type VerificationAssessmentArgs = {
    session_id: string;
    delegates: VerificationRoundDelegateInput[];
    created_after?: string;
    rescue_budget_exhausted?: boolean;
    include_messages?: boolean;
    message_limit?: number;
};
type ManagedVerificationDelegate = VerificationRoundDelegateInput & {
    agent_name: string;
    delegated_session_id?: string;
};
type AwaitVerificationRoundSettlementResult = {
    session_id: string;
    created_after: string | null;
    rescue_budget_exhausted: boolean;
    settled: boolean;
    timed_out: boolean;
    polls_performed: number;
    max_wait_ms: number;
    poll_interval_ms: number;
    verification_findings: VerificationFindingSummary[];
    classification: "complete" | "pending" | "infra_failure";
    recommended_next_action: "continue_round_analysis" | "perform_single_rescue_or_wait" | "complete_verification_with_infra_failure";
    summary: string;
    missing_required_prefixes: string[];
    delegate_assessments: ReturnType<typeof assessVerificationRound>["delegate_assessments"];
    artifacts_by_prefix: ReturnType<typeof assessVerificationRound>["artifacts_by_prefix"];
    delegate_snapshots: VerificationRoundDelegateSnapshot[];
};
type VerificationRuntimeDeps = {
    callTauri: (endpoint: string, payload: Record<string, unknown>) => Promise<unknown>;
    callTauriGet: (endpoint: string) => Promise<unknown>;
    agentType: string;
    contextType?: string;
    contextId?: string;
};
export declare function createVerificationRuntime(deps: VerificationRuntimeDeps): {
    assessVerificationRoundState: (args: VerificationAssessmentArgs) => Promise<Record<string, unknown>>;
    getPlanVerificationForTool: (args: {
        session_id?: string;
    }) => Promise<unknown>;
    reportVerificationRoundForTool: (args: {
        session_id?: string;
        round: number;
        gaps?: unknown[];
        generation: number;
        [key: string]: unknown;
    }) => Promise<unknown>;
    completePlanVerificationForTool: (args: {
        session_id?: string;
        status: string;
        round?: number;
        gaps?: unknown[];
        convergence_reason?: string;
        generation: number;
        required_delegates?: VerificationRoundDelegateInput[];
        created_after?: string;
        rescue_budget_exhausted?: boolean;
        include_full_content?: boolean;
        include_messages?: boolean;
        message_limit?: number;
        max_wait_ms?: number;
        poll_interval_ms?: number;
    }) => Promise<unknown>;
    runVerificationEnrichment: (args: {
        session_id?: string;
        disabled_specialists?: string[];
        include_full_content?: boolean;
        include_messages?: boolean;
        message_limit?: number;
        max_wait_ms?: number;
        poll_interval_ms?: number;
    }) => Promise<unknown>;
    runVerificationRound: (args: {
        session_id?: string;
        round: number;
        disabled_specialists?: string[];
        include_full_content?: boolean;
        include_messages?: boolean;
        message_limit?: number;
        max_wait_ms?: number;
        optional_wait_ms?: number;
        poll_interval_ms?: number;
    }) => Promise<unknown>;
    runRequiredVerificationCriticRoundTool: (args: {
        session_id?: string;
        round: number;
        include_full_content?: boolean;
        include_messages?: boolean;
        message_limit?: number;
        max_wait_ms?: number;
        poll_interval_ms?: number;
    }) => Promise<RequiredCriticRoundResult>;
    awaitVerificationRoundSettlementForTool: (args: VerificationSettlementArgs) => Promise<AwaitVerificationRoundSettlementResult>;
    selectLatestArtifactsByPrefix: (artifacts: TeamArtifactSummary[], prefixes: string[], createdAfter?: string) => Array<{
        prefix: string;
        found: boolean;
        total_matches: number;
        artifact?: TeamArtifactSummary;
    }>;
    loadVerificationFindingsByCritic: (args: {
        sessionId: string;
        critics: string[];
        round?: number;
        createdAfter?: string;
    }) => Promise<VerificationFindingMatch[]>;
    loadVerificationDelegateSnapshots: (args: {
        delegates: VerificationRoundDelegateInput[];
        includeMessages: boolean;
        messageLimit: number;
    }) => Promise<(VerificationRoundDelegateSnapshot & {
        delegated_status?: unknown;
    })[]>;
    loadVerificationPlanSnapshot: (sessionId: string) => Promise<VerificationPlanSnapshot>;
    awaitOptionalVerificationDelegates: (args: {
        delegates: ManagedVerificationDelegate[];
        sessionId: string;
        createdAfter: string;
        prefixes: string[];
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<{
        created_after: string;
        polls_performed: number;
        timed_out: boolean;
        delegates: {
            job_id: string;
            artifact_prefix: string;
            label: string | undefined;
            required: boolean | undefined;
        }[];
        artifacts_by_prefix: ({
            prefix: string;
            found: boolean;
            total_matches: number;
            artifact?: TeamArtifactSummary;
        } | {
            artifact: {
                content: string;
                id: string;
                name: string;
                artifact_type: string;
                version: number;
                content_preview: string;
                created_at: string;
                author_teammate?: string | null;
            };
            prefix: string;
            found: boolean;
            total_matches: number;
        })[];
        delegate_snapshots: (VerificationRoundDelegateSnapshot & {
            delegated_status?: unknown;
        })[];
    }>;
    startManagedVerificationDelegate: (args: {
        agentName: string;
        parentSessionId: string;
        prompt: string;
        delegatedSessionId?: string;
    }) => Promise<{
        job_id: string;
        delegated_session_id?: string;
        agent_name: string;
        harness?: string;
        status?: string;
    }>;
    runRequiredVerificationCriticRound: (args: {
        sessionId: string;
        round: number;
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<RequiredCriticRoundResult>;
    awaitVerificationRoundSettlement: (args: VerificationSettlementArgs) => Promise<AwaitVerificationRoundSettlementResult>;
    resolveVerifierParentSessionId: (rawSessionId: unknown, toolName: string) => Promise<string>;
    resolveVerificationFindingSessionId: (rawSessionId: unknown, toolName: string) => Promise<string>;
    resolveContextSessionId: (rawSessionId: unknown, toolName: string) => string;
};
export {};
//# sourceMappingURL=verification-runtime.d.ts.map