type ArtifactSummary = {
  id?: string;
  name?: string;
  created_at?: string;
  content?: string;
};

export type VerificationFindingGap = {
  severity: string;
  category: string;
  description: string;
  why_it_matters?: string | null;
  source?: string | null;
  lens?: string | null;
};

export type VerificationFindingSummary = {
  artifact_id: string;
  title: string;
  created_at: string;
  author_teammate?: string | null;
  critic: string;
  round: number;
  status: string;
  coverage?: string | null;
  summary: string;
  gaps: VerificationFindingGap[];
};

export type VerificationRoundArtifactMatch = {
  prefix: string;
  found: boolean;
  total_matches: number;
  artifact?: ArtifactSummary;
};

export type VerificationRoundDelegateInput = {
  job_id: string;
  artifact_prefix: string;
  required?: boolean;
  label?: string;
};

export type VerificationRoundDelegateSnapshot = {
  job_id: string;
  status?: string;
  error?: string | null;
  delegated_status?: {
    agent_state?: {
      estimated_status?: string | null;
    };
    latest_run?: {
      status?: string | null;
      error_message?: string | null;
    } | null;
  } | null;
};

type DelegateAssessmentKind =
  | "artifact_published"
  | "pending"
  | "infra_failure";

export type VerificationRoundDelegateAssessment = {
  job_id: string;
  label: string;
  artifact_prefix: string;
  required: boolean;
  artifact_found: boolean;
  assessment: DelegateAssessmentKind;
  status: string;
  reason: string;
};

export type VerificationRoundAssessment = {
  classification: "complete" | "pending" | "infra_failure";
  recommended_next_action:
    | "continue_round_analysis"
    | "perform_single_rescue_or_wait"
    | "complete_verification_with_infra_failure";
  summary: string;
  missing_required_prefixes: string[];
  delegate_assessments: VerificationRoundDelegateAssessment[];
  artifacts_by_prefix: VerificationRoundArtifactMatch[];
};

export type ParsedVerificationGap = {
  severity: "critical" | "high" | "medium" | "low";
  category: string;
  description: string;
  why_it_matters?: string;
  source?: "layer1" | "layer2" | "both";
};

export type ParsedVerificationCriticArtifact = {
  prefix: string;
  label: string;
  usable: boolean;
  artifact_id?: string;
  artifact_name?: string;
  artifact_created_at?: string;
  status?: string;
  critic?: string;
  round?: number;
  coverage?: string;
  summary?: string;
  gaps: ParsedVerificationGap[];
  parse_error?: string;
};

export type VerificationGapCounts = {
  critical: number;
  high: number;
  medium: number;
  low: number;
};

function mergeGapSource(
  current: ParsedVerificationGap["source"],
  incoming: ParsedVerificationGap["source"]
): ParsedVerificationGap["source"] {
  if (!current) {
    return incoming;
  }
  if (!incoming || current === incoming) {
    return current;
  }
  if (current === "both" || incoming === "both") {
    return "both";
  }
  return "both";
}

function gapDedupKey(gap: ParsedVerificationGap): string {
  return [
    gap.severity.trim().toLowerCase(),
    gap.category.trim().toLowerCase(),
    gap.description.trim().toLowerCase(),
    (gap.why_it_matters ?? "").trim().toLowerCase(),
  ].join("::");
}

export function aggregateVerificationGaps(findings: ParsedVerificationCriticArtifact[]): {
  merged_gaps: ParsedVerificationGap[];
  gap_counts: VerificationGapCounts;
} {
  const merged = new Map<string, ParsedVerificationGap>();

  for (const finding of findings) {
    if (!finding.usable) {
      continue;
    }

    for (const gap of finding.gaps) {
      const key = gapDedupKey(gap);
      const existing = merged.get(key);
      if (!existing) {
        merged.set(key, {
          ...gap,
        });
        continue;
      }

      merged.set(key, {
        ...existing,
        why_it_matters: existing.why_it_matters ?? gap.why_it_matters,
        source: mergeGapSource(existing.source, gap.source),
      });
    }
  }

  const merged_gaps = Array.from(merged.values());
  const gap_counts: VerificationGapCounts = {
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
  };

  for (const gap of merged_gaps) {
    gap_counts[gap.severity] += 1;
  }

  return {
    merged_gaps,
    gap_counts,
  };
}

function normalizeSeverity(value: unknown): ParsedVerificationGap["severity"] | null {
  if (value === "critical" || value === "high" || value === "medium" || value === "low") {
    return value;
  }
  return null;
}

function normalizeSource(value: unknown): ParsedVerificationGap["source"] | undefined {
  if (value === "layer1" || value === "layer2" || value === "both") {
    return value;
  }
  return undefined;
}

export function parseVerificationCriticArtifact(params: {
  prefix: string;
  label: string;
  artifact?: ArtifactSummary;
}): ParsedVerificationCriticArtifact {
  const base: ParsedVerificationCriticArtifact = {
    prefix: params.prefix,
    label: params.label,
    usable: false,
    artifact_id: params.artifact?.id,
    artifact_name: params.artifact?.name,
    artifact_created_at: params.artifact?.created_at,
    gaps: [],
  };

  const rawContent = params.artifact?.content;
  if (typeof rawContent !== "string" || rawContent.trim().length === 0) {
    return {
      ...base,
      parse_error: "Artifact content is missing.",
    };
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(rawContent);
  } catch (error) {
    return {
      ...base,
      parse_error:
        error instanceof Error ? error.message : "Artifact content is not valid JSON.",
    };
  }

  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return {
      ...base,
      parse_error: "Artifact JSON must be an object.",
    };
  }

  const record = parsed as Record<string, unknown>;
  const status = typeof record.status === "string" ? record.status : undefined;
  const critic = typeof record.critic === "string" ? record.critic : undefined;
  const round = typeof record.round === "number" ? record.round : undefined;
  const coverage = typeof record.coverage === "string" ? record.coverage : undefined;
  const summary = typeof record.summary === "string" ? record.summary : undefined;
  const rawGaps = Array.isArray(record.gaps) ? record.gaps : [];

  const gaps: ParsedVerificationGap[] = [];
  for (const gap of rawGaps) {
    if (!gap || typeof gap !== "object" || Array.isArray(gap)) {
      continue;
    }
    const gapRecord = gap as Record<string, unknown>;
    const severity = normalizeSeverity(gapRecord.severity);
    const category =
      typeof gapRecord.category === "string" ? gapRecord.category.trim() : "";
    const description =
      typeof gapRecord.description === "string" ? gapRecord.description.trim() : "";
    if (!severity || category.length === 0 || description.length === 0) {
      continue;
    }
    const whyItMatters =
      typeof gapRecord.why_it_matters === "string"
        ? gapRecord.why_it_matters
        : undefined;
    gaps.push({
      severity,
      category,
      description,
      why_it_matters: whyItMatters,
      source: normalizeSource(gapRecord.source),
    });
  }

  const usable =
    (status === "complete" || status === "partial" || status === "error") &&
    typeof critic === "string" &&
    typeof summary === "string";

  return {
    ...base,
    usable,
    status,
    critic,
    round,
    coverage,
    summary,
    gaps,
    parse_error: usable ? undefined : "Artifact JSON is missing required verifier fields.",
  };
}

export function parseTypedVerificationFinding(params: {
  label: string;
  finding?: VerificationFindingSummary;
}): ParsedVerificationCriticArtifact {
  const finding = params.finding;
  const base: ParsedVerificationCriticArtifact = {
    prefix: params.label,
    label: params.label,
    usable: false,
    artifact_id: finding?.artifact_id,
    artifact_name: finding?.title,
    artifact_created_at: finding?.created_at,
    gaps: [],
  };

  if (!finding) {
    return {
      ...base,
      parse_error: "Typed verification finding is missing.",
    };
  }

  const status =
    finding.status === "complete" ||
    finding.status === "partial" ||
    finding.status === "error"
      ? finding.status
      : undefined;

  const gaps: ParsedVerificationGap[] = [];
  for (const gap of finding.gaps ?? []) {
    const severity = normalizeSeverity(gap.severity);
    const category =
      typeof gap.category === "string" ? gap.category.trim() : "";
    const description =
      typeof gap.description === "string" ? gap.description.trim() : "";
    if (!severity || category.length === 0 || description.length === 0) {
      continue;
    }
    gaps.push({
      severity,
      category,
      description,
      why_it_matters:
        typeof gap.why_it_matters === "string" ? gap.why_it_matters : undefined,
      source: normalizeSource(gap.source),
    });
  }

  const usable =
    typeof finding.critic === "string" &&
    typeof finding.summary === "string" &&
    status !== undefined;

  return {
    ...base,
    usable,
    status,
    critic: finding.critic,
    round: finding.round,
    coverage:
      typeof finding.coverage === "string" ? finding.coverage : undefined,
    summary: finding.summary,
    gaps,
    parse_error: usable ? undefined : "Typed verification finding is missing required fields.",
  };
}

function isRunningLike(status?: string | null): boolean {
  return (
    status === "running" ||
    status === "queued" ||
    status === "likely_generating" ||
    status === "likely_waiting"
  );
}

function classifyDelegate(
  delegate: VerificationRoundDelegateInput,
  artifact: VerificationRoundArtifactMatch | undefined,
  snapshot: VerificationRoundDelegateSnapshot | undefined,
  rescueBudgetExhausted: boolean
): VerificationRoundDelegateAssessment {
  const label = delegate.label || delegate.artifact_prefix.trim() || delegate.job_id;
  const required = delegate.required !== false;
  const artifactFound = artifact?.found === true;

  if (artifactFound) {
    return {
      job_id: delegate.job_id,
      label,
      artifact_prefix: delegate.artifact_prefix,
      required,
      artifact_found: true,
      assessment: "artifact_published",
      status: "completed",
      reason: "Required artifact was published for this delegate.",
    };
  }

  const jobStatus = snapshot?.status ?? "unknown";
  const runStatus = snapshot?.delegated_status?.latest_run?.status ?? null;
  const estimatedStatus = snapshot?.delegated_status?.agent_state?.estimated_status ?? null;
  const error =
    snapshot?.error ??
    snapshot?.delegated_status?.latest_run?.error_message ??
    null;

  if (
    isRunningLike(jobStatus) ||
    isRunningLike(runStatus) ||
    isRunningLike(estimatedStatus)
  ) {
    return {
      job_id: delegate.job_id,
      label,
      artifact_prefix: delegate.artifact_prefix,
      required,
      artifact_found: false,
      assessment: "pending",
      status: jobStatus,
      reason: "Delegate still appears to be running and has not published the required artifact yet.",
    };
  }

  const terminalWithoutArtifact =
    jobStatus === "completed" ||
    jobStatus === "failed" ||
    jobStatus === "cancelled" ||
    runStatus === "completed" ||
    runStatus === "failed" ||
    runStatus === "cancelled";

  if (!rescueBudgetExhausted) {
    return {
      job_id: delegate.job_id,
      label,
      artifact_prefix: delegate.artifact_prefix,
      required,
      artifact_found: false,
      assessment: "pending",
      status: jobStatus,
      reason:
        error ??
        (terminalWithoutArtifact
          ? "Delegate finished without publishing the required artifact yet; the bounded rescue/wait budget is still available."
          : "Required artifact is still missing and the bounded rescue/wait budget is still available."),
    };
  }

  return {
    job_id: delegate.job_id,
    label,
    artifact_prefix: delegate.artifact_prefix,
    required,
    artifact_found: false,
    assessment: "infra_failure",
    status: jobStatus,
    reason:
      error ??
      (terminalWithoutArtifact
        ? "Delegate reached a terminal state without publishing the required artifact."
        : "Required artifact is still missing after the allowed wait/rescue budget."),
  };
}

export function assessVerificationRound(params: {
  delegates: VerificationRoundDelegateInput[];
  artifactsByPrefix: VerificationRoundArtifactMatch[];
  delegateSnapshots: VerificationRoundDelegateSnapshot[];
  rescueBudgetExhausted?: boolean;
}): VerificationRoundAssessment {
  const rescueBudgetExhausted = params.rescueBudgetExhausted === true;
  const artifactByPrefix = new Map(
    params.artifactsByPrefix.map((artifact) => [artifact.prefix, artifact] as const)
  );
  const snapshotByJobId = new Map(
    params.delegateSnapshots.map((snapshot) => [snapshot.job_id, snapshot] as const)
  );

  const delegateAssessments = params.delegates.map((delegate) =>
    classifyDelegate(
      delegate,
      artifactByPrefix.get(delegate.artifact_prefix),
      snapshotByJobId.get(delegate.job_id),
      rescueBudgetExhausted
    )
  );

  const requiredAssessments = delegateAssessments.filter((delegate) => delegate.required);
  const missingRequiredPrefixes = requiredAssessments
    .filter((delegate) => !delegate.artifact_found)
    .map((delegate) => delegate.artifact_prefix);

  const classification = missingRequiredPrefixes.length === 0
    ? "complete"
    : requiredAssessments.some((delegate) => delegate.assessment === "infra_failure")
      ? "infra_failure"
      : "pending";

  const recommendedNextAction =
    classification === "complete"
      ? "continue_round_analysis"
      : classification === "pending"
        ? "perform_single_rescue_or_wait"
        : "complete_verification_with_infra_failure";

  const summary =
    classification === "complete"
      ? "All required verification artifacts were published."
      : classification === "pending"
        ? `Required verification artifacts are still pending for: ${missingRequiredPrefixes.join(", ")}.`
        : `Required verification artifacts are missing after the allowed wait/rescue budget: ${missingRequiredPrefixes.join(", ")}.`;

  return {
    classification,
    recommended_next_action: recommendedNextAction,
    summary,
    missing_required_prefixes: missingRequiredPrefixes,
    delegate_assessments: delegateAssessments,
    artifacts_by_prefix: params.artifactsByPrefix,
  };
}
