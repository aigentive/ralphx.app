export interface ProviderMetadata {
  providerHarness?: string | null | undefined;
  providerSessionId?: string | null | undefined;
  upstreamProvider?: string | null | undefined;
  providerProfile?: string | null | undefined;
  logicalModel?: string | null | undefined;
  effectiveModelId?: string | null | undefined;
  logicalEffort?: string | null | undefined;
  effectiveEffort?: string | null | undefined;
  inputTokens?: number | null | undefined;
  outputTokens?: number | null | undefined;
  cacheCreationTokens?: number | null | undefined;
  cacheReadTokens?: number | null | undefined;
  estimatedUsd?: number | null | undefined;
}

export interface ProviderHarnessBadgeStyle {
  color: string;
  backgroundColor: string;
  border: string;
}

const DEFAULT_BADGE_STYLE: ProviderHarnessBadgeStyle = {
  color: "hsl(150 55% 63%)",
  backgroundColor: "hsla(150 55% 45% / 0.12)",
  border: "1px solid hsla(150 55% 45% / 0.2)",
};

export function formatProviderHarnessLabel(
  harness: string | null | undefined,
): string | null {
  if (!harness) {
    return null;
  }

  if (harness === "codex") {
    return "Codex";
  }

  if (harness === "claude") {
    return "Claude";
  }

  return harness
    .split(/[-_]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function getProviderHarnessBadgeStyle(
  harness: string | null | undefined,
): ProviderHarnessBadgeStyle {
  if (harness === "codex") {
    return {
      color: "hsl(14 100% 68%)",
      backgroundColor: "hsla(14 100% 60% / 0.12)",
      border: "1px solid hsla(14 100% 60% / 0.18)",
    };
  }

  if (harness === "claude") {
    return {
      color: "hsl(220 10% 68%)",
      backgroundColor: "hsla(220 10% 100% / 0.06)",
      border: "1px solid hsla(220 10% 100% / 0.08)",
    };
  }

  return DEFAULT_BADGE_STYLE;
}

export function formatProviderSessionSnippet(
  providerSessionId: string | null | undefined,
  length = 8,
): string | null {
  if (!providerSessionId) {
    return null;
  }

  return providerSessionId.length > length
    ? `${providerSessionId.slice(0, length)}...`
    : providerSessionId;
}

export function describeProviderLineage(
  metadata: ProviderMetadata,
  variant: "panel" | "selector" = "panel",
): string {
  const harnessLabel = formatProviderHarnessLabel(metadata.providerHarness);

  if (metadata.providerSessionId && harnessLabel) {
    return variant === "panel"
      ? `Continuing stored ${harnessLabel} session`
      : `Stored ${harnessLabel} session`;
  }

  if (metadata.providerSessionId) {
    return variant === "panel"
      ? "Continuing stored provider session"
      : "Stored provider session";
  }

  if (harnessLabel) {
    return variant === "panel"
      ? `New attempt will use ${harnessLabel}`
      : `${harnessLabel} selected for next run`;
  }

  return "New attempt will use current settings";
}

export function formatProviderTooltip(metadata: ProviderMetadata): string | null {
  const lineage = describeProviderLineage(metadata);
  const snippet = formatProviderSessionSnippet(metadata.providerSessionId, 12);

  if (!lineage && !snippet) {
    return null;
  }

  if (snippet) {
    return `${lineage} (${snippet})`;
  }

  return lineage;
}

export function formatProviderEvidenceTooltip(metadata: ProviderMetadata): string | null {
  const details: string[] = [];
  const harnessLabel = formatProviderHarnessLabel(metadata.providerHarness);

  if (harnessLabel) {
    details.push(`Harness: ${harnessLabel}`);
  }

  if (metadata.upstreamProvider) {
    details.push(`Upstream: ${metadata.upstreamProvider}`);
  }

  if (metadata.providerProfile) {
    details.push(`Profile: ${metadata.providerProfile}`);
  }

  if (metadata.providerSessionId) {
    details.push(`Session ref: ${formatProviderSessionSnippet(metadata.providerSessionId, 12)}`);
  }

  return details.length > 0 ? details.join(" • ") : null;
}

export function formatProviderModelEffortLabel(metadata: ProviderMetadata): string | null {
  const model = metadata.effectiveModelId ?? metadata.logicalModel;
  const effort = metadata.effectiveEffort ?? metadata.logicalEffort;

  if (model && effort) {
    return `${model} · ${effort}`;
  }

  return model ?? effort ?? null;
}

export function formatProviderUsageTooltip(metadata: ProviderMetadata): string | null {
  const details: string[] = [];

  if (metadata.inputTokens != null) {
    details.push(`Input: ${metadata.inputTokens.toLocaleString("en-US")}`);
  }
  if (metadata.outputTokens != null) {
    details.push(`Output: ${metadata.outputTokens.toLocaleString("en-US")}`);
  }

  const cacheTotal =
    (metadata.cacheCreationTokens ?? 0) + (metadata.cacheReadTokens ?? 0);
  if (cacheTotal > 0) {
    details.push(`Cache: ${cacheTotal.toLocaleString("en-US")}`);
  }

  if (metadata.estimatedUsd != null) {
    details.push(`Est. cost: $${metadata.estimatedUsd.toFixed(2)}`);
  }

  return details.length > 0 ? details.join(" • ") : null;
}

export function formatMessageAttributionTooltip(
  metadata: ProviderMetadata,
): string | null {
  const evidence = formatProviderEvidenceTooltip(metadata);
  const modelEffort = formatProviderModelEffortLabel(metadata);
  const usage = formatProviderUsageTooltip(metadata);

  return [evidence, modelEffort, usage].filter(Boolean).join(" • ") || null;
}
