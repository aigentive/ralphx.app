import { Loader2, Package, Sparkles } from "lucide-react";
import type { CSSProperties } from "react";

import type { DesignStyleguidePreviewResponse } from "@/api/design";
import type { DesignStyleguideItem } from "./designSystems";
import { useDesignStyleguidePreview } from "./useProjectDesignSystems";

type DesignPreviewContent = DesignStyleguidePreviewResponse["content"];

export function PreviewBlock({ designSystemId, item }: { designSystemId: string; item: DesignStyleguideItem }) {
  const persistedPreviewArtifactId = item.isPersisted ? item.previewArtifactId ?? null : null;
  const previewQuery = useDesignStyleguidePreview(
    designSystemId,
    persistedPreviewArtifactId,
  );
  const preview = previewQuery.data?.content;
  const sourceCount = preview?.source_refs.length ?? item.sourceRefs.length;
  const previewKind = preview?.preview_kind ?? previewKindForItem(item);

  if (!item.isPersisted) {
    return (
      <PreviewSample
        item={item}
        preview={undefined}
        previewKind={previewKind}
        sourceCount={sourceCount}
      />
    );
  }

  if (!item.previewArtifactId) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-empty"
      >
        <Package className="w-4 h-4" style={{ color: "var(--text-muted)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Preview pending
        </span>
      </div>
    );
  }

  if (previewQuery.isLoading) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-loading"
      >
        <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--accent-primary)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Loading preview
        </span>
      </div>
    );
  }

  return (
    <PreviewSample
      item={item}
      preview={preview}
      previewKind={previewKind}
      sourceCount={sourceCount}
    />
  );
}

function PreviewSample({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  if (previewKind === "color_swatch") {
    return <ColorPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "typography_sample") {
    return <TypographyPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "spacing_sample") {
    return <SpacingPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "layout_sample" || previewKind === "screen_artifact_preview") {
    return <LayoutPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "asset_sample") {
    return <AssetPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  return <ComponentPreview item={item} preview={preview} previewKind={previewKind} sourceCount={sourceCount} />;
}

type PreviewRecord = Record<string, unknown>;

function previewRecords(preview: DesignPreviewContent | undefined, key: string): PreviewRecord[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is PreviewRecord => Boolean(entry) && typeof entry === "object" && !Array.isArray(entry));
}

function previewStringArray(preview: DesignPreviewContent | undefined, key: string): string[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0);
}

function recordString(record: PreviewRecord, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function previewRecord(preview: DesignPreviewContent | undefined, key: string): PreviewRecord | null {
  const value = (preview as PreviewRecord | undefined)?.[key];
  return Boolean(value) && typeof value === "object" && !Array.isArray(value)
    ? value as PreviewRecord
    : null;
}

function recordObject(record: PreviewRecord, key: string): PreviewRecord | null {
  const value = record[key];
  return Boolean(value) && typeof value === "object" && !Array.isArray(value)
    ? value as PreviewRecord
    : null;
}

function recordArray(record: PreviewRecord, key: string): PreviewRecord[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is PreviewRecord => Boolean(entry) && typeof entry === "object" && !Array.isArray(entry));
}

const CSS_STYLE_KEYS: Record<string, keyof CSSProperties> = {
  background: "background",
  "background-color": "backgroundColor",
  border: "border",
  "border-radius": "borderRadius",
  "box-shadow": "boxShadow",
  color: "color",
  height: "height",
  "min-height": "minHeight",
  padding: "padding",
  "font-size": "fontSize",
  "font-weight": "fontWeight",
  "letter-spacing": "letterSpacing",
  transform: "transform",
};

function cssStyleFromRecord(record: PreviewRecord | null): CSSProperties {
  if (!record) {
    return {};
  }
  const style: CSSProperties = {};
  for (const [sourceKey, targetKey] of Object.entries(CSS_STYLE_KEYS)) {
    const value = recordString(record, sourceKey);
    if (value) {
      (style as Record<string, string | number>)[targetKey] = value;
    }
  }
  return style;
}

function sourceLabelsForPreview(
  item: DesignStyleguideItem,
  preview: DesignPreviewContent | undefined,
  fallbackCount = 4,
): string[] {
  const labels = previewStringArray(preview, "source_labels");
  if (labels.length > 0) {
    return labels.slice(0, fallbackCount);
  }
  const paths = previewStringArray(preview, "source_paths");
  const pathLabels = paths.map(labelFromSourcePath).filter(Boolean);
  if (pathLabels.length > 0) {
    return pathLabels.slice(0, fallbackCount);
  }
  const itemLabels = item.sourceRefs.map((sourceRef) => labelFromSourcePath(sourceRef.path));
  return itemLabels.length > 0 ? itemLabels.slice(0, fallbackCount) : [item.label];
}

function labelFromSourcePath(path: string): string {
  const filename = path.split("/").pop() ?? path;
  const stem = filename.replace(/\.[^.]+$/, "");
  return stem
    .replace(/[-_.]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (match) => match.toUpperCase())
    .trim();
}

function SourceBackedPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const labels = sourceLabelsForPreview(item, preview, 6);
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-source-backed-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((label) => (
          <div
            key={label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[11px] font-medium" style={{ color: "var(--text-primary)" }}>
              {label}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function ColorPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const swatches = previewRecords(preview, "swatches")
    .map((swatch, index) => ({
      label: recordString(swatch, "label") ?? `Source ${index + 1}`,
      value: recordString(swatch, "value"),
    }))
    .filter((swatch): swatch is { label: string; value: string } => Boolean(swatch.value));

  if (swatches.length === 0) {
    return (
      <SourceBackedPreview
        item={item}
        preview={preview}
        previewKind={preview?.preview_kind ?? "color_swatch"}
        sourceCount={sourceCount}
      />
    );
  }

  return (
    <div className="space-y-2" data-testid="design-color-preview">
      <div className="grid grid-cols-2 gap-2">
        {swatches.map((swatch) => (
          <div
            key={swatch.label}
            className="min-h-20 rounded-lg border p-2"
            style={{
              borderColor: "var(--overlay-weak)",
              background: swatch.value,
              color: readableTextColor(swatch.value),
            }}
          >
            <div className="text-[10px] font-semibold uppercase">{swatch.label}</div>
            <div className="mt-5 text-[11px]">{swatch.value}</div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "color_swatch"} sourceCount={sourceCount} />
    </div>
  );
}

function readableTextColor(color: string): string {
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    const normalized = hex.length === 3
      ? hex.split("").map((value) => `${value}${value}`).join("")
      : hex.slice(0, 6);
    const red = Number.parseInt(normalized.slice(0, 2), 16);
    const green = Number.parseInt(normalized.slice(2, 4), 16);
    const blue = Number.parseInt(normalized.slice(4, 6), 16);
    if ([red, green, blue].every(Number.isFinite)) {
      return red * 0.299 + green * 0.587 + blue * 0.114 > 150 ? "#111111" : "#ffffff";
    }
  }
  return "var(--text-primary)";
}

function TypographyPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const samples = previewRecords(preview, "typography_samples");
  const rows = samples.length
    ? samples.map((sample, index) => ({
        label: recordString(sample, "label") ?? ["Display", "Body", "Label", "Code"][index] ?? "Type",
        sample: recordString(sample, "sample") ?? item.label,
        className: [
          "text-[20px] leading-7 font-semibold",
          "text-[13px] leading-5",
          "text-[11px] leading-4 font-semibold uppercase",
          "font-mono text-[12px] leading-5",
        ][index] ?? "text-[13px] leading-5",
      }))
    : [
        { label: "Display", sample: preview?.label ?? item.label, className: "text-[20px] leading-7 font-semibold" },
        { label: "Body", sample: preview?.summary ?? item.summary, className: "text-[13px] leading-5" },
        { label: "Label", sample: item.confidence.toUpperCase(), className: "text-[11px] leading-4 font-semibold uppercase" },
        { label: "Code", sample: item.sourceRefs[0]?.path ?? item.itemId, className: "font-mono text-[12px] leading-5" },
      ];
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-typography-preview"
    >
      <div className="space-y-1">
        <div className="text-[28px] leading-9 font-semibold" style={{ color: "var(--text-primary)" }}>
          {preview?.label ?? item.label}
        </div>
        <div className="text-[13px] leading-5" style={{ color: "var(--text-secondary)" }}>
          {preview?.summary ?? item.summary}
        </div>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {rows.map((row) => (
          <div
            key={row.label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {row.label}
            </div>
            <div className={row.className} style={{ color: "var(--text-primary)" }}>
              {row.sample}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "typography_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function ComponentPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const heroArtifact = previewRecord(preview, "hero_artifact");
  if (heroArtifact) {
    return (
      <HeroArtifactPreview
        item={item}
        preview={preview}
        heroArtifact={heroArtifact}
        sourceCount={sourceCount}
      />
    );
  }

  const componentSamples = previewRecords(preview, "component_samples");
  const buttonSamples = componentSamples.filter(
    (sample) => recordString(sample, "kind") === "button",
  );
  if (buttonSamples.length > 0) {
    return (
      <ButtonSystemPreview
        item={item}
        preview={preview}
        samples={buttonSamples}
        previewKind={previewKind}
        sourceCount={sourceCount}
      />
    );
  }

  const labels = componentSamples
    .map((sample) => recordString(sample, "label"))
    .filter((label): label is string => Boolean(label));
  const previewLabels = labels.length > 0
    ? labels
    : sourceLabelsForPreview(item, preview, 4);
  const stateLabels = ["default", "hover", "focus", "loading"];

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-component-preview"
    >
      <div className="flex flex-wrap items-center gap-2">
        {previewLabels.map((label, index) => (
          <button
            key={`${label}-${index}`}
            type="button"
            className="h-8 rounded-md border px-3 text-[12px] font-medium"
            style={{
              borderColor: index === 0 ? "var(--accent-border)" : "var(--overlay-weak)",
              color: index === 0 ? "var(--accent-primary)" : "var(--text-primary)",
              background: index === 0 ? "var(--accent-muted)" : "var(--bg-base)",
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        className="rounded-md border p-2"
        style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
      >
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {stateLabels.map((stateLabel) => (
            <div
              key={stateLabel}
              className="rounded-md border px-2 py-1.5 text-[11px]"
              style={{ borderColor: "var(--overlay-weak)", color: "var(--text-secondary)", background: "var(--bg-surface)" }}
            >
              {stateLabel}
            </div>
          ))}
        </div>
      </div>
      <div className="flex items-center justify-between gap-3 text-[12px]">
        <div className="min-w-0">
          <div className="font-medium truncate" style={{ color: "var(--text-primary)" }}>
            {preview?.label ?? item.label}
          </div>
          <div className="truncate" style={{ color: "var(--text-secondary)" }}>
            {preview?.summary ?? item.summary}
          </div>
        </div>
        <span
          className="shrink-0 rounded-full border px-2 py-1 text-[11px]"
          style={{ borderColor: "var(--overlay-faint)", color: "var(--text-secondary)" }}
        >
          {item.confidence} confidence
        </span>
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
    </div>
  );
}

function ButtonSystemPreview({
  item,
  preview,
  samples,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  samples: PreviewRecord[];
  previewKind: string;
  sourceCount: number;
}) {
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-button-system-preview"
    >
      <div className="flex flex-wrap items-center gap-2">
        {samples.map((sample, index) => {
          const label = recordString(sample, "label") ?? `Button ${index + 1}`;
          const sampleStyle = cssStyleFromRecord(recordObject(sample, "styles"));
          return (
            <button
              key={`${label}-${index}`}
              type="button"
              className="inline-flex items-center justify-center whitespace-nowrap transition-transform"
              style={{
                minHeight: "var(--space-10)",
                padding: "0 var(--space-4)",
                borderRadius: "var(--radius-full)",
                border: "1px solid var(--overlay-weak)",
                fontSize: "var(--font-size-sm)",
                fontWeight: 600,
                color: "var(--text-primary)",
                background: "var(--bg-base)",
                ...sampleStyle,
              }}
            >
              {label}
            </button>
          );
        })}
      </div>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {["default", "hover", "focus", "loading"].map((stateLabel) => {
          const sample = samples[0] ?? null;
          const sampleStyle = cssStyleFromRecord(sample ? recordObject(sample, "styles") : null);
          return (
            <button
              key={stateLabel}
              type="button"
              className="inline-flex min-w-0 items-center justify-center truncate border px-2 text-[11px] font-semibold"
              style={{
                minHeight: "var(--space-9)",
                borderColor: "var(--overlay-weak)",
                borderRadius: sampleStyle.borderRadius ?? "var(--radius-full)",
                color: sampleStyle.color ?? "var(--text-primary)",
                background: sampleStyle.background ?? sampleStyle.backgroundColor ?? "var(--bg-base)",
                boxShadow: stateLabel === "focus"
                  ? `0 0 0 2px var(--accent-muted), ${sampleStyle.boxShadow ?? "none"}`
                  : sampleStyle.boxShadow,
                transform: stateLabel === "hover"
                  ? sampleStyle.transform ?? "translateY(-1px)"
                  : undefined,
                opacity: stateLabel === "loading" ? 0.78 : undefined,
              }}
            >
              {stateLabel === "loading" ? "Loading ..." : stateLabel}
            </button>
          );
        })}
      </div>
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
    </div>
  );
}

function HeroArtifactPreview({
  item,
  preview,
  heroArtifact,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  heroArtifact: PreviewRecord;
  sourceCount: number;
}) {
  const panelStyle = cssStyleFromRecord(recordObject(heroArtifact, "panel_styles"));
  const triggerStyle = cssStyleFromRecord(recordObject(heroArtifact, "trigger_styles"));
  const workflowStyle = cssStyleFromRecord(recordObject(heroArtifact, "workflow_styles"));
  const steps = recordArray(heroArtifact, "steps");
  const stepRows = steps.length > 0
    ? steps
    : [
        { label: "Lead captured", state: "done" },
        { label: "CRM checked", state: "done" },
        { label: "Proposal drafted", state: "pending" },
      ];

  return (
    <div
      className="rounded-lg border p-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-hero-artifact-preview"
    >
      <div
        className="space-y-3 rounded-xl border p-3 shadow-lg"
        style={{
          borderColor: "var(--overlay-faint)",
          background: "var(--bg-base)",
          ...panelStyle,
        }}
      >
        <div
          className="rounded-lg border p-3"
          style={{
            borderColor: "var(--status-success-border)",
            background: "var(--status-success-muted)",
            ...triggerStyle,
          }}
        >
          <div className="flex items-center justify-between gap-3 text-[11px] font-semibold">
            <span style={{ color: "var(--status-success)" }}>
              {recordString(heroArtifact, "channel_label") ?? "WhatsApp"}
            </span>
            <span style={{ color: "var(--text-muted)" }}>09:41</span>
          </div>
          <div className="mt-2 text-[12px] leading-5" style={{ color: "var(--text-primary)" }}>
            New qualified lead asked for a deployment quote.
          </div>
        </div>

        <div
          className="rounded-lg border p-3"
          style={{
            borderColor: "var(--overlay-weak)",
            background: "var(--bg-surface)",
            ...workflowStyle,
          }}
        >
          <div className="flex items-center justify-between gap-3">
            <div className="inline-flex items-center gap-2 text-[12px] font-semibold" style={{ color: "var(--accent-primary)" }}>
              <Sparkles className="h-4 w-4" />
              {recordString(heroArtifact, "agent_label") ?? "Agent AI"}
            </div>
            <span
              className="rounded-full border px-2 py-1 text-[11px]"
              style={{
                borderColor: "var(--accent-border)",
                background: "var(--accent-muted)",
                color: "var(--accent-primary)",
              }}
            >
              {recordString(heroArtifact, "status_label") ?? "Workflow running"}
            </span>
          </div>
          <div className="mt-3 space-y-2">
            {stepRows.map((step, index) => {
              const label = recordString(step as PreviewRecord, "label") ?? `Step ${index + 1}`;
              const state = recordString(step as PreviewRecord, "state") ?? "pending";
              return (
                <div key={`${label}-${index}`} className="flex items-center gap-2 text-[12px]">
                  <span
                    className="inline-flex h-5 w-5 items-center justify-center rounded-full border text-[10px]"
                    style={{
                      borderColor: state === "done" ? "var(--status-success-border)" : "var(--overlay-weak)",
                      background: state === "done" ? "var(--status-success-muted)" : "var(--bg-base)",
                      color: state === "done" ? "var(--status-success)" : "var(--text-muted)",
                    }}
                  >
                    {state === "done" ? "ok" : "-"}
                  </span>
                  <span style={{ color: "var(--text-primary)" }}>{label}</span>
                </div>
              );
            })}
          </div>
          <div className="mt-3 rounded-md border p-2 text-[12px] leading-5" style={{ borderColor: "var(--overlay-faint)", color: "var(--text-secondary)" }}>
            {preview?.summary ?? item.summary}
          </div>
        </div>
      </div>
      <div className="mt-3">
        <PreviewMeta previewKind={preview?.preview_kind ?? "component_sample"} sourceCount={sourceCount} />
      </div>
    </div>
  );
}

function SpacingPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-spacing-preview"
    >
      <div className="grid grid-cols-4 gap-2">
        {[4, 8, 12, 16].map((size) => (
          <div
            key={size}
            className="rounded-md border p-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div
              className="rounded-sm"
              style={{
                height: `${Math.max(size, 8)}px`,
                background: "var(--accent-muted)",
              }}
            />
            <div className="mt-2 text-[11px]" style={{ color: "var(--text-secondary)" }}>
              {size}px
            </div>
          </div>
        ))}
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        <div className="h-14 rounded-lg border" style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-base)" }} />
        <div className="h-14 rounded-lg border shadow-lg" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }} />
      </div>
      <div className="flex flex-wrap gap-2">
        {labels.map((label) => (
          <span
            key={label}
            className="rounded-md border px-2 py-1 text-[11px]"
            style={{
              borderColor: "var(--overlay-faint)",
              background: "var(--bg-base)",
              color: "var(--text-secondary)",
            }}
          >
            {label}
          </span>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "spacing_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function LayoutPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-layout-preview"
    >
      <div
        className="grid h-36 overflow-hidden rounded-md border"
        style={{
          borderColor: "var(--overlay-faint)",
          gridTemplateColumns: labels.length >= 3 ? "0.8fr 1.4fr 1fr" : "1fr 1fr",
        }}
        data-testid="design-workspace-surface-preview"
      >
        {labels.map((label, index) => (
          <div
            key={`${label}-${index}`}
            className={index < labels.length - 1 ? "border-r p-2 space-y-2" : "p-2 space-y-2"}
            style={{ borderColor: "var(--overlay-faint)", background: index % 2 === 0 ? "var(--bg-base)" : "transparent" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {label}
            </div>
            <div className="h-3 w-16 rounded-sm" style={{ background: "var(--overlay-weak)" }} />
            <div className="h-6 rounded-md" style={{ background: index === 0 ? "var(--accent-muted)" : "var(--overlay-faint)" }} />
            <div className="h-6 rounded-md border" style={{ borderColor: "var(--overlay-faint)" }} />
          </div>
        ))}
      </div>
      <div className="text-[12px] font-medium" style={{ color: "var(--text-primary)" }}>
        {preview?.label ?? item.label}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "layout_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function AssetPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const assets = previewRecords(preview, "asset_samples")
    .map((asset) => ({
      label: recordString(asset, "label"),
      path: recordString(asset, "path"),
      uri: recordString(asset, "uri"),
      mediaType: recordString(asset, "media_type"),
      surface: recordString(asset, "surface") ?? "light",
    }))
    .filter((asset): asset is {
      label: string;
      path: string | null;
      uri: string | null;
      mediaType: string | null;
      surface: string;
    } => Boolean(asset.label));
  const labels = assets.length > 0
    ? assets
    : sourceLabelsForPreview(item, preview, 4).map((label) => ({
        label,
        path: null,
        uri: null,
        mediaType: null,
        surface: "light",
      }));

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-asset-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((asset) => (
          <div key={asset.label} className="rounded-md border p-2" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}>
            <div
              className="flex min-h-20 items-center justify-center rounded-lg border p-3"
              style={{
                borderColor: "var(--overlay-faint)",
                background: asset.surface === "dark" ? "var(--text-primary)" : "var(--bg-surface)",
              }}
            >
              {asset.uri ? (
                <img
                  alt={asset.label}
                  className="max-h-14 max-w-full object-contain"
                  src={asset.uri}
                  data-testid="design-asset-image"
                />
              ) : (
                <Sparkles className="h-4 w-4" style={{ color: "var(--accent-primary)" }} />
              )}
            </div>
            <div className="mt-2 min-w-0">
              <div className="truncate text-[12px] font-semibold" style={{ color: "var(--text-primary)" }}>
                {asset.label}
              </div>
              {asset.path ? (
                <div className="truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                  {asset.path}
                </div>
              ) : null}
              {asset.mediaType ? (
                <div className="truncate text-[10px]" style={{ color: "var(--text-muted)" }}>
                  {asset.mediaType}
                </div>
              ) : null}
            </div>
          </div>
        ))}
      </div>
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "asset_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function PreviewMeta({ previewKind, sourceCount }: { previewKind: string; sourceCount: number }) {
  return (
    <div className="text-[11px]" style={{ color: "var(--text-muted)" }} data-testid="design-preview-kind">
      {formatPreviewKind(previewKind)} / {sourceCount} {sourceCount === 1 ? "source" : "sources"}
    </div>
  );
}

function previewKindForItem(item: DesignStyleguideItem): string {
  switch (item.group) {
    case "colors":
      return "color_swatch";
    case "type":
      return "typography_sample";
    case "spacing":
      return "spacing_sample";
    case "ui_kit":
      return "layout_sample";
    case "brand":
      return "asset_sample";
    case "components":
    default:
      return "component_sample";
  }
}

function formatPreviewKind(previewKind: string): string {
  return previewKind.replace(/_/g, " ");
}
