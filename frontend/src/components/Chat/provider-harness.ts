export interface ProviderMetadata {
  providerHarness?: string | null | undefined;
  providerSessionId?: string | null | undefined;
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
