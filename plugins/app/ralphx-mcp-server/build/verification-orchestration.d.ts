import { type ParsedVerificationCriticArtifact, type VerificationFindingSummary } from "./verification-round-assessment.js";
export type VerificationPlanSnapshot = {
    artifact_id?: string;
    content: string;
    project_working_directory?: string | null;
};
export type VerificationManagedDelegate = {
    job_id: string;
    delegated_session_id?: string;
    agent_name: string;
    artifact_prefix: string;
    label?: string;
    required?: boolean;
};
type ArtifactByPrefix = Array<{
    prefix: string;
    found: boolean;
    total_matches: number;
    artifact?: {
        id?: string;
        name?: string;
        created_at?: string;
        content?: string;
    };
}>;
type AwaitOptionalDelegateResult = {
    created_after: string;
    polls_performed: number;
    timed_out: boolean;
    delegates: Array<{
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }>;
    artifacts_by_prefix: ArtifactByPrefix;
    delegate_snapshots: unknown[];
};
export type RequiredCriticRoundResult = {
    session_id: string;
    round: number;
    created_after: string;
    rescue_dispatched: boolean;
    required_delegates: Array<{
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }>;
    rescue_delegates?: Array<{
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }>;
    settlement: {
        classification: "complete" | "pending" | "infra_failure";
        verification_findings?: VerificationFindingSummary[];
        artifacts_by_prefix?: ArtifactByPrefix;
        [key: string]: unknown;
    };
};
type VerificationOrchestrationDeps = {
    loadPlanSnapshot: (sessionId: string) => Promise<VerificationPlanSnapshot>;
    startDelegate: (args: {
        agentName: string;
        parentSessionId: string;
        prompt: string;
        delegatedSessionId?: string;
    }) => Promise<VerificationManagedDelegate>;
    awaitOptionalDelegates: (args: {
        delegates: VerificationManagedDelegate[];
        sessionId: string;
        createdAfter: string;
        prefixes: string[];
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<AwaitOptionalDelegateResult>;
    runRequiredCriticRound: (args: {
        sessionId: string;
        round: number;
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<RequiredCriticRoundResult>;
};
export declare function runVerificationEnrichmentPass(deps: VerificationOrchestrationDeps, args: {
    sessionId: string;
    disabledSpecialists: Set<string>;
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    pollIntervalMs: number;
}): Promise<{
    created_after: string;
    polls_performed: number;
    timed_out: boolean;
    delegates: Array<{
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }>;
    artifacts_by_prefix: ArtifactByPrefix;
    delegate_snapshots: unknown[];
    session_id: string;
    disabled_specialists: string[];
    selected_specialists: {
        name: "code-quality" | "intent";
        label: "code-quality" | "intent";
        artifact_prefix: "IntentAlignment: " | "CodeQuality: ";
        agent_name: "ralphx:ralphx-ideation-specialist-intent" | "ralphx:ralphx-ideation-specialist-code-quality";
    }[];
}>;
export declare function runVerificationRoundPass(deps: VerificationOrchestrationDeps, args: {
    sessionId: string;
    round: number;
    disabledSpecialists: Set<string>;
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    optionalWaitMs: number;
    pollIntervalMs: number;
}): Promise<{
    session_id: string;
    round: number;
    created_after: string;
    classification: "pending" | "infra_failure";
    required_delegates: {
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }[];
    rescue_delegates: {
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }[];
    required_critic_settlement: {
        [key: string]: unknown;
        classification: "complete" | "pending" | "infra_failure";
        verification_findings?: VerificationFindingSummary[];
        artifacts_by_prefix?: ArtifactByPrefix;
    };
    required_findings: ParsedVerificationCriticArtifact[];
    merged_gaps: never[];
    gap_counts: {
        critical: number;
        high: number;
        medium: number;
        low: number;
    };
    optional_specialists: {
        name: "ux" | "prompt-quality" | "pipeline-safety" | "state-machine";
        label: "ux" | "prompt-quality" | "pipeline-safety" | "state-machine";
        artifact_prefix: "UX: " | "PromptQuality: " | "PipelineSafety: " | "StateMachine: ";
        agent_name: "ralphx:ralphx-ideation-specialist-ux" | "ralphx:ralphx-ideation-specialist-prompt-quality" | "ralphx:ralphx-ideation-specialist-pipeline-safety" | "ralphx:ralphx-ideation-specialist-state-machine";
    }[];
    optional_delegates: {
        job_id: string;
        artifact_prefix: "UX: " | "PromptQuality: " | "PipelineSafety: " | "StateMachine: ";
        label: "ux" | "prompt-quality" | "pipeline-safety" | "state-machine";
        required: boolean;
    }[];
    optional_artifacts_by_prefix: never[];
    optional_delegate_snapshots: never[];
} | {
    session_id: string;
    round: number;
    created_after: string;
    classification: "complete";
    required_delegates: {
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }[];
    rescue_delegates: {
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }[];
    required_critic_settlement: {
        [key: string]: unknown;
        classification: "complete" | "pending" | "infra_failure";
        verification_findings?: VerificationFindingSummary[];
        artifacts_by_prefix?: ArtifactByPrefix;
    };
    required_findings: ParsedVerificationCriticArtifact[];
    merged_gaps: import("./verification-round-assessment.js").ParsedVerificationGap[];
    gap_counts: import("./verification-round-assessment.js").VerificationGapCounts;
    optional_specialists: {
        name: "ux" | "prompt-quality" | "pipeline-safety" | "state-machine";
        label: "ux" | "prompt-quality" | "pipeline-safety" | "state-machine";
        artifact_prefix: "UX: " | "PromptQuality: " | "PipelineSafety: " | "StateMachine: ";
        agent_name: "ralphx:ralphx-ideation-specialist-ux" | "ralphx:ralphx-ideation-specialist-prompt-quality" | "ralphx:ralphx-ideation-specialist-pipeline-safety" | "ralphx:ralphx-ideation-specialist-state-machine";
    }[];
    optional_delegates: {
        job_id: string;
        artifact_prefix: string;
        label?: string;
        required?: boolean;
    }[];
    optional_artifacts_by_prefix: ArtifactByPrefix;
    optional_delegate_snapshots: unknown[];
}>;
export {};
//# sourceMappingURL=verification-orchestration.d.ts.map